//! morph(M8 / 0016)— streaming 形变机制:past→current 双关键帧 + retained keyed scene。
//!
//! 与内容无关的渲染机制(引擎):上游(0017 驱动)每产一份「活跃区布局快照」就 [`Scene::commit`],
//! join 出生灭/过渡;每帧 [`Scene::instances`] 走 `f(past,current,t)` 线性插值发射单态实例(路 B,
//! CPU mix,0016 §4.5)。静止/冻结节点 `past=None` → 零成本单态旁路。纯 CPU,native 可测。
//!
//! v1 范围:**几何(pos/size)补间**——位移/缩放不跳变(品味硬约束)。alpha 入场淡入仍走
//! `spawn_time` 着色器路径(不改 shader);exit 节点 v1 直接移除(淡出留后续)。`Geom.alpha` /
//! `Phase` 保留在数据模型里供 settle 判定与后续升级(GPU 双态路 A / exit 淡出)。

use std::collections::{HashMap, HashSet};

use crate::scene::GpuInstance;

/// 跨重排稳定的字块身份(0016 §4.1)。打包成 u64 作 key(高 32 = block_seq,低 32 = glyph_idx)。
/// 稳定性来自数据源 append-only(0017 §6);机制只依赖"它稳定"。
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct NodeId(u64);

impl NodeId {
    pub fn new(block_seq: u32, glyph_idx: u32) -> Self {
        Self((u64::from(block_seq) << 32) | u64::from(glyph_idx))
    }
}

/// 参与插值的几何 + alpha(0016 §4.2)。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Geom {
    pub pos: [f32; 2],
    pub size: [f32; 2],
    pub alpha: f32,
}

/// 不插值的身份载荷(0016 §4.3):uv/style/layer/kind + spawn_time(着色器淡入)。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sample {
    pub uv: [f32; 4],
    pub style: u32,
    pub layer: u32,
    pub kind: u32,
    pub spawn_time: f32,
}

/// 节点生命周期相位(0016 §4.3)。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Phase {
    Enter,
    Update,
    Exit,
}

/// 渲染节点:目标态 `current` + 可选过去态 `past`(过渡窗口内才有)。
#[derive(Clone, Copy, Debug)]
struct RenderNode {
    sample: Sample,
    current: Geom,
    past: Option<Geom>,
    t_start: f32,
    phase: Phase,
}

impl RenderNode {
    fn entering(g: Geom, s: Sample, now: f32) -> Self {
        Self {
            sample: s,
            current: g,
            past: None,
            t_start: now,
            phase: Phase::Enter,
        }
    }

    /// 当前**显示**几何:`past=None` → `current`;否则按归一化 t 插值(0016 §5)。
    fn displayed(&self, now: f32, dur: f32) -> Geom {
        match self.past {
            None => self.current,
            Some(p) => {
                let t = ((now - self.t_start) / dur.max(1.0)).clamp(0.0, 1.0);
                lerp_geom(p, self.current, ease_cubic_out(t))
            }
        }
    }

    /// 过渡是否已完成(可塌回 `past=None`)。
    fn done(&self, now: f32, dur: f32) -> bool {
        self.past.is_some() && now - self.t_start >= dur
    }
}

/// 缓动:cubic-out `1-(1-t)³`(标准公式自写,不抄 lygia,Plan5「shader 复用」约定)。
fn ease_cubic_out(t: f32) -> f32 {
    let u = 1.0 - t;
    1.0 - u * u * u
}

fn lerp(a: f32, b: f32, e: f32) -> f32 {
    a + (b - a) * e
}

fn lerp_geom(a: Geom, b: Geom, e: f32) -> Geom {
    Geom {
        pos: [lerp(a.pos[0], b.pos[0], e), lerp(a.pos[1], b.pos[1], e)],
        size: [lerp(a.size[0], b.size[0], e), lerp(a.size[1], b.size[1], e)],
        alpha: lerp(a.alpha, b.alpha, e),
    }
}

/// 几何近似相等(join 判定是否变化;避免浮点抖动误判)。
fn geom_eq(a: Geom, b: Geom) -> bool {
    let close = |x: f32, y: f32| (x - y).abs() < 0.01;
    close(a.pos[0], b.pos[0])
        && close(a.pos[1], b.pos[1])
        && close(a.size[0], b.size[0])
        && close(a.size[1], b.size[1])
        && close(a.alpha, b.alpha)
}

/// 保留态场景(0016 §4.4):retained keyed `HashMap<NodeId,RenderNode>` + join。`dur_ms` 是
/// policy 参数(过渡时长)。只装活跃区,与会话长度无关(冻结块不进 Scene,0016 §6)。
pub struct Scene {
    nodes: HashMap<NodeId, RenderNode>,
    dur_ms: f32,
}

impl Scene {
    pub fn new(dur_ms: f32) -> Self {
        Self {
            nodes: HashMap::new(),
            dur_ms: dur_ms.max(1.0),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// 是否所有节点都已静止(无过渡进行中)→ 可冻结(0016 §6)。
    pub fn all_settled(&self, now: f32) -> bool {
        self.nodes
            .values()
            .all(|n| n.past.is_none() || now - n.t_start >= self.dur_ms)
    }

    /// 清空(块全冻结/会话切换时)。
    pub fn clear(&mut self) {
        self.nodes.clear();
    }

    /// 提交一份活跃区布局快照,join 标注生灭/过渡(0016 §4.4)。`now` = 当前帧 ms。
    pub fn commit(&mut self, layout: &[(NodeId, Geom, Sample)], now: f32) {
        let mut seen = HashSet::with_capacity(layout.len());
        for (id, geom, sample) in layout {
            seen.insert(*id);
            match self.nodes.get_mut(id) {
                Some(n) => {
                    n.sample = *sample; // 载荷恒取最新(uv/style/page 可变,不插值)
                    if !geom_eq(n.current, *geom) {
                        // 关键:past 取「此刻显示态」→ 过渡可被打断而不回跳(0016 §4.4)。
                        n.past = Some(n.displayed(now, self.dur_ms));
                        n.current = *geom;
                        n.t_start = now;
                        n.phase = Phase::Update;
                    }
                }
                None => {
                    self.nodes
                        .insert(*id, RenderNode::entering(*geom, *sample, now));
                }
            }
        }
        // 不在新快照里 = exit。v1 直接标记;`instances` 下次清除(淡出留后续)。
        for (id, n) in &mut self.nodes {
            if !seen.contains(id) && !matches!(n.phase, Phase::Exit) {
                n.phase = Phase::Exit;
                n.t_start = now;
            }
        }
    }

    /// 发射本帧实例(0016 §5):插值显示几何 + 不插值载荷 → `GpuInstance`。
    /// 顺带塌缩已完成过渡(`past=None`)、清除 exit 节点。
    pub fn instances(&mut self, now: f32) -> Vec<GpuInstance> {
        let dur = self.dur_ms;
        // exit 节点 v1 立即清除(无淡出);其余塌缩完成的过渡。
        self.nodes.retain(|_, n| !matches!(n.phase, Phase::Exit));
        let mut out = Vec::with_capacity(self.nodes.len());
        for n in self.nodes.values_mut() {
            if n.done(now, dur) {
                n.current = n.displayed(now, dur);
                n.past = None;
                n.phase = Phase::Update;
            }
            let g = n.displayed(now, dur);
            out.push(GpuInstance {
                pos: g.pos,
                size: g.size,
                uv: n.sample.uv,
                spawn_time: n.sample.spawn_time,
                style: n.sample.style,
                layer: n.sample.layer,
                kind: n.sample.kind,
            });
        }
        out
    }
}

// ── SDF 面板形变(0018 §5 / Plan 6D)──────────────────────────────────────────
// 表格框/网格随 streaming 列变宽不跳变:与字(`Scene`)同款 CPU-mix 补间(路 B),同 `dur`/缓动 →
// 框字同步。只补**几何**(box pos/size + header/col/row 比例);颜色/AO/线宽不插值(snap,取最新)。
// col 列数稳定 → 逐元素补;row 行数随吐字增 → 补公共前缀、新行直接出现(其字本就 spawn 淡入)。

/// 面板可补间几何(0018 §5)。`id` 由上游(app)按 `(block_seq, 表序号)` 打包,append-only 稳定。
#[derive(Clone, Debug, PartialEq)]
pub struct PanelGeom {
    pub pos: [f32; 2],
    pub size: [f32; 2],
    pub header_ratio: f32,
    pub col_ratios: Vec<f32>,
    pub row_ratios: Vec<f32>,
}

/// 比例数组插值:逐元素补到 `c` 的长度;`p` 缺位(新行)→ 直接取 `c`(出现)。
fn lerp_ratios(p: &[f32], c: &[f32], e: f32) -> Vec<f32> {
    c.iter()
        .enumerate()
        .map(|(i, &cv)| p.get(i).map_or(cv, |&pv| lerp(pv, cv, e)))
        .collect()
}

fn lerp_panel(a: &PanelGeom, b: &PanelGeom, e: f32) -> PanelGeom {
    PanelGeom {
        pos: [lerp(a.pos[0], b.pos[0], e), lerp(a.pos[1], b.pos[1], e)],
        size: [lerp(a.size[0], b.size[0], e), lerp(a.size[1], b.size[1], e)],
        header_ratio: lerp(a.header_ratio, b.header_ratio, e),
        col_ratios: lerp_ratios(&a.col_ratios, &b.col_ratios, e),
        row_ratios: lerp_ratios(&a.row_ratios, &b.row_ratios, e),
    }
}

fn panel_geom_eq(a: &PanelGeom, b: &PanelGeom) -> bool {
    let close = |x: f32, y: f32| (x - y).abs() < 0.01;
    let arr =
        |p: &[f32], c: &[f32]| p.len() == c.len() && p.iter().zip(c).all(|(x, y)| close(*x, *y));
    close(a.pos[0], b.pos[0])
        && close(a.pos[1], b.pos[1])
        && close(a.size[0], b.size[0])
        && close(a.size[1], b.size[1])
        && close(a.header_ratio, b.header_ratio)
        && arr(&a.col_ratios, &b.col_ratios)
        && arr(&a.row_ratios, &b.row_ratios)
}

struct PanelNode {
    current: PanelGeom,
    past: Option<PanelGeom>,
    t_start: f32,
}

impl PanelNode {
    fn displayed(&self, now: f32, dur: f32) -> PanelGeom {
        match &self.past {
            None => self.current.clone(),
            Some(p) => {
                let t = ((now - self.t_start) / dur.max(1.0)).clamp(0.0, 1.0);
                lerp_panel(p, &self.current, ease_cubic_out(t))
            }
        }
    }

    fn done(&self, now: f32, dur: f32) -> bool {
        self.past.is_some() && now - self.t_start >= dur
    }
}

/// 面板保留态场景(0018 §5 / Plan 6D):`id → PanelNode`,join + 几何补间。与 [`Scene`] 并列,
/// 同 `dur`(同步)。每帧 [`commit`](Self::commit) 全部可见面板,再用 [`displayed`](Self::displayed)
/// 取插值几何建参数。未出现的面板自动丢弃(v1 无退出淡出,同 0016)。
pub struct PanelScene {
    nodes: HashMap<u64, PanelNode>,
    dur_ms: f32,
}

impl PanelScene {
    pub fn new(dur_ms: f32) -> Self {
        Self {
            nodes: HashMap::new(),
            dur_ms: dur_ms.max(1.0),
        }
    }

    /// 提交本帧全部面板几何:几何变 → past 取显示态(可打断不回跳);未变且过渡完成 → 塌回 None;
    /// 不在本帧的面板丢弃。`now` = 当前帧 ms。
    pub fn commit(&mut self, incoming: &[(u64, PanelGeom)], now: f32) {
        let seen: HashSet<u64> = incoming.iter().map(|(id, _)| *id).collect();
        for (id, geom) in incoming {
            match self.nodes.get_mut(id) {
                Some(n) => {
                    if !panel_geom_eq(&n.current, geom) {
                        n.past = Some(n.displayed(now, self.dur_ms));
                        n.current = geom.clone();
                        n.t_start = now;
                    } else if n.done(now, self.dur_ms) {
                        n.past = None; // 过渡完成塌缩
                    }
                }
                None => {
                    self.nodes.insert(
                        *id,
                        PanelNode {
                            current: geom.clone(),
                            past: None,
                            t_start: now,
                        },
                    );
                }
            }
        }
        self.nodes.retain(|id, _| seen.contains(id)); // 退出:丢弃(v1 无淡出)
    }

    /// 取某面板的本帧插值几何(commit 之后调)。缺失 → None(调用方回退原始几何)。
    pub fn displayed(&self, id: u64, now: f32) -> Option<PanelGeom> {
        self.nodes.get(&id).map(|n| n.displayed(now, self.dur_ms))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn geom(x: f32, y: f32) -> Geom {
        Geom {
            pos: [x, y],
            size: [10.0, 10.0],
            alpha: 1.0,
        }
    }
    fn sample() -> Sample {
        Sample {
            uv: [0.0, 0.0, 1.0, 1.0],
            style: 0,
            layer: 0,
            kind: 1,
            spawn_time: 0.0,
        }
    }
    const DUR: f32 = 100.0;

    fn near(a: f32, b: f32) -> bool {
        (a - b).abs() < 0.01
    }

    #[test]
    fn ease_endpoints() {
        assert!((ease_cubic_out(0.0) - 0.0).abs() < 1e-6);
        assert!((ease_cubic_out(1.0) - 1.0).abs() < 1e-6);
        assert!(ease_cubic_out(0.5) > 0.5, "cubic-out 前快");
    }

    #[test]
    fn same_id_geom_change_sets_past() {
        let id = NodeId::new(0, 0);
        let mut s = Scene::new(DUR);
        s.commit(&[(id, geom(0.0, 0.0), sample())], 0.0);
        // 入场:past=None(几何静止,靠 spawn_time 淡入)。
        assert!(s.nodes[&id].past.is_none());
        s.commit(&[(id, geom(100.0, 0.0), sample())], 0.0);
        let n = &s.nodes[&id];
        let past = n.past.expect("几何变 → past 应记下旧态");
        assert!(near(n.current.pos[0], 100.0));
        assert!(near(past.pos[0], 0.0));
    }

    #[test]
    fn interpolates_between_keyframes() {
        let id = NodeId::new(1, 2);
        let mut s = Scene::new(DUR);
        s.commit(&[(id, geom(0.0, 0.0), sample())], 0.0);
        s.commit(&[(id, geom(100.0, 0.0), sample())], 0.0);
        let inst = s.instances(50.0); // t=0.5
        let x = inst[0].pos[0];
        assert!(x > 50.0 && x < 100.0, "cubic-out 半程过半: {x}");
    }

    #[test]
    fn interrupt_takes_displayed_no_snap_back() {
        let id = NodeId::new(0, 0);
        let mut s = Scene::new(DUR);
        s.commit(&[(id, geom(0.0, 0.0), sample())], 0.0);
        s.commit(&[(id, geom(100.0, 0.0), sample())], 0.0);
        // 半程被打断,新目标 200。
        let mid = s.nodes[&id].displayed(50.0, DUR).pos[0];
        s.commit(&[(id, geom(200.0, 0.0), sample())], 50.0);
        let n = &s.nodes[&id];
        // 新 past = 打断那刻的显示态(≈mid),不是原点 0 → 不回跳。
        let past = n.past.expect("打断应记 past");
        assert!((past.pos[0] - mid).abs() < 0.5, "past 应取显示态");
        assert!(near(n.current.pos[0], 200.0));
    }

    #[test]
    fn transition_collapses_after_dur() {
        let id = NodeId::new(0, 0);
        let mut s = Scene::new(DUR);
        s.commit(&[(id, geom(0.0, 0.0), sample())], 0.0);
        s.commit(&[(id, geom(100.0, 0.0), sample())], 0.0);
        let _ = s.instances(200.0); // t>1 → 塌缩
        assert!(s.nodes[&id].past.is_none(), "过渡完成 → past 塌回 None");
        assert!(near(s.nodes[&id].current.pos[0], 100.0));
        assert!(s.all_settled(200.0));
    }

    #[test]
    fn exit_node_removed() {
        let a = NodeId::new(0, 0);
        let b = NodeId::new(0, 1);
        let mut s = Scene::new(DUR);
        s.commit(
            &[
                (a, geom(0.0, 0.0), sample()),
                (b, geom(10.0, 0.0), sample()),
            ],
            0.0,
        );
        assert_eq!(s.len(), 2);
        s.commit(&[(a, geom(0.0, 0.0), sample())], 10.0); // b 消失
        let _ = s.instances(10.0);
        assert_eq!(s.len(), 1, "exit 节点应清除");
        assert!(s.nodes.contains_key(&a));
    }

    #[test]
    fn static_node_zero_retained() {
        let id = NodeId::new(0, 0);
        let mut s = Scene::new(DUR);
        s.commit(&[(id, geom(0.0, 0.0), sample())], 0.0);
        s.commit(&[(id, geom(0.0, 0.0), sample())], 16.0); // 几何不变
        assert!(s.nodes[&id].past.is_none(), "静止节点零保留态");
        assert!(s.all_settled(16.0));
    }

    fn pgeom(w: f32, h: f32, col: f32, rows: Vec<f32>) -> PanelGeom {
        PanelGeom {
            pos: [0.0, 0.0],
            size: [w, h],
            header_ratio: 0.5,
            col_ratios: vec![col],
            row_ratios: rows,
        }
    }

    #[test]
    fn panel_reflow_tweens_and_rows_grow() {
        let mut ps = PanelScene::new(DUR);
        let id = 1u64;
        ps.commit(&[(id, pgeom(100.0, 20.0, 0.5, vec![]))], 0.0);
        // 列变宽(0.5→0.6)+ 框变宽(100→140)+ 新增一行(0→1 条行线)。
        ps.commit(&[(id, pgeom(140.0, 40.0, 0.6, vec![0.6]))], 0.0);
        let mid = ps.displayed(id, 50.0).expect("present"); // t=0.5
        assert!(
            mid.size[0] > 100.0 && mid.size[0] < 140.0,
            "框宽补间中: {}",
            mid.size[0]
        );
        assert!(
            mid.col_ratios[0] > 0.5 && mid.col_ratios[0] < 0.6,
            "列比例补间"
        );
        assert_eq!(
            mid.row_ratios.len(),
            1,
            "新增行立即出现(无 past 直接取 current)"
        );
        // 过渡完成 → 塌回 current。
        ps.commit(&[(id, pgeom(140.0, 40.0, 0.6, vec![0.6]))], 200.0);
        let done = ps.displayed(id, 200.0).expect("present");
        assert!((done.size[0] - 140.0).abs() < 0.5, "完成后取 current");
    }

    #[test]
    fn panel_unseen_dropped() {
        let mut ps = PanelScene::new(DUR);
        ps.commit(&[(1u64, pgeom(10.0, 10.0, 0.5, vec![]))], 0.0);
        ps.commit(&[(2u64, pgeom(10.0, 10.0, 0.5, vec![]))], 0.0); // 1 未出现 → 丢弃
        assert!(ps.displayed(1, 0.0).is_none(), "退出面板应丢弃");
        assert!(ps.displayed(2, 0.0).is_some());
    }
}
