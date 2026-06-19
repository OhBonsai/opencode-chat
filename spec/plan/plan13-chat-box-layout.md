# Plan 13 — chat 级盒子布局(Taffy)+ 用户/模型左右分栏 + web 调试输入框

- 日期:2026-06-19
- 前置:[0023 Taffy 盒子布局](../decision/0023-taffy-box-layout.md)(主)、[0020 内容节点身份](../decision/0020-content-node-identity-model.md)(树,Plan 7 已落 `nodes.rs`)、[0005 Turn 聚合](../decision/0005-turn-aggregation-and-settlement.md)(**角色/回合,硬约束源**)、[0000 §无限会话/反例](../decision/0000-overview.md)、[0016 形变](../decision/0016-streaming-morph-render-model.md)、[0021 JS-Rust 边界/样式](../decision/0021-js-rust-boundary-and-configurable-render.md)、[0001 §2.2 measureText 护城河](../decision/0001-canvas-architecture.md)
- 一句话:把 chat 内容从「build_frame 手搓块竖直堆叠」升级为 **Taffy 盒子树(over 0020 节点树)**,并在最外层按 **0005 角色**做**微信式左右分栏**——**user = 一个盒(右)**、**assistant 一个回合 = 一个容器盒(左)**(结构稳定;盒内画什么随 part 类型/文本语法分阶段长出,v1 先落 markdown/纯文本,见 §2.2);另在 `web/` 加一个**纯前端调试输入框**,直接 `POST /session/{id}/message` 和 opencode serve 实时对话(回包走现有 Rust SSE 渲染)。

> 范围决策(2026-06-19,与作者确认):盒子布局**直接上 Taffy**(非最小手搓);输入发送**web 直接 POST**(不经 wasm)。

---

## 0. 现状 & 缺口

- **布局**:`crates/core/src/app.rs::build_frame`(~1060)对每个 part(view/`block_seq`)`top += c.height + BLOCK_GAP` 竖直堆叠——**一列、手搓、无嵌套弹性、无左右、无角色感**。0023 已定 Taffy 方向但未落地(§10 清单)。
- **节点树**:0020/Plan 7 已落 `crates/core/src/nodes.rs`(`NodeTree`,扁平 `Vec<Node>` 嵌套区间 + parent),`app.rs::block_nodes()` 暴露,但**尚无消费者**——Taffy 是它的第一个真正消费者。
- **角色**:`store`/`fsm` 已有 role(user/assistant,见 app.rs snapshot 测试);**build_frame 当前不区分角色**,user 与 assistant 一样左对齐铺一列。
- **输入**:`web/src/transport.ts` 其实是**揭示时间轴播放器**(非 SSE);`web/src/input.ts` 是画布 pan/zoom。**真正的实时对话只能靠 `scripts/chat.mjs`(CLI)发消息**;浏览器里没有输入口。
- **缺口**:chat 级布局(角色气泡 + container)目前只散见于 [0000 §反例]「一个回答别切五个气泡」与 [Plan 10 §4]「气泡分组融合」,**无正主**。本 plan 落地它。

## 1. 终态数据流

```
opencode 事件 → FSM 收敛(0003/0005)→ 0020 节点树/turn 投影(append-only)
  → 角色分栏外壳(本 plan §2)+ Taffy 盒子布局(§3,over 节点树,measureText 作叶子 measure)
  → build_frame 读盒位发 FrameGlyph/FrameRect/FramePanel(收编手搓堆叠)
  → 0016 形变补间(reflow 不跳变)→ render
```

Taffy 给**目标态盒位**,0016 给**过渡**,二者正交(0023 §6)。

## 2. 角色分栏模型(0005 为准)

```
ChatContainer                 ← 最外 Taffy flex column(整条会话一个大盒;无限会话 = 往下加 Turn)
  └ Turn(0005 纯投影,不存储)
      ├ UserRow      align-self:flex-end(右)   → 一个 User 盒
      └ AssistantRow align-self:flex-start(左) → 一个 Assistant 容器盒(回合内 parts 作内部块)
```

### 2.1 结构不变量(硬,不可违反)
- **每个角色一个盒**:assistant **一个回合 = 一个容器盒**(回合内多 message / 多 part 作**盒内部块**堆叠,**绝不为气泡而气泡**——守 0005 拍平语义 + 0000 §反例);user 一条 = 一个盒。
- 角色来自 store(0005),**不新增存储**;turn 边界 = 下一条 user 锚点(0005 §2),重投影即恢复。
- 左右 = Taffy `align-self` + 盒 `max_width`(留边),**不是新图元、不是新管线**。

### 2.2 盒内内容模型(演进,**v1 = 现状切片,后续按类型/语法长出**)
盒子结构稳定,但**盒内"画什么"是分阶段扩展的,不固定为 markdown / 纯文本**:
- **assistant 容器盒内 = 异构 part 块**:reasoning / tool / text / …,**每种 part 类型有各自的渲染 + 动画处理**(tool 调用有它的展开/状态机动画,reasoning/text 有各自的揭示)——接 Part 状态机(0002)+ 0005 拍平 + 0019 揭示编排。**本 plan v1 先只落 markdown(text part)**;tool / 其余 part 类型**后续按类型各自接处理(单独排期)**。"一回合一盒"约束的是**不另起气泡**,**不是**让 part 失去各自的处理。
- **user 盒内 = 文本,但文本带特殊语法**:`@ref` / 图片 / 文件 / link / 技能… 是 text 的特殊语法,**后续按语法渲染不同内容**(接 P 标签层 0006 / O 嵌入 0007)。**本 plan v1 先纯文本**。

> 一句话:**「一角色一盒 + 左右分栏」= 稳定结构;「盒内渲染什么」随 part 类型 / 文本语法分阶段扩展,markdown 与纯文本只是 v1 切片。**

### 2.3 动画与 Taffy 的接缝(§2.2 各类 part 动画怎么和布局共处)

后续 tool / reasoning / text 各有各的渲染 + 动画,**与 Taffy 不在同一根轴上,本身不冲突**——架构把**几何 / 过渡 / 表现 / 状态**拆四层,动画落在后三层,Taffy 只守几何:

| 轴 | 谁负责 | 改布局? |
|---|---|---|
| **目标几何**(盒 rect) | **Taffy**(静态,每次算一遍,不认识时间) | 是(唯一定位者) |
| **盒位过渡**(上方增高顶下面、列变宽) | **0016**:Taffy 给目标 rect,0016 从旧→新补间 | 否(追 Taffy 的目标态) |
| **盒内表现**(逐字淡入、tool pop、scale-in、glow、reasoning 渐显) | **0025 anim + 0019 揭示节奏**:盒几何内做 alpha/scale/offset | 否(几何内变换) |
| **状态**(tool pending/running/done) | **Part FSM**(0002) | 否(驱动上面三层) |

**唯一真接缝 = "改尺寸的动画"**(tool 展开/折叠、reasoning 折叠、流式块增高)。纪律:

```
part 的 FSM/动画 → measure 报「目标态尺寸」 → Taffy 局部重排 → 下游盒位变 → 0016 平滑
```

- **measure 只报离散"目标态尺寸",时间维度交 0016/0025**;**不让 measure 逐帧吐插值高度**(否则动画时间逻辑漏进布局层,职责糊 → 那才会冲突)。例:tool 展开 = measure 报展开后最终高,0016 补间高度 0→full。
- **enter 动画要占位**:Taffy 按最终尺寸排,内容在盒内淡入(0019 骨架先行),避免真身到达跳变。
- **exit/折叠**:0016 exit 淡出(路 A)+ measure 趋 0,Taffy 收编后移除。

**已有样板**:**表格就是这套模式的验证**——`placeTable` 作叶子 measure、`PanelScene` 走 0016、reveal gating 控节奏(0014 + Plan 8/9)。tool / reasoning **照表格这条已验证的路复制**:各作一个 Taffy 叶子(measure 报目标尺寸)+ 各自的 PanelScene/glyph anim + 各自揭示编排,**不是新冲突,是样板复用**。

## 3. 现状边界(被收编/演进的四处,带 file:符号)

| # | 在哪 | 现状签名/行为 | 本 plan 怎么动 |
|---|---|---|---|
| ① JS 排版 | `web/src/layout-bridge.ts::layout` | `layout(runTexts:string[], runRoles:Uint32Array, maxWidth:number, tables?) -> {glyphs:[x,y,w,h]*N, blockHeight, tablePanels}`——**整 part 一次** measureText 折行 | 拆成 **measure(量尺寸)+ layout(定宽后摆位)** 两趟(§3.4);新增轻量 `measure` |
| ② Rust 缝 | `crates/core/src/seam.rs::LayoutEngine` | `fn layout(&mut self,&[StyledSpan],&[TableRegion],max_width)->LayoutResult{glyphs:Vec<PlacedGlyph>,block_height,table_panels}` | 加 `fn measure(&mut self,&[StyledSpan],avail_w)->MeasuredSize`(Taffy 叶子回调) |
| ③ wasm 桥 | `crates/wasm/src/lib.rs::LayoutBridge`(:683)持 `layout_fn:js_sys::Function` | 经 config `get_fn` 注入 | 加 `measure_fn`,impl `LayoutEngine::measure` |
| ④ 堆叠 | `crates/core/src/app.rs::build_frame`(~1060) | `views:Vec<PartView>` 扁平,逐 view `top += cache.height + BLOCK_GAP`;**无角色/嵌套/左右** | 整体替为 Taffy 树(§3.6) |

节点树 `crates/core/src/nodes.rs`(`NodeTree`:`Node{kind,parent,range,key}` + `build(block_seq,span_glyph,blocks)`)是 0020 已落、**尚无消费者**的地基——本 plan 是它第一个消费者。

## 4. 三层 Taffy 树(一棵树跨 chat 级 + 内容级)

```
Tier A(chat 级 · 新建 · core 从 views+roles 构)
  ChatRoot          Flex, dir=Column, gap=TURN_GAP, size=(viewport_w, auto)
   └ Turn[i]        Flex, dir=Column, gap=MSG_GAP            ← 0005 投影,不存储
       ├ UserBox    align_self=FlexEnd,  max_size.w=BUBBLE_MAX, padding=BUBBLE_PAD
       └ AsstBox    align_self=FlexStart, max_size.w=CONTENT_MAX
Tier B(part 内块堆叠 · 收编 top+=height)
           └ Block*  每 part NodeTree 顶层块 → Block 容器(dir=Column, gap=BLOCK_GAP)
Tier C(块内嵌套 · over nodes.rs 子树)
               └ List/ListItem/Quote/Table/Run …
                    容器节点 → Taffy 容器;Run/Table/Embed = **叶子(measure)**
```

- **一棵 Taffy 树**:Tier A 节点 core 现建;每个 `PartView` 的 `NodeTree` 映射成其 UserBox/AsstBox 下的子树(Tier B/C)。`Node.parent`(nodes.rs)直接给 Taffy 层级(0023 §2)。
- **叶子 = measure 源**(守 measureText 护城河 0001/0021):文本 Run → measureText 回调;Table → `placeTable`(0014,降级为叶子 measure);Embed → `reportSize`(0022,本 plan 仅占位)。

### 4.1 NodeKind → `taffy::Style` 映射(数据驱动,0021)

新增 chat 级 kind(core 内部枚举,不入 nodes.rs 的 `NodeKind`,避免污染内容树):`ChatRoot/Turn/UserBox/AsstBox/Block`。映射表(值从 0021 的 `BoxStyle` 数据取,**不写死**,同 `TableStyle` 实时 setter 先例):

| 节点 | display | 方向/对齐 | 关键约束 |
|---|---|---|---|
| ChatRoot | Flex | Column, gap=TURN_GAP | size.w = viewport |
| Turn | Flex | Column, gap=MSG_GAP | — |
| **UserBox** | Flex | **align_self=FlexEnd** | **max_size.w=BUBBLE_MAX**, padding |
| **AsstBox** | Flex | **align_self=FlexStart** | max_size.w=CONTENT_MAX |
| Block/Paragraph/Heading/CodeBlock/Quote | Block | Column | margin/gap |
| List | Block | Column | padding_left = INDENT×depth |
| ListItem | Flex | Row(marker + body) | — |
| Table | Block | (叶子外层) | 内部走 placeTable measure |
| Run / Glyph | **Leaf** | — | measure = measureText |

> 左右分栏的全部"魔法" = **UserBox `align_self:FlexEnd` + `max_size.w`**;assistant 同理 FlexStart。**无新图元、无新管线**(0023 §1)。

### 4.2 measure 边界演进(整 part → per-leaf 回调,两趟)

Taffy `compute_layout_with_measure` 对每个叶子调 `measure(known, available) -> Size`。现 `layout()` 是"整 part 一次",故拆两趟:

```
measure 趟:Taffy 排版期对文本叶子调 LayoutEngine::measure(spans, avail_w) → (w,h)   ← 廉价、只量
   ↓ Taffy 定下每叶子最终宽
layout 趟:对文本叶子调现有 layout(runTexts,runRoles, leaf_w) → glyph 相对位 + 行高  ← 仅最终宽一次
```

- **新签名**(seam.rs):`fn measure(&mut self, spans:&[StyledSpan], avail_w:f32) -> MeasuredSize{ w:f32, h:f32 }`。
- **JS 侧**(layout-bridge.ts):`measure(runTexts, runRoles, availW) -> [w,h]`,内部 `ctx.measureText` 折行算高;`Map<hash(runTexts+roles)+'@'+availW, [w,h]>` 缓存(measureText 微秒级,命中后近零)。`layout()` 同加缓存。
- **wasm**(lib.rs):config 增 `measure` 函数(同 `layout`/`rasterize` 经 `get_fn`),`LayoutBridge` 加 `measure_fn`。
- **增量**:append-only(0017 提交前沿)⇒ 仅活动尾部 part/块脏 → measure 只对活动叶子跑;settled view 冻结复用(0005 / app.rs `PartView.settled`)。

### 4.3 角色数据通路(core,接 0005)

```rust
// 新:crates/core/src/store.rs 或 fsm.rs
pub enum Role { User, Assistant }              // "user" → User;其余 → Assistant

// Store(store.rs):现有 partID→messageID→sessionID(:93 part_session)旁加一条
message_role: HashMap<String, Role>,           // apply_snapshot/apply_part_updated 写(protocol 已带 role)
pub fn part_role(&self, part_id:&str) -> Role   // partID→messageID→role,同 part_session 路数

// PartView(app.rs:430)加字段
role: Role,                                     // view_mut(~1337)创建时 store.part_role 填
```

**Turn 分组(0005 §2,纯投影、不存储)**:build_frame 先扫 `views`(到达序),`User` part 开新 Turn,后续连续 `Assistant` part(跨 message/part)归当前 Turn 的 **同一个 AsstBox**(守 §2.1「一回合一盒」)→ 产 `Vec<TurnGroup{ user:Option<usize>, assistant:Vec<usize> }>`(view 下标),喂 §4 构树。重投影即恢复,无新存储。

### 4.4 build_frame 重写(收编 `top+=height`)

```rust
fn build_frame(&mut self) -> FrameData {
    let turns = group_turns(&self.views);                      // §4.3 角色分组
    let mut bl = boxlayout::ChatTree::new(&self.box_style);     // 新模块 crates/core/src/boxlayout.rs
    let root = bl.build(&turns, &self.views, self.max_width);   // Tier A 现建 + 各 view NodeTree 映射子树
    bl.compute(root, self.max_width,                            // taffy compute_layout_with_measure
               |spans, avail_w| self.layout.measure(spans, avail_w));
    for (vi, view) in self.views.iter().enumerate() {
        let origin = bl.abs_origin(view.box_node);             // taffy Layout.location 相对父 → DFS 累加绝对
        // view.cache.placed 各 glyph 相对位【不变】+ origin → FrameGlyph(身份 (block_seq,glyph_idx) 稳定)
        // view.cache.table_panels + origin → FramePanel
    }
    // reflow:origin 较上帧变 → 喂 Scene(字)/PanelScene(框)补间(0016,build_frame 已有 join 先例 ~355)
}
```

- **关键**:view 内 glyph **相对位不变**(仍现 layout 摆),Taffy 只决定**盒 origin**,整体平移 → morph 身份稳定,0016 只补间平移量(§2.3)。
- **绝对偏移**:taffy `Layout.location` 是相对父坐标 → 需一趟 DFS 累加得 world `abs_origin`(`ChatTree` 内缓存)。

### 4.5 盒子与相机:不同坐标空间(不冲突,守 4 道接缝)

Taffy 与相机**正交**(同 §2.3 的几何 vs 动画,这次另一根轴是"视图"):

- **Taffy = 文档/世界空间布局**:盒子摆在哪(world px,在 `max_width` 文档宽里排)。
- **相机(`camera.rs::Camera2D`)= 世界→屏幕视图变换**:你从哪看(pan/zoom → shader `Globals.cam_pan/cam_zoom`)。

今天就是这么跑的:`build_frame` 产世界坐标,相机盖在上面。Taffy 只换"产世界坐标"那步引擎,**相机层不动**,故结构零冲突。要守的 4 道接缝:

1. **缩放 ≠ 重排**:zoom = 相机 scale,**不触发 layout**;窗口 resize 才走 `set_max_width`(重排 + `camera.set_viewport`)。"放大看更大"(字不重折行)与"窗口变窄重折行"是两根轴,代码已分(app.rs `set_max_width` ~722 重排;`zoom_at` 不动 layout)。
2. **左右对齐锚文档宽,不是屏幕宽**:UserBox `align_self:FlexEnd` 对齐 ChatRoot 宽(world `max_width`),相机再整体 pan/zoom。chat 列即文档 → 符合预期;**不期望"永远贴屏幕右"**(pan.x≠0 / zoom≠1 时贴的是文档右沿)。
3. **★ 锚底读 Taffy 的 computed bottom**(收编唯一必接对的点):现 `stick_to_bottom`+`pan_vel_y`(app.rs)跟随"revealed bottom",收编时把该底从 `top+=height` 改读 **ChatRoot/末盒 computed bottom**,相机跟随逻辑不变。
4. **无限会话**:Taffy 只重排活动尾部(settled view 冻结),相机 pan over 冻结世界坐标;world y 无限增长的 f32 精度是**既有问题(非 Taffy 引入)**,留虚拟化(TODO2 C)。

未来 3D 相机(0024)仍正交:Taffy 产 2D 世界 rect,3D 相机加 `model × view_proj`(0024 §4A),Taffy 不认识相机维度。**一句话:Taffy 写世界坐标,相机写视图矩阵,互不写对方字段。**

## 5. web 调试输入框(纯前端便利件,§3.8 详)

```ts
// web/src/chat-input.ts
export function mountChatInput(o: {
  serverUrl: string; sessionId: string;
  model: { providerID: string; modelID: string };   // 必带:无默认 provider 时空回(knowledge §4)
  parent: HTMLElement;
}): () => void {
  const ta = document.createElement("textarea");
  let inFlight = false;
  const send = async () => {
    const text = ta.value.trim();
    if (!text || inFlight) return;
    inFlight = true; ta.disabled = true;
    try {
      const r = await fetch(`${o.serverUrl}/session/${o.sessionId}/message`, {
        method: "POST", headers: { "content-type": "application/json" },
        body: JSON.stringify({ parts: [{ type: "text", text }], model: o.model }),
      });
      if (!r.ok) showError(`${r.status} ${await r.text()}`); else ta.value = "";
    } catch (e) { showError(String(e)); }
    finally { inFlight = false; ta.disabled = false; ta.focus(); }
  };
  ta.addEventListener("keydown", (e) => {
    if (e.key === "Enter" && !e.shiftKey) { e.preventDefault(); send(); }  // Shift+Enter 换行
  });
  o.parent.appendChild(ta); /* + 发送按钮 + error 区 */ return () => o.parent.removeChild(ta);
}
```

- `serverUrl`/`sessionId` 复用 `main.ts` 现有 config(传 `new ChatCanvas` 的同源);`model` 默认沿用 `scripts/chat.mjs`(`aliyuntokenplan/deepseek-v4-pro`),可经 config/env 覆盖。
- **回包零处理**:assistant SSE(delta/updated)由**现有 Rust transport(M1)** 接收并渲染——输入框只「把话发出去」,**不碰 wasm/core**。
- main.ts:`mountChatInput({ serverUrl, sessionId, model, parent: document.body })`(canvas 下方),`?session=` 已有则复用,否则先 `POST /session` 建。

## 6. 依赖 / 构建

- `crates/core/Cargo.toml` 加 `taffy = { version = "0.7", default-features = false, features = ["std", "flexbox"] }`——纯 Rust,**无 wasm-bindgen/web-sys/wgpu**(守 CR1 native 可测 + AGENTS §8);`grid` 暂不开(省体积,chat 内容用 flex+block 足够)。
- wasm 目标编译验证(同 Plan 12):`cargo build -p infinite-chat-core --target wasm32-unknown-unknown`。

## 7. 相位拆分(每相位独立可验;file:符号)

| 相位 | 交付(file:符号) | 验证 |
|---|---|---|
| **① 角色通路** | `enum Role` + `Store.message_role`/`part_role`(store.rs)+ `PartView.role`(app.rs:430)+ `group_turns`(app.rs §4.3) | cargo:user/assistant snapshot → turn 分组对;连续 assistant 多 part/message → **一组(一个 AsstBox)** |
| **② taffy + Tier A** | taffy dep(Cargo.toml §6)+ `crates/core/src/boxlayout.rs`(`ChatTree`:ChatRoot/Turn/UserBox/AsstBox);每 part = 一个叶子(measure = 现 layout `block_height`);build_frame 收编最外堆叠 | cargo:`UserBox.location.x` 右对齐(=root_w−box_w)、`AsstBox` 左 0、`max_size.w` 生效;**GPU 人工**:左右上屏 |
| **③ Tier B 块堆叠** | part 内 NodeTree 顶层块 → Block 容器(替 `BLOCK_GAP` 手搓);view glyph 加块 origin;**锚底改读 ChatRoot/末盒 computed bottom**(§4.5 接缝3) | cargo:块 y 位与现 `top+=height` **等价回归(±0)**;锚底 y == 末盒 bottom |
| **④ measure 回调 + Tier C** | `LayoutEngine::measure`(seam.rs)+ JS `measure`+cache(layout-bridge.ts)+ `measure_fn`(lib.rs);nodes.rs 子树→Taffy(list/quote 缩进、table 叶子 measure=placeTable) | cargo+tsc:折行/缩进/表格位置回归;缓存命中率基准 |
| **⑤ 0016 接合** | 盒 origin delta → `Scene`/`PanelScene` 补间(build_frame ~355 先例) | cargo:origin 变注入补间端点;**GPU 人工**:reflow/列变宽平滑不跳 |
| **⑥ web 输入框** | `chat-input.ts`(§5)+ main.ts 挂载 | tsc 绿;**人工**:起 opencode serve → 输入 → user 右 + assistant 左流式 |
| **⑦ 基准+卡口** | reflow 布局耗时 + measure 缓存命中(`?debug` perf 行) | 全卡口绿;基准入册 |

> **沙箱可验 vs 人工 GPU**(同 Plan 12):core(Taffy 树/measure/盒位/角色分组)+ tsc 沙箱跑测;**左右上屏、streaming 平滑、实时对话须人工 GPU/浏览器 + 本地 opencode serve**。

## 8. 测试用例提纲

- [ ] 正常:单 user + 单 assistant snapshot → `group_turns` 1 turn;`UserBox.x > AsstBox.x`(右/左);assistant 多 message/part → `assistant.len()>1` 但**一个 AsstBox 节点**(0005 硬约束)。
- [ ] 正常:嵌套列表(depth 1/2)→ Taffy `padding_left = INDENT×depth`;多级引用嵌套盒。
- [ ] 边界:空 assistant 回合(AsstBox 高=0,不占位跳变);超长 user 文本 → `max_size.w=BUBBLE_MAX` 折行、不铺满;表格流中列变宽 → origin delta 进 0016 不跳。
- [ ] 边界:append-only 增量——仅活动尾部 view 重 measure/重排,前缀盒 `abs_origin` 冻结复用(settled view 不进 compute)。
- [ ] 回归:Tier B 关掉角色后,块 y 位 == 现 `top+=height+BLOCK_GAP`(±0,证收编无几何漂移)。
- [ ] 错误:measure 缓存 miss 路径正确;Taffy 输出 NaN/Inf 防护(clamp);POST 失败(网络 / 4xx / 无 model 空回)输入框提示不崩。

## 9. Scope · 不做什么

- ❌ 不做全 CSS / RTL / 复杂脚本(0023 §7);文字测量**不 Rust 化**(0021 否,measure 走 JS 回调)。
- ❌ **不把 assistant 一回合拆成多气泡**(0005 拍平不动)——part **按类型各自渲染/动画**是方向(§2.2),但本 plan v1 只落 markdown(text part);tool/其余 part 类型后续单独排期。
- ❌ v1 user 盒只渲纯文本;`@ref`/图片/文件/link/技能等富语法后续(§2.2),本 plan 不含。
- ❌ Tier C 的 grid 布局、`align-content`/`justify` 全集——只用 flex(row/column)+block + `align_self` + `max_size`/`padding`。
- ❌ 输入框不做富文本/附件/历史回溯/快捷键完整——只够联调;**不进 Taffy、不动 wasm**。
- ❌ 头像 / 连续同发送者气泡融合(Plan 10 §4)、hover/选中——后续。

## 10. Risk / Open Questions

- **measure 两趟 + 回调频次**(0023 §9):measure 趟对活动叶子多次回调 JS;靠 `Map` 缓存(text+role+availW)+ 只重排活动 view + measureText 微秒级守。相位⑦量化命中率/布局耗时。
- **绝对偏移累加**:taffy `Layout.location` 相对父 → DFS 累加 `abs_origin`,O(节点);只对活动子树重算,前缀缓存。
- **wasm 包体**:+taffy(纯 Rust,中等);`grid` 不开省一截。基准记包体 delta。
- **契约扩张**(0023 §9):content→layout 从「扁平 run + sidecar」升到「节点树 + Taffy 样式 + measure 回调」,改动面大;Tier A→B→C 分相位降风险(A 独立可上屏,B/C 回归守等价)。
- **Open**:① tool/reasoning 各自渲染+动画(§2.2)的分相位——结构定「同盒不另气泡」,呈现单独排期;② user 富语法(0006/0007)单独排期;③ 角色分栏 + §2.3 动画接缝纪律是否回填 **ADR 0027**(现散在 0005/0000/本 plan,落地后建议上提)。

## 11. Done

`boxlayout::ChatTree` 收编 build_frame `top+=height`、Tier B 等价回归(±0);**user 右 / assistant 左,assistant 一回合一个 AsstBox(v1 内部落 markdown)**;`LayoutEngine::measure` 回调 + JS 缓存接通(护城河守住);盒 origin 经 0016 补间、reflow 不跳;web `chat-input.ts` 直 POST 可与本地 opencode serve 实时对话(回包走现有 SSE);卡口(cargo fmt/clippy/test native+wasm、`cargo build --target wasm32`、wasm-pack build、tsc)全绿;reflow 布局 + measure 缓存命中基准入册。

## 12. 关联

- decision:[0023](../decision/0023-taffy-box-layout.md)(主)/ [0020](../decision/0020-content-node-identity-model.md)(`nodes.rs` 树)/ [0005](../decision/0005-turn-aggregation-and-settlement.md)(角色/turn 硬约束)/ [0016](../decision/0016-streaming-morph-render-model.md)(盒位过渡)/ [0019](../decision/0019-reveal-gating-and-choreography.md)·[0025](../decision/0025-sdf-node-animation-system.md)(揭示/盒内动画,§2.3)/ [0002](../decision/0002-event-driven-pipeline.md)(Part FSM)/ [0014](../decision/0014-table-two-pass-layout.md)(measure+补间样板)/ [0021](../decision/0021-js-rust-boundary-and-configurable-render.md)(样式数据/measure 边界)/ [0001 §2.2](../decision/0001-canvas-architecture.md)(measureText 护城河);可上提 **ADR 0027**。
- Code 入口:`crates/core/src/app.rs::build_frame`(~1060,被收编)·`PartView`(:430,加 role)·`view_mut`(~1337)/ `crates/core/src/seam.rs::LayoutEngine`(加 measure)·`PlacedGlyph`/`TablePanel`/`LayoutResult`/ `crates/core/src/store.rs`(:93 part_session 旁加 part_role)/ `crates/core/src/nodes.rs`(`NodeTree`→Taffy)/ `crates/core/src/boxlayout.rs`(**新**,`ChatTree`)/ `crates/wasm/src/lib.rs`(:683 `LayoutBridge` 加 measure_fn)/ `web/src/layout-bridge.ts::layout`(拆 measure)/ `web/src/main.ts`(config + 挂 chat-input)/ `web/src/chat-input.ts`(**新**)/ `scripts/chat.mjs`(POST 范本)/ knowledge/opencode.md §4(发消息契约)。
