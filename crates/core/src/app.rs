//! app(M13)— 每帧编排循环,串起 conn→protocol→store→smoother→content→layout→render。
//!
//! 严格分相(AR1):事件改状态(`apply`),渲染只读状态(`build_frame`)。
//! 时间确定性(R8):内部 `now_ms` 由注入的 `dt_ms` 逐帧累加,不碰墙钟。

use crate::content::plain;
use crate::frame::{FrameData, FrameGlyph};
use crate::protocol::{decode, parse_snapshot, Event};
use crate::seam::{Connection, LayoutEngine, LayoutResult, PlacedGlyph, RenderSink};
use crate::smoother::Smoother;
use crate::store::Store;
use crate::support::graphemes;

/// catch-up 字形的 spawn_time:置于"远古",着色器淡入早已完成(alpha=1),实现零动画(AR6)。
const CATCHUP_SPAWN: f32 = -1.0e9;

/// 块间纵向间距(px)。
const BLOCK_GAP: f32 = 8.0;

/// 锚底阈值:滚到离底 ≤ 此值即重新跟随新内容(0002 §6)。
const ANCHOR_THRESHOLD: f32 = 48.0;

/// 已排版块的缓存(Phase G 块冻结):内容/宽度不变则不重排,根治每帧全量重排。
struct BlockCache {
    /// 排版时的 grapheme 数(变了即脏)。
    revealed_len: usize,
    /// 排版时的宽度(变了即脏)。
    width: f32,
    /// 块内相对位置(与 `revealed` 顺序 1:1;cluster/spawn 仍取自 revealed)。
    placed: Vec<PlacedGlyph>,
    /// 块高度。
    height: f32,
}

/// 每个可见 part 的上屏进度 + 排版缓存。
struct PartView {
    part_id: String,
    /// 已 push 进 smoother 的 grapheme 数(对账后从尾部续推)。
    pushed: usize,
    /// 已上屏的 (grapheme, spawn_time_ms),按上屏顺序。
    revealed: Vec<(String, f32)>,
    /// 排版缓存(冻结);None 或脏时重排。
    cache: Option<BlockCache>,
}

/// 每帧编排引擎。`C` 事件源、`L` 排版、`R` 渲染汇均经 seam 注入(CR2)。
pub struct Engine<C: Connection, L: LayoutEngine, R: RenderSink> {
    conn: C,
    layout: L,
    sink: R,
    store: Store,
    smoother: Smoother,
    views: Vec<PartView>,
    now_ms: f64,
    max_width: f32,
    /// 只渲染该 session 的 part(`?session=`);None = 全渲染(Plan1 行为)。
    target_session: Option<String>,
    /// 从顶部下滚的像素(0 = 顶部)。
    scroll_offset: f32,
    /// 视口高度(px),视口裁剪与锚底用。
    viewport_height: f32,
    /// 锚底:在底部时新内容跟随滚动(0002 §6)。
    stick_to_bottom: bool,
}

impl<C: Connection, L: LayoutEngine, R: RenderSink> Engine<C, L, R> {
    /// `base_cps` 吐字基线(~200),`max_width` 排版宽度(px)。
    pub fn new(conn: C, layout: L, sink: R, base_cps: f64, max_width: f32) -> Self {
        Self {
            conn,
            layout,
            sink,
            store: Store::new(),
            smoother: Smoother::new(base_cps),
            views: Vec::new(),
            now_ms: 0.0,
            max_width,
            target_session: None,
            scroll_offset: 0.0,
            viewport_height: 600.0,
            stick_to_bottom: true,
        }
    }

    /// 设视口高度(画布尺寸变化时);视口裁剪与锚底据此。
    pub fn set_viewport_height(&mut self, height: f32) {
        self.viewport_height = height.max(1.0);
    }

    /// 滚动 `dy` 像素(正 = 向下/看更新内容)。向上滚脱离锚底,滚回底部自动重新跟随。
    pub fn scroll_by(&mut self, dy: f32) {
        self.scroll_offset += dy;
        if dy < 0.0 {
            self.stick_to_bottom = false;
        }
    }

    /// 设过滤目标 session(`?session=`);None 全渲染。
    pub fn set_target_session(&mut self, session: Option<String>) {
        self.target_session = session;
    }

    /// 用快照历史预热(Phase F catch-up):灌入 store + 直接整段上屏(零淡入,AR6)。
    /// `raw` 为 `GET /session/{id}/message` 的响应原文。
    pub fn prime_from_snapshot(&mut self, raw: &str) {
        let messages = match parse_snapshot(raw) {
            Ok(m) => m,
            Err(e) => {
                tracing::warn!(target: "M2", error = %e, "快照解析失败,跳过 catch-up");
                return;
            }
        };
        self.store.apply_snapshot(&messages);
        for msg in &messages {
            for tp in &msg.text_parts {
                let revealed: Vec<(String, f32)> = graphemes(&tp.text)
                    .into_iter()
                    .map(|g| (g.to_owned(), CATCHUP_SPAWN))
                    .collect();
                let view = self.view_mut(&tp.part_id);
                view.pushed = revealed.len();
                view.revealed = revealed;
            }
        }
        tracing::info!(target: "M3", n = messages.len(), "快照 catch-up 灌入");
    }

    /// 只读访问 store(供对账/断言,R4)。
    pub fn store(&self) -> &Store {
        &self.store
    }

    /// 只读访问渲染汇(供测试断言末帧内容,R4)。
    pub fn sink(&self) -> &R {
        &self.sink
    }

    /// 可变访问渲染汇(供宿主在 resize 时直驱后端重配 surface)。
    pub fn sink_mut(&mut self) -> &mut R {
        &mut self.sink
    }

    /// 更新排版宽度(画布尺寸变化时)。
    pub fn set_max_width(&mut self, max_width: f32) {
        self.max_width = max_width;
    }

    /// 推进一帧。
    pub fn frame(&mut self, dt_ms: f64) {
        self.now_ms += dt_ms;
        self.ingest_events();
        self.enqueue_new_text();
        self.reveal(dt_ms);
        let frame = self.build_frame();
        self.sink.submit(&frame);
    }

    /// 1) 收事件 → 解码 → 落 store(含 updated 对账,AR4)。
    fn ingest_events(&mut self) {
        for raw in self.conn.poll() {
            match decode(raw.raw()) {
                Ok(Event::PartDelta {
                    part_id,
                    message_id,
                    field,
                    delta,
                    ..
                }) => self
                    .store
                    .apply_delta(&part_id, &message_id, &field, &delta),
                Ok(Event::PartUpdated { part, .. }) => self.store.apply_part_updated(&part),
                // 状态/心跳/握手/未知:Plan1 不改文档状态(AR12)。
                Ok(_) => {}
                Err(e) => {
                    tracing::warn!(target: "M2", error = %e, "丢弃无法解码的事件");
                }
            }
        }
    }

    /// 2) 把 store 里新增的文本尾部切 grapheme 入 smoother。
    fn enqueue_new_text(&mut self) {
        let part_ids: Vec<String> = self
            .store
            .parts_in_order()
            .map(|(id, _)| id.to_owned())
            .collect();
        for part_id in part_ids {
            // 克隆成 owned grapheme,先释放 store 借用,再去改 view/smoother。
            let gs: Vec<String> = match self.store.part_text(&part_id) {
                Some(text) => graphemes(text).into_iter().map(str::to_owned).collect(),
                None => continue,
            };
            let pushed = self.view_mut(&part_id).pushed;
            if gs.len() > pushed {
                let new: Vec<&str> = gs[pushed..].iter().map(String::as_str).collect();
                self.smoother.push(&part_id, &new);
                self.view_mut(&part_id).pushed = gs.len();
            }
        }
    }

    /// 3) 整流吐字 → 记入各 part 的已上屏序列。
    fn reveal(&mut self, dt_ms: f64) {
        for r in self.smoother.update(dt_ms, self.now_ms) {
            self.view_mut(&r.part_id)
                .revealed
                .push((r.cluster, r.spawn_time_ms));
        }
    }

    /// session 过滤(Phase F):目标已设且该 part 归属已知且不匹配 → 过滤掉。
    /// 归属未知时乐观保留(本地单会话常态),待 updated/snapshot 解析后收敛。
    fn is_filtered(&self, view: &PartView) -> bool {
        if let Some(target) = &self.target_session {
            if let Some(sid) = self.store.part_session(&view.part_id) {
                return sid != target;
            }
        }
        false
    }

    /// 排版各块(Phase G 块冻结):内容长度/宽度不变的块跳过 layout 直接用缓存,只有正在
    /// 生长的尾部块(或宽度变化)才重排——根治每帧全量重排。
    fn ensure_layouts(&mut self) {
        for i in 0..self.views.len() {
            let len = self.views[i].revealed.len();
            let dirty = match &self.views[i].cache {
                Some(c) => c.revealed_len != len || (c.width - self.max_width).abs() > f32::EPSILON,
                None => true,
            };
            if !dirty {
                continue;
            }
            let text: String = self.views[i]
                .revealed
                .iter()
                .map(|(c, _)| c.as_str())
                .collect();
            let result = if text.is_empty() {
                LayoutResult::default()
            } else {
                let spans = plain(&text);
                self.layout.layout(&spans, self.max_width)
            };
            self.views[i].cache = Some(BlockCache {
                revealed_len: len,
                width: self.max_width,
                placed: result.glyphs,
                height: result.block_height,
            });
        }
    }

    /// 5) 组 FrameData:纵向堆叠各块,锚底 + 视口裁剪(屏外块不出 glyph,Phase G)。
    fn build_frame(&mut self) -> FrameData {
        self.ensure_layouts();

        // 可绘制块(过滤掉非目标 session / 空块),累计内容高度。
        let mut blocks: Vec<usize> = Vec::new();
        let mut content_height = 0.0f32;
        for (i, view) in self.views.iter().enumerate() {
            if self.is_filtered(view) {
                continue;
            }
            match &view.cache {
                Some(c) if !c.placed.is_empty() => {
                    blocks.push(i);
                    content_height += c.height + BLOCK_GAP;
                }
                _ => {}
            }
        }

        // 锚底 + 夹取滚动位置。
        let max_scroll = (content_height - self.viewport_height).max(0.0);
        if self.stick_to_bottom {
            self.scroll_offset = max_scroll;
        }
        self.scroll_offset = self.scroll_offset.clamp(0.0, max_scroll);
        if self.scroll_offset >= max_scroll - ANCHOR_THRESHOLD {
            self.stick_to_bottom = true;
        }

        // 裁剪 + 出 glyph。overscan 一屏,减少滚动边缘空白。
        let overscan = self.viewport_height;
        let mut glyphs = Vec::new();
        let mut top = 0.0f32;
        for &i in &blocks {
            let view = &self.views[i];
            let cache = view.cache.as_ref().expect("blocks 已筛选非空缓存"); // reason: 上面已保证
            let bottom = top + cache.height;
            let visible = bottom >= self.scroll_offset - overscan
                && top <= self.scroll_offset + self.viewport_height + overscan;
            if visible {
                for (placed, (cluster, spawn)) in cache.placed.iter().zip(view.revealed.iter()) {
                    if cluster == "\n" {
                        continue; // 换行不出字形,但仍消费 spawn 保持对齐
                    }
                    glyphs.push(FrameGlyph {
                        cluster: cluster.clone(),
                        pos: [placed.pos[0], placed.pos[1] + top - self.scroll_offset],
                        size: placed.size,
                        spawn_time: *spawn,
                    });
                }
            }
            top = bottom + BLOCK_GAP;
        }
        FrameData {
            glyphs,
            time_ms: self.now_ms as f32,
        }
    }

    /// 取或建某 part 的视图(保持 store 顺序)。
    fn view_mut(&mut self, part_id: &str) -> &mut PartView {
        if let Some(idx) = self.views.iter().position(|v| v.part_id == part_id) {
            return &mut self.views[idx];
        }
        self.views.push(PartView {
            part_id: part_id.to_owned(),
            pushed: 0,
            revealed: Vec::new(),
            cache: None,
        });
        self.views.last_mut().expect("just pushed") // reason: 上面刚 push
    }
}

#[cfg(test)]
mod tests {
    use crate::record::Player;
    use crate::support::{CollectSink, MonospaceLayout};
    use crate::Engine;

    fn delta(part: &str, delta: &str) -> String {
        format!(
            r#"{{"type":"message.part.delta","properties":{{"sessionID":"s","messageID":"m","partID":"{part}","field":"text","delta":{delta:?}}}}}"#
        )
    }

    #[test]
    fn streams_text_to_visible_glyphs() {
        let records = vec![(0.0, delta("p1", "Hi 你好"))];
        let player = Player::from_pairs(records, 16.0);
        let mut eng = Engine::new(
            player,
            MonospaceLayout::default(),
            CollectSink::default(),
            500.0, // 快一点,几帧就吐完
            800.0,
        );
        for _ in 0..30 {
            eng.frame(16.0);
        }
        assert_eq!(eng.store().part_text("p1"), Some("Hi 你好"));
        // 渲染汇可见文本应等于完整串(顺序无损)。
        assert_eq!(eng.sink().visible_text(), "Hi 你好");
    }

    #[test]
    fn fade_in_spawn_times_increase() {
        let player = Player::from_pairs(vec![(0.0, delta("p", "abcdef"))], 16.0);
        let mut eng = Engine::new(
            player,
            MonospaceLayout::default(),
            CollectSink::default(),
            60.0, // 慢:每帧约吐 1 字,spawn_time 递增
            800.0,
        );
        for _ in 0..60 {
            eng.frame(16.0);
        }
        let frame = eng.sink().last().expect("frame");
        let spawns: Vec<f32> = frame.glyphs.iter().map(|g| g.spawn_time).collect();
        assert_eq!(spawns.len(), 6);
        // 非递减(逐字上屏,后到的 spawn_time >= 先到的)。
        assert!(spawns.windows(2).all(|w| w[1] >= w[0]), "{spawns:?}");
        assert!(spawns[5] > spawns[0], "末字应晚于首字: {spawns:?}");
    }

    #[test]
    fn snapshot_primes_instantly_without_fade() {
        // Phase F:快照历史一帧即整段上屏,spawn_time 在远古(零淡入,AR6)。
        let mut eng = Engine::new(
            Player::from_pairs(vec![], 16.0),
            MonospaceLayout::default(),
            CollectSink::default(),
            200.0,
            800.0,
        );
        let snap = r#"[{"info":{"id":"m1","sessionID":"sX","role":"assistant"},
            "parts":[{"type":"text","id":"p1","messageID":"m1","text":"历史回复"}]}]"#;
        eng.prime_from_snapshot(snap);
        eng.frame(16.0); // 一帧
        assert_eq!(eng.sink().visible_text(), "历史回复");
        let frame = eng.sink().last().expect("frame");
        assert!(
            frame.glyphs.iter().all(|g| g.spawn_time < 0.0),
            "catch-up 字形 spawn_time 应在远古"
        );
    }

    #[test]
    fn session_filter_excludes_other_session() {
        // Phase F:目标 sX → 只渲染 sX 的 part,sY 被排除。
        let mut eng = Engine::new(
            Player::from_pairs(vec![], 16.0),
            MonospaceLayout::default(),
            CollectSink::default(),
            200.0,
            800.0,
        );
        eng.set_target_session(Some("sX".into()));
        let snap = r#"[
            {"info":{"id":"m1","sessionID":"sX","role":"assistant"},
             "parts":[{"type":"text","id":"p1","messageID":"m1","text":"AAA"}]},
            {"info":{"id":"m2","sessionID":"sY","role":"assistant"},
             "parts":[{"type":"text","id":"p2","messageID":"m2","text":"BBB"}]}
        ]"#;
        eng.prime_from_snapshot(snap);
        eng.frame(16.0);
        assert_eq!(eng.sink().visible_text(), "AAA");
    }

    // 计数排版器:数 layout 被调用多少次,验证块冻结(Phase G)。
    struct CountingLayout {
        inner: MonospaceLayout,
        calls: std::rc::Rc<std::cell::Cell<usize>>,
    }
    impl crate::LayoutEngine for CountingLayout {
        fn layout(&mut self, spans: &[crate::StyledSpan], w: f32) -> crate::LayoutResult {
            self.calls.set(self.calls.get() + 1);
            self.inner.layout(spans, w)
        }
    }

    #[test]
    fn block_freeze_skips_settled_relayout() {
        // 流式期间每帧重排尾部;吐完(settled)后再多帧不应再调 layout。
        let calls = std::rc::Rc::new(std::cell::Cell::new(0usize));
        let layout = CountingLayout {
            inner: MonospaceLayout::default(),
            calls: calls.clone(),
        };
        let mut eng = Engine::new(
            Player::from_pairs(vec![(0.0, delta("p", "abcdefghij"))], 16.0),
            layout,
            CollectSink::default(),
            500.0,
            800.0,
        );
        for _ in 0..40 {
            eng.frame(16.0); // 吐完
        }
        assert_eq!(eng.sink().visible_text(), "abcdefghij");
        let settled = calls.get();
        for _ in 0..10 {
            eng.frame(16.0); // 无新增 → 不应再排版
        }
        assert_eq!(calls.get(), settled, "已冻结块被重排了");
    }

    #[test]
    fn viewport_culls_offscreen_blocks() {
        // 多块 + 小视口 + 锚底 → 顶部块裁掉,底部块可见(Phase G)。
        let mut eng = Engine::new(
            Player::from_pairs(vec![], 16.0),
            MonospaceLayout::default(),
            CollectSink::default(),
            200.0,
            800.0,
        );
        eng.set_viewport_height(25.0); // 约一行
        let snap = r#"[
            {"info":{"id":"m1","sessionID":"s","role":"a"},"parts":[{"type":"text","id":"p1","messageID":"m1","text":"AAAA"}]},
            {"info":{"id":"m2","sessionID":"s","role":"a"},"parts":[{"type":"text","id":"p2","messageID":"m2","text":"BBBB"}]},
            {"info":{"id":"m3","sessionID":"s","role":"a"},"parts":[{"type":"text","id":"p3","messageID":"m3","text":"CCCC"}]},
            {"info":{"id":"m4","sessionID":"s","role":"a"},"parts":[{"type":"text","id":"p4","messageID":"m4","text":"DDDD"}]},
            {"info":{"id":"m5","sessionID":"s","role":"a"},"parts":[{"type":"text","id":"p5","messageID":"m5","text":"EEEE"}]}
        ]"#;
        eng.prime_from_snapshot(snap);
        eng.frame(16.0);
        let visible = eng.sink().visible_text();
        assert!(visible.contains("EEEE"), "底部块应可见: {visible}");
        assert!(!visible.contains("AAAA"), "顶部块应被裁剪: {visible}");
    }
}
