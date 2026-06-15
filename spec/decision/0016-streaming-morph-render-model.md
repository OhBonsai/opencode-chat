# 决策记录 0016:streaming 形变渲染机制 —— past→current 双关键帧 + retained keyed scene

- 日期:2026-06-15
- 状态:已采纳(机制定调;具体动画/缓动留作上层 policy,见 §8)
- 前置:0001 §2.2(content→layout→render 契约 / 每帧一次跨界 AR10)、0005(回合聚合/settle、**块冻结**)、0011(quad/SDF 图元、spawn_time 淡入)、`design/thinking.md §2`(字块 move / FLIP)
- 触发:Plan 5「markdown 还原」要求 streaming 中内容几何变化(列宽/行高、闭合致重排)**不许 snap**(品味硬约束:位移用 translate、尺寸用 scale 补间)。现管线只有单一状态 + 淡入,无法表达"任意 past→current 变化"。
- 定位:**本篇 = 与内容无关的渲染机制(引擎)**。"markdown 如何产出本机制所需的输入(何时产生 past/current 端点)"是其专用驱动,见 **[0017](0017-markdown-streaming-landing.md)**。

---

## 1. 现状与缺口

现管线(`crates/render/src/scene.rs` / `shaders/glyph.wgsl`):`GpuInstance` 只带**一套**几何;着色器 `world = pos + c*size`(单一位置),唯一随时间变的是 `alpha`(**仅淡入**);每帧从布局**无状态重建**,不保留"上一帧某字块在哪"。

能力矩阵:出现(fade)= 有;**移动 / 缩放 = 无;"过去状态"不存在**。故 streaming 一旦重排只能瞬移 = 跳变。缺口:**没有"过去态"、没有跨重排的身份、没有插值机制**。

## 2. 设计目标与边界

1. **机制 ≠ 策略**:管线只提供"任意 past→current 的可插值表示";具体像 translate 还是 scale、缓动曲线、时长全是上层 policy,本 ADR **不定**(§8)。
2. **退化为今天**:静止/冻结内容退化成单状态单 draw,**零额外成本**。
3. **infinite session 内存有界**:retained 态只覆盖活跃区,**与历史长度无关**。
4. **与内容无关**:机制只认"带稳定 id 的活跃区布局快照"(§7 上游契约),不感知 markdown。

## 3. 为何不必上重型机制(通用部分)

业界"状态随时间变、要在新旧态间平滑过渡"的两套先例,我们**取其观念、弃其引擎**:

- **GGPO 回滚 netcode**:预测 → 投机执行 → 真值到了不符则回滚重模拟;**低延迟修正肉眼不可见**([ggpo](https://github.com/pond3r/ggpo/blob/master/doc/README.md))。我们保留其 **predict-reconcile + 误差平滑(修正不 snap)** 框架 = §5 的 tween;**回滚缓冲/预测器/重模拟不要**(为什么不需要,见 0017:数据源 append-only)。
- **Valve 插值延迟**:渲染在过去约 100ms,永远手握前后两个快照可插值([Source Networking](https://developer.valvesoftware.com/wiki/Source_Multiplayer_Networking))。本机制支持"滞后一个快照"以保证每次补间跨两个完整关键帧——节奏由上游(0017)给。

两条**通用前提**让机制极简:**数据源 append-only ⇒ 身份廉价且稳定**(§4.1);**纯 2D 文字 ⇒ 只几何 + alpha 需插值**(§4.2)。markdown 语法固定如何进一步消去解析侧重负,属 0017。

## 4. 数据结构(核心)

四层:身份 → 可插值几何 → 渲染节点 → 保留态场景。

### 4.1 身份 `NodeId`(稳定、与重排无关)

机制要求上游给每个字块一个**跨快照稳定的 id**(否则无法配对 past↔current)。本项目用 `(block_seq, glyph_idx)`——其稳定性来自数据源 append-only(论证见 0017 §6),机制本身只依赖"它稳定"。

```rust
/// 跨重排稳定的字块身份。打包成 u64 作 HashMap key(高 32 = block_seq,低 32 = glyph_idx)。
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(u64);
impl NodeId { pub fn new(block_seq: u32, glyph_idx: u32) -> Self { Self(((block_seq as u64) << 32) | glyph_idx as u64) } }
```

### 4.2 可插值几何 `Geom` vs 不插值载荷

**只有几何 + alpha 参与插值**;`uv/style/layer/kind` 是身份载荷,不插值(style 变 = 重新着色,默认 snap,留 policy)。

```rust
#[derive(Clone, Copy)]
pub struct Geom { pub pos: [f32; 2], pub size: [f32; 2], pub alpha: f32 }
```

### 4.3 渲染节点 `RenderNode`

```rust
pub enum Phase { Enter, Update, Exit }

pub struct RenderNode {
    pub uv: [f32; 4], pub style: u32, pub layer: u32, pub kind: u32, // 不插值身份载荷
    pub current: Geom,        // 目标态(最近一次提交)
    pub past: Option<Geom>,   // 过去态;None = 静止 → 走零成本单态路径
    pub t_start: f32,         // 过渡起点 ms;past=Some 时有效
    pub phase: Phase,
}
```

`past == None` 是常态(冻结/已 settle);`past == Some` 仅在过渡窗口内,settle 即塌回 None。

### 4.4 保留态场景 `Scene` + join

```rust
pub struct Scene { nodes: HashMap<NodeId, RenderNode>, dur_ms: f32 /* policy */ }

impl Scene {
    /// 提交一份活跃区布局快照:join、标注生灭与过渡。now = 当前帧 ms。
    pub fn commit(&mut self, layout: &[(NodeId, Geom, Sample)], now: f32) {
        let mut seen = HashSet::new();
        for (id, geom, s) in layout {
            seen.insert(*id);
            match self.nodes.get_mut(id) {
                Some(n) if !geom_eq(n.current, *geom) => {
                    // 关键:past 取「当前真实显示态(可能插值中)」→ 过渡可被打断而不回跳。
                    n.past = Some(n.displayed(now, self.dur_ms));
                    n.current = *geom; n.t_start = now; n.phase = Phase::Update;
                }
                Some(_) => {}
                None => { self.nodes.insert(*id, RenderNode::entering(*geom, s, now)); }
            }
        }
        for (id, n) in self.nodes.iter_mut() { // 不在新快照里 = exit:淡出,t=1 后删
            if !seen.contains(id) && !matches!(n.phase, Phase::Exit) {
                n.past = Some(n.displayed(now, self.dur_ms));
                n.current = Geom { alpha: 0.0, ..n.current }; n.t_start = now; n.phase = Phase::Exit;
            }
        }
    }

    pub fn instances(&mut self, now: f32) -> Vec<GpuInstance> { /* 见 §5;塌缩 settle、清除完成 exit */ }
}
```

**打断处理(关键)**:过渡未完又来新提交时,新 `past` = 节点**此刻插值显示态** `displayed()`,保证链式过渡平滑续接、永不回跳——"不许 snap"在 join 层的兑现点。

### 4.5 GPU 编码(两条实现路,本 ADR 不锁)

- **路 A — GPU 双态**:`GpuInstance` 扩 `pos0/size0` + `t_start`,着色器 `mix(past,current,ease(t))`。几何载荷约翻倍,几千字块同动不掉帧。
- **路 B — CPU 每帧 mix**:算好插值后上传单态,不改 shader,代价是过渡期每帧重传活跃区。
- 建议:**v1 走 B**(零 shader 改动、活跃区小、先跑通),热点后升 A。

### 4.6 与 content→layout 契约的接合

身份在 content/layout 边界按结构位置赋:layout 输出每字块带 `glyph_idx`,`block_seq` 由 app 给。扁平数组保持扁平,只多一个稳定序号 → **不破 AR10、不扩成富树**(同 0014 sidecar 精神)。`Scene` 活在 render sink(`GpuSink`)一侧,join 在那发生;core 仍只产"当前布局",不感知动画。

## 5. 插值契约 `f(past, current, t)`

```
t = clamp((now - t_start) / dur_ms, 0, 1);  e = ease(t)   // ease/dur 是 policy,不入结构
displayed.{pos,size,alpha} = lerp(past.*, current.*, e)
t >= 1:past = None(塌回静止);phase=Exit 则删除节点
```

**任意变换 = 填两端点**:translate→`pos` 异;scale→`size` 异;fade→`alpha` 异;组合→多字段同填。管线只认"两端点 + t",不知道是滑动还是缩放——机制/策略的切面。

## 6. 与块冻结 / infinite session

- **退化零成本**:冻结块所有节点 `past=None` → 单态发射 = 今天行为。
- **内存有界**:`Scene.nodes` 只装活跃区;冻结离屏历史块**不进 Scene**,按需从冻结布局发静态单态。故 `Scene` 大小 ∝ 活跃区,与会话长度无关。
- **冻结时机**:活跃区闭合后等过渡 settle(节点全 `past=None`)再冻结,冻结存动画终态 → 冻结瞬间无视觉变化。

## 7. 上游契约(本机制对驱动层的要求)

机制是被动的;它要求驱动层(对 markdown 即 [0017])提供:

1. **带稳定 id 的活跃区布局快照**:每次提交 = 活跃区全部字块的 `(NodeId, Geom, Sample)`,id 跨快照稳定。
2. **committed / active 区分**:哪些块已提交(冻结、不再进 Scene)、哪个是活跃区。
3. **提交节奏(cadence)**:何时产生一份新快照(可带插值延迟)。

机制据此 join + 插值 + 退化,**不关心快照从何而来、为何变化**。

## 8. 决策与不决定的

**采纳**:双关键帧 `RenderNode{current, past:Option, t_start, phase}` + retained keyed `Scene`(join,past 取显示态防回跳)+ `f(past,current,t)` 线性插值;退化单态零成本;Scene 只装活跃区;GPU 编码 v1 走 CPU mix。

**理由**:past→current 归约为"两端点 + 归一化 t",覆盖 translate/scale/fade/生灭全部组合;append-only ⇒ 身份稳定、2D 文字 ⇒ 只插值几何+alpha,使机制极简且与块冻结/infinite session/AR10 相容。动画观感隔离在 policy 层,随时换不动管线。

**不决定的(留 policy / 后续)**:具体 `ease`/`dur_ms`/各类变化动 pos·size·fade(见 [TODO V/T]);style 存活期变化的过渡(默认 snap);GPU 双态编码(路 A)落地时机;**驱动层(markdown 何时产生端点)= [0017]**。

## 9. 落地清单(机制层)— 已落地(Plan 5A,2026-06-15)

- [x] `render/morph.rs` 引入 `Scene`(retained `HashMap<NodeId,RenderNode>`)+ `commit` join + `instances` 插值(路 B,§4.4/§5);`GpuSink` 持有 Scene、逐帧 commit→instances。
- [x] 静止/冻结旁路(`past=None` → 单态);过渡完成塌缩(`instances` 内);exit 节点清除(v1 立即移除,**淡出留后续**)。
- [x] `FrameGlyph` 带 `block_seq`/`glyph_idx`,`build_frame` 由(views 下标,placed 下标)赋 → `NodeId`(契约最小扩展,§4.6)。
- [x] 测试(7 个,`morph.rs`):同 id 几何变 → past 记旧态;过渡中再 commit → past 取显示态不回跳;过渡 dur 后塌缩;exit 清除;静止零保留;cubic-out 端点。
- [ ] (留尾)块冻结 settle 后**移出 Scene**的内存优化(§6;现 Scene 装全部可见字,已按视口有界,非会话长度);**exit 淡出**;**GPU 双态编码(路 A)**;policy 层(ease/dur 大表)接入(§8)。

> v1 范围:**几何(pos/size)补间**;alpha 入场仍走 `spawn_time` 着色器路径(不改 shader)。

---

参考先例:[GGPO](https://github.com/pond3r/ggpo/blob/master/doc/README.md) / [Rollback netcode (SnapNet)](https://www.snapnet.dev/blog/netcode-architectures-part-2-rollback/)(预测-和解-误差平滑) · [Source Multiplayer Networking](https://developer.valvesoftware.com/wiki/Source_Multiplayer_Networking) / [Interpolation](https://developer.valvesoftware.com/wiki/Interpolation)(插值延迟)。驱动层见 [0017](0017-markdown-streaming-landing.md)。
