# 决策记录 0020:内容节点身份模型(scene-graph-lite)—— 嵌套区间 + parent 下标 + 路径哈希

- 日期:2026-06-16
- 状态:已采纳(数据结构定调;落地分相位,见 §9)
- 前置:0001 §2.2(content→layout→render 契约 / AR10 每帧一次跨界 / 扁平 `#[repr(C)]`)、0005(块冻结)、[0016](0016-streaming-morph-render-model.md)(`NodeId(block_seq,glyph_idx)` 稳定身份 + retained `Scene`)、[0017](0017-markdown-streaming-landing.md)(append-only / 提交前沿)、[0014](0014-table-two-pass-layout.md)(`TableRegion` sidecar = run 区间)、[0018](0018-sdf-panel-decoration-primitive.md)(装饰/效果 = 图元)、[0019](0019-reveal-gating-and-choreography.md)(reveal selector 需节点身份)
- 定位:给内容一层**节点身份**(像游戏引擎的 scene graph,但**不是指针树**)。今天身份只有 glyph 级扁平键 `(block_seq, glyph_idx)`,说不出"这是**表 / 第 2 行 / (1,2) 格 / 这段 bold run**"。0016 节点级 morph、0019 reveal selector、0014 sidecar 的一般化都要它。本篇定 **append-only 下取巧的扁平编码**,GPU 路不变。

---

## 1. 问题:缺一层"节点身份"

现管线全程扁平:`StyledSpan[]`(线性 run)→ `PlacedGlyph[]` → `FrameGlyph[]` → 0016 `Scene = HashMap<NodeId, RenderNode>`,身份 = `(block_seq, glyph_idx)`(两级扁平定位)。jcode 解析期那棵 `Document→Block` 树**当场拍平丢弃**(`content.rs::emit_doc`),下游无保留态树。

缺口:要表达 / 操作**结构节点**(表、行、cell、run)而非单字时,扁平 glyph 键不够:
- **0019 selector**:"先画 Grid、再填 Header 字、各 cell 并行" —— 选择子是节点(cell/row),不是 glyph 下标。
- **0016 节点级 morph**:让"一个 cell"或"一段 run"**整体**补间 / 入场,而非一堆 glyph 各动。
- **局部 restyle**:`**` 闭合 → 整段 run 变粗;表格某格变化 → 重算该节点。
- **0014 `TableRegion`** 已是"给某类块补结构身份"的雏形(cell = run 区间),但 ad-hoc、只服务表格。

## 2. 关键认知:不要指针树(引擎早就不用了)

游戏引擎/UI 的"组件树"几乎从不是指针链树 —— Tom Forsyth《Scene Graphs — just say no》+ data-oriented design:指针树 = cache 不友好 + 每节点堆分配 + 难序列化/上 GPU。父子关系普遍降级成**数组下标**。更别学 React/DOM 的"保留指针树 + 全量 diff":那是为**任意位置可变**兜底;我们 **append-only + 唯一活动块**(0017)已把可变区锁死在尾部,**不需要 diff**。

## 3. 取巧编码(append-only 让它几乎白嫖)

**A. 嵌套区间 / Euler-tour(主结构,最契合)**
append-only 且按文档序渲染 ⇒ **任何节点的后代在扁平 glyph 数组里必然连续**。故一个节点 = `[start, end)` 一个区间,**免子指针**:子树成员 = 区间包含;祖先 = 包住你的更大区间;"某 glyph 属于哪个 cell" = 区间判定。这正是 SQL 的 **nested-set model**(Celko),它擅长**读多、极少重构**的树 —— append-only 完美命中。`TableRegion`(cell=run 区间)是其特例。

**B. SoA + parent 下标(扁平节点表)**
节点拆数组,每个存 `parent: u32`,孩子隐式(文档序紧邻 / 或 children-range)。= glTF node 数组 / DOD 标准做法,cache 友好、可序列化、可上 GPU。A+B 合成一张表。

**C. 路径哈希稳定身份(Dear ImGui ID stack)**
跨帧稳定身份又不想保留整树:`id = hash(parent_id, local_seq)`。append-only 让 `local_seq` 天然稳定 ⇒ 免费拿到 React key 级稳定性却不留树。现 `(block_seq, glyph_idx)` 是其深度=1 的退化版。

**D. retained 冻结前缀 + immediate-mode 活动块(混合)**
冻结块 → 保留态扁平节点表,永不动;活动块 → 每 tick 重建(immediate mode),身份靠 C,故 0016 morph 仍能 join。= 0017 提交前沿落到节点层。

## 4. 数据结构

```rust
/// 内容节点类型(语义层级;leaf = Glyph)。
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum NodeKind { Doc, Paragraph, Heading, List, ListItem, Quote, CodeBlock,
                    Table, TableRow, TableCell, Run /* 同样式连续段 */, Glyph }

/// 一个内容节点。扁平表(SoA 亦可),按 `range.start`(= 文档序 = 追加序)有序。
pub struct Node {
    pub kind: NodeKind,
    pub parent: u32,            // 父节点下标(根 = 自身 / 哨兵)
    pub range: (u32, u32),      // [start,end) into 块内扁平 glyph 数组(嵌套区间,§3A)
    pub key: u64,               // 跨帧稳定身份(路径哈希,§3C)
}

/// 一个块(part)的节点表:append-only 维护,冻结后只读。
pub struct NodeTree { nodes: Vec<Node> /* 文档序 */ }
```

- **统一三处零散身份**:0016 `NodeId`(= `range` 长度 1 的 `Glyph` 叶)、0014 `TableRegion`(= 一组 `TableCell` 区间)、0019 `Selector`(= 按 `kind`/`range` 查询)全部塌进"`Node` = 区间 + parent + kind + key"。
- **查询**(selector / 命中测试 / 局部 restyle)= 在有序 `nodes` 上按 kind 过滤 + 区间二分,**无指针遍历**。
- **更新** = append 叶 + 重建活动块尾段子树(区间在末尾),前缀不碰,**无 diff**。

## 5. GPU 路不变(节点表 = CPU 索引)

- 仍吃**扁平 glyph/panel buffer**(守 AR10 / `#[repr(C)]` / 小包体)。`NodeTree` 是 **CPU 侧索引**,不进每帧热路径的跨界。
- **节点级效果不特殊化**:选中整行 / cell AO / run 发光 → 做成一个 [0018](0018-sdf-panel-decoration-primitive.md) **SDF 面板图元**(带自己 AABB),照常走图元路;shader **不认 node**。
- 0016 `Scene` 维持 `id → instance` 映射(身份与渲染槽解耦);节点表坐在 `Scene` **之上**,只在需要时把某 `range` 喂下去。

## 6. 与 tile 分桶正交(关键:两套层级,别混)

后续 GPU-driven **tile 分桶**([0018 §11](0018-sdf-panel-decoration-primitive.md) / onedraw §7 长期)与本节点树是**两套正交层级**,索引同一批图元的不同轴:

| | 内容节点树(本篇) | tile 分桶(后续) |
|---|---|---|
| 轴 | 文档序 + 语义 | 屏幕像素位置 |
| 来源 | 内容(layout 前/时) | 最终几何 AABB(layout 后) |
| 重建 | append-only 增量(前缀冻结) | 每帧/相机变即重建(本质 BVH) |
| 位置 | `Scene` 之上(CPU 身份) | 实例 buffer 之下(贴 GPU) |
| 用途 | 身份 / reveal / morph / 样式 | 单 draw / 降 overdraw / 排序 |

```
节点树(文档序, 稳定 key)          ← 逻辑
   ↓ Scene: id → instance(0016)
扁平实例 buffer(可被 tile 重排)    ← 渲染
   ↓ tile 分桶(空间, 每帧重建)
GPU 单 draw
```

**唯一纪律(守住即不冲突)**:**身份是 key(路径哈希 / `(block_seq,node_seq)`),不是"实例数组下标"**。tile 为 GPU 效率重排/重建实例数组时,只要身份 key 化、经 0016 `Scene` 的 `id→instance` 解耦,就**不碰身份/reveal/morph**。

要点:
- **append-only 只优化节点树,不优化 tile**(tile 是空间的,相机一动/一折行就重建,与 append-only 无关也不矛盾)。
- **别拿节点树当空间结构**:视口剔除走 `SpatialGrid` / 将来 tile;节点树是语义索引。两者混用 = 唯一的坑。
- **节点→tile 多对多**:一个 cell 折行后跨多行多 tile,靠 AABB 解,不靠区间(文档连续 ≠ 屏幕连续)。
- **WebGL2 兜底**(无 compute 不分 tile)下节点树照用(平台无关 CPU 结构),不挡兜底路。

**佐证**:OneDraw 本身**两套并存** —— hierarchical regions(逻辑)+ tile binning(空间);本篇即把它的 region 换成 append-only 的区间编码节点树(详见 [research/onedraw-analysis](../research/onedraw-analysis.md))。

## 7. 决策

采纳**内容节点身份模型 = 扁平 `Node` 表**(`{kind, parent, range:[start,end), key}`,文档序),用**嵌套区间(§3A)+ parent 下标(§3B)+ 路径哈希身份(§3C)+ 冻结前缀/immediate 活动块(§3D)**;统一 0016 `NodeId` / 0014 `TableRegion` / 0019 `Selector`;**GPU 仍吃扁平 buffer,节点表为 CPU 索引,节点效果走 0018 图元**;与 tile 分桶**正交**,靠"身份 key 化 + 0016 `id→instance` 解耦"互不干涉。

**理由**:append-only ⇒ 子树连续 ⇒ 树退化成"区间 + parent 下标"的扁平表,**免指针、免 diff、免重排**,且 cache 友好可上 GPU;路径哈希给跨帧稳定身份而不留重树;与 0016/0014/0019 现状是**收敛**(把已有的零散身份并进一张表),不是新增一套并行结构。

**不决定的(留后续)**:`key` 的具体哈希(FNV/xxhash 路径混合)与碰撞策略;`Run` 节点的切分粒度(是否每样式段一节点);节点表是否 SoA;activeblock 重建时旧子树的 exit 处理(交 0016 生灭);节点表是否部分上 GPU 做 range 级效果(默认 CPU,热点再说)。

## 8. 边界 / 非目标

- **不是 React vdom**:无保留指针树、无全量 diff/reconcile。
- **不做空间索引**:剔除/排序归 `SpatialGrid` / tile;本表纯语义。
- **不破 content→layout→render 契约**:扁平 glyph 流不变,节点表旁挂(同 0014 sidecar 精神,只是一般化)。
- 可编辑/乱序内容的 diff 匹配(Myers 等)超出范围(append-only 前提,同 0017 §6)。

## 9. 落地清单(分相位)

- [ ] `NodeKind` + `Node` + `NodeTree`(core;append-only 构建,从 jcode `Document` 拍平时**顺带记区间**而非丢弃)。
- [ ] `key` 路径哈希(§3C);`(block_seq, glyph_idx)` 表达为 `Glyph` 叶的退化 `key`(0016 兼容)。
- [ ] 把 `TableRegion`(0014)迁成 `Table/TableRow/TableCell` 节点(0014 sidecar = 本表特例)。
- [ ] 0019 `Selector` 落到节点查询(kind 过滤 + 区间);reveal 调度按节点产 0016 端点。
- [ ] 0016 节点级 morph:按 `range` 整体补间/入场(非逐 glyph);活动块重建走 §3D。
- [ ] (正交校验)上 tile 分桶时:确认身份 key 化、`Scene` `id→instance` 解耦、节点效果走 0018 图元(§6 纪律)。

---

参考先例:Tom Forsyth《[Scene Graphs — just say no](https://www.sea-of-memes.com/LetsCode10/LetsCode10.html)》/ data-oriented design(弃指针树)· Celko **nested-set model**(区间编码树,§3A)· **Dear ImGui** ID stack(路径哈希身份,§3C)· **ECS archetype**(Bevy/flecs/Unity DOTS:逻辑实体 + 扁平组件数组)· Karras **LBVH + Morton**(层级 = 线性数组 + 排序)· **glTF** node 数组(parent 下标,§3B)· **OneDraw** hierarchical regions + tile binning(两套层级并存,§6,见 [onedraw-analysis](../research/onedraw-analysis.md))。下游:reveal [0019](0019-reveal-gating-and-choreography.md)、morph [0016](0016-streaming-morph-render-model.md)、效果图元 [0018](0018-sdf-panel-decoration-primitive.md)。
