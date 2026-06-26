# 决策记录 0029:会话虚拟化 · 字形工作集 · 屏外块释放与重建

- 日期:2026-06-23
- 状态:**提议中(草案,待评审)**
- 前置:[0002](0002-event-driven-pipeline.md)(管线 / 块冻结)、[0003](0003-fault-tolerance.md)(catch-up / resync = **重新水化通道**)、0005(settle)、[0016](0016-streaming-morph-render-model.md)(几何端点)、[0019](0019-reveal-gating-and-choreography.md)(gate 全开=可瞬显重建)、[0020](0020-content-node-identity-model.md)(节点身份=重建后仍同一身份)、`README`(北极星:内存只与可见一屏成正比)、[architecture.md §8](../architecture.md)(早已计划"屏外块释放 instance 只留高度+文本")、[TODO2 §C](../../TODO2.md)(极致规模,本篇是其形式化前置)
- 定位:**兑现 README 那句"fps/内存只与可见的一屏成正比,与历史总量无关"中尚未做到的一半**。fps 这半已达成(`SpatialGrid` 视口裁剪 + atlas LRU + `settled` O(1) 跳过);内存这半**未做**——`Store` 全文、`PartView.cache` 全几何随历史**线性增长**,无释放。本篇把"屏外结算块的几何释放 + 可确定性重建"形式化为**一套分级工作集模型**,并指明重建/重新水化复用既有机制、不新建底座。

---

## 1. 触发:fps 有界,内存无界

招牌场景是 infinite session(100+ 轮 / 上万行 / 仍在流式增长)。当前三处随历史线性增长、无淘汰、无上限:

| 数据结构 | 保留内容 | 增长量级 | 现状 |
|---|---|---|---|
| `Store.parts[*].text` | 整条会话全文(每 part 一个 `String`) | O(总字符) | 真相源,永不释放 |
| `PartView.revealed` | 每 part 的源 grapheme 序列 | O(总源 grapheme) | 永不释放 |
| **`PartView.cache`(`BlockCache`)** | `placed`/`clusters`/`roles`/`strike`/`nodes`/`math`/`embeds`… 逐字几何 | **O(总 display 字形)** | **最大头**,屏外结算块也不释放 |
| `PartView.spawn` | 逐 display 字形 spawn_time | O(总 display 字形) | 结算后全 `Some`,信息退化为一个布尔 |
| `image_registry` / `code_scroll` | `(block_seq<<32)|idx` → 状态 | O(历史嵌入/代码块数) | append-only,不回收 |
| `atlas`(对照组) | 字形纹理瓦片 | **有界**(page-pool + LRU) | ✅ 已是工作集,不动 |

> **fps ≠ 内存**:`SpatialGrid` 让**绘制**只碰可见块,但上述结构都在 CPU 侧按**历史总量**驻留。atlas 已经证明"工作集化"对纹理可行;本篇把同一思路推广到 **CPU 侧的块几何与文本**。

## 2. 核心模型:per-view 四级工作集(单调可逆 + 滞回)

给每个 `PartView` 一个**工作集层级 `Tier`**,由"与视口的距离 + 是否 settled"驱动。层级越低,驻留越少;任意层级都能**确定性重建**回 `Hot`。

```rust
/// 块的工作集层级(0029)。越往下驻留越少;升级需确定性可逆(R8)。
enum Tier {
    Hot,        // 可见 / 近视口:全 BlockCache 驻留,参与裁剪/渲染/morph
    Warm,       // 已 settled + 屏外近:释放 placed 几何,留 height + nodes + revealed(便宜重建)
    Cold,       // 已 settled + 屏外远:再丢 nodes/cache,仅留 height + revealed(或 Store text 指针)
    FrozenFar,  // 极远:连 revealed/Store text 也可丢,仅留 height + part_id;重入靠 0003 重新水化
}
```

**驻留矩阵**(✅=驻留 / ⬇=释放):

| 字段 | Hot | Warm | Cold | FrozenFar | 重建源 |
|---|---|---|---|---|---|
| `height`(块高度) | ✅ | ✅ | ✅ | ✅ | 始终留(布局稳定不可丢,见 §3) |
| `cache.placed`/`clusters`/… | ✅ | ⬇ | ⬇ | ⬇ | `ensure_layouts(revealed, width)`(已确定性) |
| `cache.nodes`(0020 树) | ✅ | ✅ | ⬇ | ⬇ | 同上(随 cache 重排) |
| `revealed` | ✅ | ✅ | ✅ | ⬇ | `Store.parts[id].text` 重切 grapheme |
| `Store.parts[id].text` | ✅ | ✅ | ✅ | ⬇ | **server 快照重取(0003 resync/catch-up)** |
| `spawn` | ✅ | →布尔 | →布尔 | →布尔 | 结算块全 `Some` → 压成 `settled=true` |

## 3. 不变量(本篇的安全护栏)

1. **确定性可逆(R8)**:任一 `Tier → Hot` 的重建,产物**逐字节等于**首次排版结果。重建源链:`Store text → revealed → ensure_layouts → cache`;每段都已是纯函数(块冻结期 `cache` 本就是 `None→重排` 的缓存)。
2. **布局稳定:`height` 永不释放**。上方块堆叠只读 `height`,不读 `placed`。故释放屏外几何**不改变任何其它块的 y** → 滚动回看不跳变(0016 不参与:重入块按 `instant` 瞬显,无补间回滚)。
3. **身份不变(0020)**:重建后块/字仍是同一 `NodeId`(身份来自内容区间+路径哈希,与是否驻留正交)→ reveal/embed/morph 的下游引用不失效。
4. **只降级 settled 块**:`settled=false`(尾块在长)永远 `Hot`;锚底跟随时只在稳定后回收。
5. **退化等价**:全程 `Hot` = 现状,零行为差。虚拟化是**纯增量**,默认可关(`?novirt` 调试)。

## 4. 触发与滞回(防 thrash)

- 输入 = `SpatialGrid` 已有的"块 AABB vs 视口"距离;扩为**分级距离**(可见 → Hot;离视口 < D_warm → Warm;< D_cold → Cold;否则 FrozenFar)。
- **滞回带**:进 `Hot` 的阈值 < 退 `Warm` 的阈值(避免边界来回抖动反复重建)。具体数值 **Plan 18 实测定**。
- **节流**:回收器不必每帧扫全部 view;按 ~200ms 或滚动停顿时扫活跃集(与 0012 stats 拉取同节奏)。

## 5. 与现有机制接合(关键:重新水化不新建底座)

- **0003 容错链 = 重新水化通道**:`FrozenFar → Hot` 复用 `resync_from_snapshot` / catch-up——向 server 重取该段快照,**以 catch-up 模式瞬显**(0019 gate 全开、AR6 零淡入)。虚拟化的"换出再换入"因此**不引入新协议**,只是把容错的"刷新不丢历史"用在"内存换出"上。`Store` 的真相源语义随之放宽为"**可由 server 快照重取的缓存**",与 AR4 对账语义一致。
- **`settled`(0025 §4)**:已是降级前置;本篇在其上加"屏外距离"第二维。
- **0016 morph**:释放块本就不在活动 `Scene` 内(Scene 只装活动尾块),故释放/重建**与补间正交**,无回滚动画。
- **0019 reveal**:Cold/FrozenFar 必为已结算 → gate 全开 → 重建即 `instant` 上屏,不重播揭示。
- **`SpatialGrid` 每帧 O(总块) 重建**本身在极致规模下也要降为 O(活跃)——留 **quadtree 升级(TODO2 §C)** 承接,本篇只定工作集语义,不绑空间索引实现。

## 6. 决策与不决定的

**采纳**:per-view **四级工作集 `Tier`(Hot/Warm/Cold/FrozenFar)**,由"视口距离 + settled"驱动,**单调可逆 + 滞回**;**`height` 缓存独立于 `BlockCache` 且永不释放**(保布局稳定);**确定性重建链**(Store text → revealed → ensure_layouts);**FrozenFar 重入复用 0003 重新水化**(catch-up 瞬显);虚拟化为纯增量、可关。

**理由**:把 atlas 已验证的"工作集化"推广到 CPU 块几何;复用块冻结的"cache 可重排"性质 + 容错的"快照可重取"性质,使**释放安全、重建确定、重新水化零新机制**;`height` 不释放消解"释放→上方跳变"的根本风险。

**不决定的(留实现 / 后续)**:各 `Tier` 的距离阈值与滞回带数值(Plan 18 实测定);`SpatialGrid`→quadtree 升级(TODO2 §C);Worker 化下沉(TODO2 §C);`Store` 冷段是"丢弃纯重取" vs "落 IndexedDB 本地缓存"(v1 取**丢弃+重取**,最省);远景 LOD(文字简化占位,0011 留的 hook);多 part 跨块的批量重建调度。

## 7. 落地清单(与 [Plan 18](../plan/plan18-scale-memory-verification.md) 联动:**先测后做**)

- [ ] **先决:度量到位**(Plan 18 §2)——`FrameStats` 加 `store_chars / retained_views / retained_glyphs / retained_nodes`;JS 读 wasm 线性内存。**没有 before 曲线不动结构。**
- [ ] `PartView` 加 `tier: Tier` + `release_to(tier)` / `rehydrate()` 方法;`spawn` 结算后压成 `settled`。
- [ ] **`height` 提升为 view 级字段**(脱离 `BlockCache`):块堆叠/`SpatialGrid` AABB 只读 `height`;释放 `cache` 不动 `height`。
- [ ] `SpatialGrid` 输出**分级距离**(可见/Warm/Cold/FrozenFar)而非仅布尔可见。
- [ ] **滞回回收器**:节流扫活跃 view,按距离调 `tier`;调 `release_geometry`(Warm)/ 丢 `nodes`(Cold)/ 丢 `revealed`+`Store text`(FrozenFar)。
- [ ] **重建路径**:Cold→Hot 走 `ensure_layouts`(已确定性);FrozenFar→Hot 走 `resync_from_snapshot` catch-up 瞬显。
- [ ] `image_registry` / `code_scroll` 随块 `tier` 回收(块降到 Cold 即清其条目,重建时重登)。
- [ ] **回归**:Plan 18 长会话滚动来回 case——内存回落到 visible 基线、滚动无跳变、静止结算满帧。
- [ ] 调试:`?novirt` 关虚拟化(对照);debug 面板显示各 tier 块数 + 本帧重建次数。

---

参考:[architecture.md §8](../architecture.md)(屏外释放原计划)· [0003](0003-fault-tolerance.md)(catch-up/resync 即重新水化)· [0025 §4](0025-sdf-node-animation-system.md)(settled 冻结)· [TODO2 §C](../../TODO2.md)(虚拟化/LOD/Worker/quadtree 的上限层)· 业界:虚拟列表(react-window/TanStack Virtual)的"窗口外卸载 + 滚动回挂"、操作系统分页/工作集(working set)、纹理流送(texture streaming)的 mip/工作集思想。
