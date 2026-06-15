//! app(M13)— 每帧编排循环,串起 conn→protocol→store→smoother→content→layout→render。
//!
//! 严格分相(AR1):事件改状态(`apply`),渲染只读状态(`build_frame`)。
//! 时间确定性(R8):内部 `now_ms` 由注入的 `dt_ms` 逐帧累加,不碰墙钟。

use crate::camera::{Camera2D, Rect};
use crate::content::{parse_markdown_tables, StyleRole};
use crate::frame::{FrameData, FrameGlyph, FrameRect};
use crate::fsm::{TurnStatus, TurnTracker};
use crate::protocol::{decode, parse_snapshot, Event};
use crate::seam::{Connection, LayoutEngine, PlacedGlyph, RenderSink};
use crate::smoother::Smoother;
use crate::spatial::SpatialGrid;
use crate::store::Store;
use crate::support::graphemes;
use crate::theme;

/// catch-up 字形的 spawn_time:置于"远古",着色器淡入早已完成(alpha=1),实现零动画(AR6)。
const CATCHUP_SPAWN: f32 = -1.0e9;

/// 块间纵向间距(px)。
const BLOCK_GAP: f32 = 8.0;

/// 锚底阈值:滚到离底 ≤ 此值即重新跟随新内容(0002 §6)。
const ANCHOR_THRESHOLD: f32 = 48.0;

/// 把累积的行内码 chip(`[x0,x1,y0,y1]`)推成一个带内边距的圆角底。
fn flush_chip(chip: Option<[f32; 4]>, out: &mut Vec<FrameRect>) {
    if let Some([x0, x1, y0, y1]) = chip {
        out.push(FrameRect {
            pos: [x0 - 2.0, y0 - 1.0],
            size: [(x1 - x0) + 4.0, (y1 - y0) + 2.0],
            color: theme::CODE_CHIP,
            radius: 3.0,
            stroke: 0.0,
        });
    }
}

/// 从块的字形角色派生装饰矩形(代码块底 / 行内码 chip / 引用·Alert 左条 / H1·H2 细线 /
/// 分隔线,Plan 4B1)。颜色令牌见 [`crate::theme`]。
fn block_decorations(cache: &BlockCache, top: f32, max_width: f32, out: &mut Vec<FrameRect>) {
    let code = StyleRole::CodeBlock.as_u32();
    let inline = StyleRole::Code.as_u32();
    let quote = StyleRole::Quote.as_u32();
    let alert = StyleRole::AlertLabel.as_u32();
    let rule = StyleRole::Rule.as_u32();
    let h1 = StyleRole::Heading.as_u32();
    let h2 = StyleRole::Heading2.as_u32();
    let tcell = StyleRole::TableCell.as_u32();
    let theader = StyleRole::TableHeader.as_u32();
    let tstrong = StyleRole::TableStrong.as_u32();
    let tem = StyleRole::TableEm.as_u32();
    let tsep = StyleRole::TableSep.as_u32();
    let (mut cy0, mut cy1) = (f32::MAX, f32::MIN);
    let (mut qy0, mut qy1) = (f32::MAX, f32::MIN);
    let (mut ty0, mut ty1) = (f32::MAX, f32::MIN); // 整表 y 范围
    let (mut tx0, mut tx1) = (f32::MAX, f32::MIN); // 整表 x 范围(外框)
    let mut row_ys: Vec<f32> = Vec::new(); // 各行顶 y(行横线)
    let (mut has_header, mut has_table) = (false, false);
    let (mut has_code, mut has_quote, mut has_head_rule) = (false, false, false);
    let mut alert_label = String::new(); // 非空 = 该块是 Alert
                                         // 行内码 chip:同一行连续 Code 角色聚成一个圆角底,逐行 flush。
    let mut chip: Option<[f32; 4]> = None; // [x0, x1, y0, y1]
    for (j, p) in cache.placed.iter().enumerate() {
        if cache.clusters[j] == "\n" {
            continue;
        }
        let (x0, y0) = (p.pos[0], p.pos[1] + top);
        let (x1, y1) = (x0 + p.size[0], y0 + p.size[1]);
        let r = cache.roles[j];
        if r == code {
            has_code = true;
            cy0 = cy0.min(y0);
            cy1 = cy1.max(y1);
        }
        // 引用与 Alert 共用左条范围;Alert 标签字形拼出类型用于取色。
        if r == quote || r == alert {
            has_quote = true;
            qy0 = qy0.min(y0);
            qy1 = qy1.max(y1);
            if r == alert {
                alert_label.push_str(&cache.clusters[j]);
            }
        }
        if r == h1 || r == h2 {
            has_head_rule = true;
        }
        // 表格(0014 A / 5E.1 #5):收 y/x 范围、列分隔符 x、各行顶 y → 派生网格。
        if r == theader {
            has_header = true;
        }
        if r == theader || r == tcell || r == tstrong || r == tem || r == tsep {
            has_table = true;
            ty0 = ty0.min(y0);
            ty1 = ty1.max(y1);
            tx0 = tx0.min(x0);
            tx1 = tx1.max(x1);
            row_ys.push(y0);
        }
        // 分隔线:零墨 Rule 锚点 → 整宽细线(居其行垂直中点)。
        if r == rule {
            out.push(FrameRect {
                pos: [0.0, (y0 + y1) * 0.5 - 0.75],
                size: [max_width, 1.5],
                color: theme::HR_RULE,
                radius: 0.0,
                stroke: 0.0,
            });
        }
        // 行内码:连续且同行则延展,否则 flush 旧的、起新的。
        if r == inline {
            match chip {
                Some(c) if (c[2] - y0).abs() < 0.5 => {
                    chip = Some([c[0], x1, c[2].min(y0), c[3].max(y1)]);
                }
                _ => {
                    flush_chip(chip, out);
                    chip = Some([x0, x1, y0, y1]);
                }
            }
        } else if chip.is_some() {
            flush_chip(chip, out);
            chip = None;
        }
    }
    flush_chip(chip, out);
    if has_head_rule {
        // GitHub:H1/H2 底部细线,跨整块宽。
        let ry = top + cache.height - 2.0;
        out.push(FrameRect {
            pos: [0.0, ry],
            size: [max_width, 1.5],
            color: theme::HEAD_RULE,
            radius: 0.0,
            stroke: 0.0,
        });
    }
    if has_code {
        out.push(FrameRect {
            pos: [0.0, cy0 - 4.0],
            size: [max_width, (cy1 - cy0) + 8.0],
            color: theme::CODE_BG,
            radius: 6.0,
            stroke: 0.0,
        });
    }
    if has_table {
        let pad = 4.0; // 内容到边框的留白
        let (gx0, gy0) = (tx0 - pad, ty0 - pad);
        let (gw, gh) = ((tx1 - tx0) + 2.0 * pad, (ty1 - ty0) + 2.0 * pad);
        // 各行顶去重排序(表头底界 + 行横线都用)。
        row_ys.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let mut tops: Vec<f32> = Vec::new();
        for &y in &row_ys {
            if tops.last().is_none_or(|&p| (y - p).abs() >= 2.0) {
                tops.push(y);
            }
        }
        // 表头淡底:从**表顶**填到**表头/首行分隔线**(填满整个表头行,非仅字形高 → 修"底色不在表头")。
        if has_header {
            let header_bottom = tops.get(1).copied().unwrap_or(ty1 + pad);
            out.push(FrameRect {
                pos: [gx0, gy0],
                size: [gw, (header_bottom - 1.0 - gy0).max(0.0)],
                color: theme::TABLE_HEADER_BG,
                radius: 0.0,
                stroke: 0.0,
            });
        }
        // 行横线:每个数据行顶(tops[1..])→ 表头分隔 + 行间线。
        for &y in tops.iter().skip(1) {
            out.push(FrameRect {
                pos: [gx0, y - 1.0],
                size: [gw, 1.0],
                color: theme::TABLE_RULE,
                radius: 0.0,
                stroke: 0.0,
            });
        }
        // 竖直列线**暂不画连续 rect**:连续竖线要求列对齐,而列对齐依赖 #7(LXGW 真 2:1);
        // 当前 CJK 未对齐 → 各行分隔符 x 不同,画 rect 会变成错位竖线一片(5E.1 #5/#7)。
        // 列分隔靠每行自带的 `│`(TableSep)字符(随行,不跨行错位);#7 落地后再开连续竖线。
        // 外框(描边)—— 最后画。
        out.push(FrameRect {
            pos: [gx0, gy0],
            size: [gw, gh],
            color: theme::TABLE_RULE,
            radius: 0.0,
            stroke: 1.0,
        });
    }
    if has_quote {
        let is_alert = !alert_label.is_empty();
        // Alert:整块淡底(GitHub 风)+ 类型色左条;普通引用:中性左条。
        if is_alert {
            out.push(FrameRect {
                pos: [0.0, qy0 - 3.0],
                size: [max_width, (qy1 - qy0) + 6.0],
                color: theme::alert_bg(&alert_label),
                radius: 5.0,
                stroke: 0.0,
            });
        }
        out.push(FrameRect {
            pos: [0.0, qy0],
            size: [3.0, qy1 - qy0],
            color: if is_alert {
                theme::alert_bar(&alert_label)
            } else {
                theme::QUOTE_BAR
            },
            radius: 0.0,
            stroke: 0.0,
        });
    }
}

/// 已排版块的缓存(Phase G 块冻结 + Phase H markdown):内容/宽度不变则不重排。
///
/// markdown 渲染隐藏语法标记,故**显示字形序列**(`clusters`/`roles`/`placed`)与源文本
/// `revealed` 不再 1:1;三者长度一致(每个显示 grapheme 一组),spawn_time 在 build 时由
/// `revealed` 近似映射。
struct BlockCache {
    /// 排版时的源 grapheme 数(变了即脏)。
    revealed_len: usize,
    /// 排版时的宽度(变了即脏)。
    width: f32,
    /// 显示 grapheme 文本(markdown 渲染后)。
    clusters: Vec<String>,
    /// 每个显示 grapheme 的样式角色数值。
    roles: Vec<u32>,
    /// 块内相对位置(与 `clusters` 顺序 1:1)。
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

/// 每帧渲染统计(可观测;`?debug` 时 wasm 侧节流打日志)。emit/total 比值暴露"是否每帧发整篇"。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FrameStats {
    /// 本帧实际发射的 glyph 数(经 glyph 级裁剪后)。
    pub frame_glyphs: usize,
    /// 可绘制块的 glyph 总数(裁剪前;emit≈total 说明没裁到/单巨块)。
    pub total_glyphs: usize,
    /// 本帧实际出 glyph 的块数。
    pub visible_blocks: usize,
    /// 可绘制块总数。
    pub total_blocks: usize,
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
    /// 2D 相机(Plan 3 L):平移 + 缩放。Plan2 的 1D scroll 收敛进 pan.y。
    camera: Camera2D,
    /// 锚底:在底部时新内容跟随滚动(0002 §6)。
    stick_to_bottom: bool,
    /// CPU 空间索引(Plan 3 L):逐帧由块 AABB 重建,视口查可见块。
    grid: SpatialGrid,
    /// 调试几何叠加(Plan 4C3):块 AABB / 视口框。
    debug_geometry: bool,
    /// 回合收尾跟踪(Phase I):多信号 + 看门狗,解决"忘了 idle 卡死"。
    turn: TurnTracker,
    /// 上一帧渲染统计(可观测)。
    last_stats: FrameStats,
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
            camera: Camera2D::new(max_width, 600.0),
            stick_to_bottom: true,
            grid: SpatialGrid::new(),
            debug_geometry: false,
            turn: TurnTracker::new(),
            last_stats: FrameStats::default(),
        }
    }

    /// 上一帧渲染统计(可观测;`?debug` 节流打印)。
    pub fn frame_stats(&self) -> FrameStats {
        self.last_stats
    }

    /// 开关调试几何叠加(块 AABB / 视口框,Plan 4C3)。
    pub fn set_debug_geometry(&mut self, on: bool) {
        self.debug_geometry = on;
    }

    /// 当前回合收尾状态(供宿主显示 loading / 收尾,Phase I)。
    pub fn turn_status(&self) -> TurnStatus {
        self.turn.status()
    }

    /// 设视口高度(画布尺寸变化时);视口裁剪与锚底据此。
    pub fn set_viewport_height(&mut self, height: f32) {
        let w = self.camera.viewport()[0];
        self.camera.set_viewport(w, height);
    }

    /// 滚动 `dy` 屏幕像素(正 = 向下/看更新内容)= 相机平移。向上滚脱离锚底,滚回底部自动跟随。
    pub fn scroll_by(&mut self, dy: f32) {
        self.camera.pan_by_screen(0.0, dy);
        if dy < 0.0 {
            self.stick_to_bottom = false;
        }
    }

    /// 围绕屏幕点缩放(Plan 3 L:ctrl+滚轮 / 双指)。缩放即脱离锚底。
    pub fn zoom_by(&mut self, factor: f32, screen_x: f32, screen_y: f32) {
        self.camera.zoom_at(factor, screen_x, screen_y);
        self.stick_to_bottom = false;
    }

    /// 只读相机(供宿主/测试)。
    pub fn camera(&self) -> &Camera2D {
        &self.camera
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

    /// 重连/周期性对账(Phase J):只补 store 里**还没有的 part**(恢复连接间隙错过的历史),
    /// 不动正在 live 的块,避免闪烁/回退(0003 §3.4)。已知 part 的差异交由 `part.updated`
    /// 对账(AR4)。
    pub fn resync_from_snapshot(&mut self, raw: &str) {
        let messages = match parse_snapshot(raw) {
            Ok(m) => m,
            Err(e) => {
                tracing::warn!(target: "M2", error = %e, "resync 快照解析失败");
                return;
            }
        };
        // 过滤出 store 未知的 part。
        let fresh: Vec<crate::protocol::SnapshotMessage> = messages
            .into_iter()
            .filter_map(|m| {
                let new_parts: Vec<_> = m
                    .text_parts
                    .into_iter()
                    .filter(|tp| self.store.part_text(&tp.part_id).is_none())
                    .collect();
                if new_parts.is_empty() {
                    None
                } else {
                    Some(crate::protocol::SnapshotMessage {
                        text_parts: new_parts,
                        ..m
                    })
                }
            })
            .collect();
        if fresh.is_empty() {
            return;
        }
        self.store.apply_snapshot(&fresh);
        for msg in &fresh {
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
        tracing::info!(target: "M3", n = fresh.len(), "resync 补入错过的历史");
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

    /// 更新排版宽度 + 相机视口宽(画布尺寸变化时)。
    pub fn set_max_width(&mut self, max_width: f32) {
        self.max_width = max_width;
        let h = self.camera.viewport()[1];
        self.camera.set_viewport(max_width, h);
    }

    /// 作废所有块的排版缓存,强制下一帧全量重排。
    ///
    /// 字体切换等改变字形宽度但宽度/字数不变的场景:块冻结的脏判据(`revealed_len`/`width`)
    /// 不会自动触发,故显式作废(Plan 4C 调试器换字体用)。
    pub fn mark_layout_dirty(&mut self) {
        for v in &mut self.views {
            v.cache = None;
        }
    }

    /// 推进一帧。
    pub fn frame(&mut self, dt_ms: f64) {
        self.now_ms += dt_ms;
        self.turn.tick(self.now_ms);
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
                }) => {
                    self.store
                        .apply_delta(&part_id, &message_id, &field, &delta);
                    self.turn.on_activity(self.now_ms);
                }
                Ok(Event::PartUpdated { part, .. }) => {
                    self.store.apply_part_updated(&part);
                    self.turn.on_activity(self.now_ms);
                }
                // 会话状态:idle/完成 → 收尾信号;busy/retry → 仍活跃(Phase I)。
                Ok(Event::SessionStatus { status }) => match status.as_str() {
                    "idle" => self.turn.on_settle_signal(),
                    "busy" | "retry" | "working" => self.turn.on_busy(),
                    _ => {}
                },
                Ok(Event::MessageUpdated) => self.turn.on_activity(self.now_ms),
                // 心跳/握手/未知:不改文档状态(AR12)。
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
            let (spans, tables) = parse_markdown_tables(&text); // 0014 B:带表格结构
                                                                // 显示字形序列(markdown 渲染后):与 layout 的 grapheme 切分同源,保证 1:1。
            let mut clusters = Vec::new();
            let mut roles = Vec::new();
            for span in &spans {
                let role = span.role().as_u32();
                for g in graphemes(span.text()) {
                    clusters.push(g.to_owned());
                    roles.push(role);
                }
            }
            let result = self.layout.layout(&spans, &tables, self.max_width);
            self.views[i].cache = Some(BlockCache {
                revealed_len: len,
                width: self.max_width,
                clusters,
                roles,
                placed: result.glyphs,
                height: result.block_height,
            });
        }
    }

    /// 组 FrameData(Plan 3 L):块 AABB 入空间索引 → 相机视口查可见 → 出世界坐标 glyph。
    /// 相机变换在着色器里做;锚底 = 相机 pan.y 跟随底部;块冻结仍在(ensure_layouts)。
    fn build_frame(&mut self) -> FrameData {
        self.ensure_layouts();

        // 1) 收可绘制块(过滤非目标 session / 空块)+ 世界 top(只读借用)。
        let mut drawable: Vec<(usize, f32, f32)> = Vec::new(); // (块下标, top, 高)
        let mut top = 0.0f32;
        let mut total_glyphs = 0usize; // 可观测:裁剪前总量
        for (i, view) in self.views.iter().enumerate() {
            if self.is_filtered(view) {
                continue;
            }
            let Some(c) = &view.cache else { continue };
            if c.placed.is_empty() {
                continue;
            }
            total_glyphs += c.placed.len();
            drawable.push((i, top, c.height));
            top += c.height + BLOCK_GAP;
        }
        let content_height = top;

        // 2) 重建空间索引(块 AABB)。
        self.grid.clear();
        for &(i, t, h) in &drawable {
            self.grid.insert(i, &Rect::new(0.0, t, self.max_width, h));
        }

        // 3) 锚底:相机 pan.y 跟随底部并夹取。
        let visible_h = self.camera.viewport()[1] / self.camera.zoom();
        let max_pan_y = (content_height - visible_h).max(0.0);
        let mut pan = self.camera.pan();
        if self.stick_to_bottom {
            pan[1] = max_pan_y;
        }
        pan[1] = pan[1].clamp(0.0, max_pan_y);
        self.camera.set_pan(pan[0], pan[1]);
        if pan[1] >= max_pan_y - ANCHOR_THRESHOLD {
            self.stick_to_bottom = true;
        }

        // 4) 视口查可见块(grid 是 broad phase)→ 实际 AABB narrow phase → 出世界坐标 glyph。
        let boxes: std::collections::HashMap<usize, (f32, f32)> =
            drawable.iter().map(|&(i, t, h)| (i, (t, h))).collect();
        let visible = self.camera.visible_world_rect();
        let ids = self.grid.query(&visible);
        let mut glyphs = Vec::new();
        let mut rects: Vec<FrameRect> = Vec::new();
        let mut visible_blocks = 0usize; // 可观测:实际出 glyph 的块数
        for id in ids {
            let view = &self.views[id];
            let Some(cache) = &view.cache else { continue };
            let (block_top, block_h) = boxes.get(&id).copied().unwrap_or((0.0, 0.0));
            if !Rect::new(0.0, block_top, self.max_width, block_h).intersects(&visible) {
                continue; // narrow phase:实际矩形不相交 → 裁掉
            }
            block_decorations(cache, block_top, self.max_width, &mut rects); // 4B 装饰
            let glyphs_before = glyphs.len();
            let last_spawn = view.revealed.len().saturating_sub(1);
            for (j, placed) in cache.placed.iter().enumerate() {
                if cache.clusters[j] == "\n" {
                    continue;
                }
                // glyph 级 y 裁剪:单条长消息是一个巨块,块级裁剪不够 —— 块内只发与视口相交
                // 的字,把每帧发射量从"整篇"降到"约一屏",根治长消息的每帧分配风暴。
                let gworld = Rect::new(
                    placed.pos[0],
                    placed.pos[1] + block_top,
                    placed.size[0],
                    placed.size[1],
                );
                if !gworld.intersects(&visible) {
                    continue;
                }
                let spawn = view
                    .revealed
                    .get(j.min(last_spawn))
                    .map_or(self.now_ms as f32, |r| r.1);
                glyphs.push(FrameGlyph {
                    cluster: cache.clusters[j].clone(),
                    pos: [placed.pos[0], placed.pos[1] + block_top], // 世界坐标
                    size: placed.size,
                    spawn_time: spawn,
                    style: cache.roles[j],
                    // 身份(0016/0017):块在 views 里的下标(append-only 稳定)+ 块内 placed 下标。
                    block_seq: id as u32,
                    glyph_idx: j as u32,
                });
            }
            if glyphs.len() > glyphs_before {
                visible_blocks += 1;
            }
        }
        // 调试几何叠加(Plan 4C3):块 AABB(描边)+ 视口框。
        if self.debug_geometry {
            for &(_, t, h) in &drawable {
                if Rect::new(0.0, t, self.max_width, h).intersects(&visible) {
                    rects.push(FrameRect {
                        pos: [0.0, t],
                        size: [self.max_width, h],
                        color: theme::DBG_BLOCK,
                        radius: 0.0,
                        stroke: 1.5,
                    });
                }
            }
            rects.push(FrameRect {
                pos: [visible.x, visible.y],
                size: [visible.w, visible.h],
                color: theme::DBG_VIEW,
                radius: 0.0,
                stroke: 2.0,
            });
        }

        self.last_stats = FrameStats {
            frame_glyphs: glyphs.len(),
            total_glyphs,
            visible_blocks,
            total_blocks: drawable.len(),
        };
        FrameData {
            rects,
            glyphs,
            time_ms: self.now_ms as f32,
            cam_pan: self.camera.pan(),
            cam_zoom: self.camera.zoom(),
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
    use crate::content::StyleRole;
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
    fn streaming_emphasis_close_flips_role() {
        // Plan 5C:活动块逐帧重解析(0017 §3);`**bold**` 闭合后该字带 Bold 角色、无字面 `*`。
        let player = Player::from_pairs(vec![(0.0, delta("p", "a **bold** c"))], 16.0);
        let mut eng = Engine::new(
            player,
            MonospaceLayout::default(),
            CollectSink::default(),
            500.0,
            800.0,
        );
        for _ in 0..40 {
            eng.frame(16.0);
        }
        let f = eng.sink().last().expect("frame");
        assert!(
            !f.glyphs.iter().any(|g| g.cluster == "*"),
            "闭合后不应有字面 *"
        );
        let bold = StyleRole::Bold.as_u32();
        assert!(
            f.glyphs.iter().any(|g| g.cluster == "b" && g.style == bold),
            "bold 文本应是 Bold 角色"
        );
    }

    #[test]
    fn streaming_setext_upgrades_to_heading() {
        // Plan 5C:setext —— 下一行 `===` 到达 → 上一行回溯升级为标题(lookahead 重解析)。
        let player = Player::from_pairs(vec![(0.0, delta("p", "Title\n==="))], 16.0);
        let mut eng = Engine::new(
            player,
            MonospaceLayout::default(),
            CollectSink::default(),
            500.0,
            800.0,
        );
        for _ in 0..40 {
            eng.frame(16.0);
        }
        let f = eng.sink().last().expect("frame");
        let h1 = StyleRole::Heading.as_u32(); // setext `===` = H1
        assert!(
            f.glyphs.iter().any(|g| g.cluster == "T" && g.style == h1),
            "setext 下划线到达后标题行应升级为 Heading"
        );
        assert!(
            !f.glyphs.iter().any(|g| g.cluster == "="),
            "setext 下划线不应显形"
        );
    }

    #[test]
    fn glyph_identity_is_append_stable() {
        // Plan 5A/0017 §6:append-only → (block_seq, glyph_idx) 跨重排稳定。首字身份不随追加变。
        let player = Player::from_pairs(vec![(0.0, delta("p", "hello world"))], 16.0);
        let mut eng = Engine::new(
            player,
            MonospaceLayout::default(),
            CollectSink::default(),
            40.0, // 慢吐:逐字揭示
            800.0,
        );
        let mut first_seen: Option<(u32, u32)> = None;
        for _ in 0..120 {
            eng.frame(16.0);
            if let Some(f) = eng.sink().last() {
                if let Some(g) = f.glyphs.iter().find(|g| g.cluster == "h") {
                    let id = (g.block_seq, g.glyph_idx);
                    if let Some(prev) = first_seen {
                        assert_eq!(id, prev, "首字身份应跨帧稳定");
                    }
                    first_seen = Some(id);
                    assert_eq!(g.block_seq, 0, "单块 block_seq=0");
                }
            }
        }
        assert!(first_seen.is_some(), "应揭示出首字");
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
        fn layout(
            &mut self,
            spans: &[crate::StyledSpan],
            tables: &[crate::TableRegion],
            w: f32,
        ) -> crate::LayoutResult {
            self.calls.set(self.calls.get() + 1);
            self.inner.layout(spans, tables, w)
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

    #[test]
    fn turn_settles_via_watchdog_even_without_idle() {
        // Phase I:delta 到达 → Active;之后久无事件(模型忘了 idle)→ 看门狗强制收尾。
        let player = Player::from_pairs(vec![(0.0, delta("p", "hi"))], 16.0);
        let mut eng = Engine::new(
            player,
            MonospaceLayout::default(),
            CollectSink::default(),
            500.0,
            800.0,
        );
        eng.frame(16.0);
        assert_eq!(eng.turn_status(), crate::TurnStatus::Active);
        for _ in 0..6 {
            eng.frame(8_000.0); // 推进 ~48s,无新事件
        }
        assert_eq!(eng.turn_status(), crate::TurnStatus::Settled);
    }

    #[test]
    fn resync_adds_missed_history_without_disturbing_live() {
        // Phase J:p1 正在 live;resync 补入错过的历史 p0,但不重置 live 的 p1。
        let player = Player::from_pairs(vec![(0.0, delta("p1", "live"))], 16.0);
        let mut eng = Engine::new(
            player,
            MonospaceLayout::default(),
            CollectSink::default(),
            500.0,
            800.0,
        );
        for _ in 0..10 {
            eng.frame(16.0);
        }
        assert_eq!(eng.store().part_text("p1"), Some("live"));
        // resync:含错过的 p0 + 已知的 p1(快照里 p1 更长,但应被跳过不动 live)。
        let snap = r#"[
            {"info":{"id":"m0","sessionID":"s","role":"a"},"parts":[{"type":"text","id":"p0","messageID":"m0","text":"missed"}]},
            {"info":{"id":"m1","sessionID":"s","role":"a"},"parts":[{"type":"text","id":"p1","messageID":"m1","text":"liveXXXX"}]}
        ]"#;
        eng.resync_from_snapshot(snap);
        eng.frame(16.0);
        assert_eq!(
            eng.store().part_text("p0"),
            Some("missed"),
            "应补入错过历史"
        );
        assert_eq!(
            eng.store().part_text("p1"),
            Some("live"),
            "live 块不应被 resync 覆盖"
        );
    }

    #[test]
    fn camera_pan_up_shows_earlier_blocks() {
        // Plan 3 L:小视口锚底先看底部块;向上滚(相机平移)→ 看到顶部块,底部裁掉。
        let mut eng = Engine::new(
            Player::from_pairs(vec![], 16.0),
            MonospaceLayout::default(),
            CollectSink::default(),
            200.0,
            800.0,
        );
        eng.set_viewport_height(25.0);
        let snap = r#"[
            {"info":{"id":"m1","sessionID":"s","role":"a"},"parts":[{"type":"text","id":"p1","messageID":"m1","text":"AAAA"}]},
            {"info":{"id":"m2","sessionID":"s","role":"a"},"parts":[{"type":"text","id":"p2","messageID":"m2","text":"BBBB"}]},
            {"info":{"id":"m3","sessionID":"s","role":"a"},"parts":[{"type":"text","id":"p3","messageID":"m3","text":"CCCC"}]},
            {"info":{"id":"m4","sessionID":"s","role":"a"},"parts":[{"type":"text","id":"p4","messageID":"m4","text":"DDDD"}]},
            {"info":{"id":"m5","sessionID":"s","role":"a"},"parts":[{"type":"text","id":"p5","messageID":"m5","text":"EEEE"}]}
        ]"#;
        eng.prime_from_snapshot(snap);
        eng.frame(16.0);
        assert!(
            eng.sink().visible_text().contains("EEEE"),
            "初始锚底应见底部"
        );
        eng.scroll_by(-1000.0); // 滚到顶
        eng.frame(16.0);
        let v = eng.sink().visible_text();
        assert!(v.contains("AAAA"), "向上滚应见顶部: {v}");
        assert!(!v.contains("EEEE"), "底部应被裁掉: {v}");
        assert!((eng.camera().zoom() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn code_block_emits_background_rect() {
        // Plan 4B:代码块由角色派生底色 rect。
        let mut eng = Engine::new(
            Player::from_pairs(vec![], 16.0),
            MonospaceLayout::default(),
            CollectSink::default(),
            200.0,
            800.0,
        );
        let snap = r#"[{"info":{"id":"m1","sessionID":"s","role":"a"},
            "parts":[{"type":"text","id":"p1","messageID":"m1","text":"```\nlet x = 1;\n```"}]}]"#;
        eng.prime_from_snapshot(snap);
        eng.frame(16.0);
        let f = eng.sink().last().expect("frame");
        assert!(!f.rects.is_empty(), "代码块应有底色 rect");
        assert!(f.rects.iter().all(|r| r.stroke == 0.0), "装饰是填充非描边");
    }

    #[test]
    fn inline_code_emits_chip_rect() {
        // Plan 4B1:行内码 `x` 派生一块 chip 底(填充,非整宽)。
        let mut eng = Engine::new(
            Player::from_pairs(vec![], 16.0),
            MonospaceLayout::default(),
            CollectSink::default(),
            200.0,
            800.0,
        );
        let snap = r#"[{"info":{"id":"m1","sessionID":"s","role":"a"},
            "parts":[{"type":"text","id":"p1","messageID":"m1","text":"run `cargo test` now"}]}]"#;
        eng.prime_from_snapshot(snap);
        eng.frame(16.0);
        let f = eng.sink().last().expect("frame");
        assert!(!f.rects.is_empty(), "行内码应有 chip");
        assert!(
            f.rects.iter().all(|r| r.size[0] < 800.0),
            "chip 不应占整块宽"
        );
    }

    #[test]
    fn github_alert_emits_tinted_bar_and_bg() {
        // Plan 4B1:`> [!WARNING]` → 类型色左条(实心)+ 整块淡底。
        let mut eng = Engine::new(
            Player::from_pairs(vec![], 16.0),
            MonospaceLayout::default(),
            CollectSink::default(),
            200.0,
            800.0,
        );
        let snap = r#"[{"info":{"id":"m1","sessionID":"s","role":"a"},
            "parts":[{"type":"text","id":"p1","messageID":"m1","text":"> [!WARNING]\n> be careful"}]}]"#;
        eng.prime_from_snapshot(snap);
        eng.frame(16.0);
        let f = eng.sink().last().expect("frame");
        let warn = crate::theme::alert_bar("WARNING");
        let close = |a: [f32; 4], b: [f32; 4]| a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-6);
        assert!(
            f.rects.iter().any(|r| close(r.color, warn)),
            "应有 WARNING 类型色左条"
        );
        // 淡底:整宽、低 alpha。
        assert!(
            f.rects
                .iter()
                .any(|r| r.size[0] > 700.0 && r.color[3] < 0.2 && r.color[3] > 0.0),
            "应有整块淡底"
        );
    }

    #[test]
    fn thematic_break_emits_full_width_rule() {
        let mut eng = Engine::new(
            Player::from_pairs(vec![], 16.0),
            MonospaceLayout::default(),
            CollectSink::default(),
            200.0,
            800.0,
        );
        let snap = r#"[{"info":{"id":"m1","sessionID":"s","role":"a"},
            "parts":[{"type":"text","id":"p1","messageID":"m1","text":"above\n\n---\n\nbelow"}]}]"#;
        eng.prime_from_snapshot(snap);
        eng.frame(16.0);
        let f = eng.sink().last().expect("frame");
        assert!(
            f.rects
                .iter()
                .any(|r| r.size[0] > 700.0 && r.size[1] <= 2.0 && r.stroke == 0.0),
            "分隔线应是整宽细实线 rect"
        );
    }

    #[test]
    fn debug_geometry_adds_stroked_rects() {
        // Plan 4C3:开调试几何 → 块 AABB / 视口框(描边)。
        let mut eng = Engine::new(
            Player::from_pairs(vec![(0.0, delta("p", "hi"))], 16.0),
            MonospaceLayout::default(),
            CollectSink::default(),
            500.0,
            800.0,
        );
        eng.set_debug_geometry(true);
        for _ in 0..5 {
            eng.frame(16.0);
        }
        let f = eng.sink().last().expect("frame");
        assert!(
            f.rects.iter().any(|r| r.stroke > 0.0),
            "调试几何应有描边 rect"
        );
    }
}
