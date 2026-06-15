# infinite-chat —— 用游戏引擎的思路做 LLM 对话渲染

一个面向 LLM 对话的**高性能渲染引擎**:把大模型流式吐出的对话事件,渲染成一块流畅、
带丰富动效、可无限缩放平移的画布。用 Rust 写、编译成 WebAssembly、打包成 npm 包,
**React / Vue 可直接引入**。

## 立场:LLM 时代的 chat,值得被重做

**LLM 改变了一切。而人与 LLM 之间的主入口,就是 chat。** 不是 IDE 插件、不是一堆表单和按钮——是一条对话。**对话就是新的命令行、新的操作系统入口。**

可现在的 chat 界面,坦白说**基本都是垃圾**:逼你一个任务开一个新对话、长会话越用越卡、富内容(代码 / 表格 / 公式 / 图)渲染得又丑又糙、流式吐字像打嗝、动效廉价或干脆没有。**它们配不上背后那个正在改变世界的模型。**

所以我们重做一个——一条**永不结束、无比流畅、效果上限拉满**的对话,配得上 LLM 时代的主入口。这不是又一个聊天框,是把 chat 当成**第一公民界面**来做。

> 核心信念:做好一个实时流式 + 高动效 + 弱网可丢消息的 AI 对话界面,本质上是在做一个
> 游戏引擎——所以我们就用游戏引擎的手法做。详见 [spec/decision/0000-overview](spec/decision/0000-overview.md)。

---

## 核心场景:无限会话(infinite session)

**痛点**:现在的 AI 对话工具逼你"一个任务开一个新对话"——做完一件事就新建会话,历史散落各处;可真实使用里一个会话经常 **100+ 轮**还停不下来,而"为每个任务不停建新对话"的体验很糟。

**我要的**:像微信聊天那样,和某个"对象"(某个 agent / 项目)的**所有历史永远在一条会话里**,一直往下滚——**超长会话 / 无限会话(infinite session)**。这是本项目最强的应用场景。

**为什么这逼出一个游戏引擎**:一条 100+ 轮、上万行、还在流式增长的会话,用普通 DOM / react 渲染会**越用越卡**(DOM 节点爆炸、reflow、内存只涨不降)。所以才用游戏引擎手法——**GPU 实例化 + 块冻结(settled 不重排)+ 视口裁剪(只画可见)+ 无边画布**——让会话**无限长也始终丝滑**,fps/内存只与"可见的一屏"成正比,与历史总量无关。

> 一句话:**infinite-chat 的存在就是为了承载 infinite session**——其余的流式丝滑、SDF 文字、无边画布、极致规模(见 [TODO2](TODO2.md) C),本质都是为"一条永不结束的会话依然流畅好看"服务。

---

## 这是什么 / 不是什么

- **是**:一个对话**渲染引擎**(画布 + 文字 + 嵌入块 + 动效 + 流式/容错状态机),以
  可嵌入组件形态交付。前端框架只管外围控件(输入框、按钮、弹窗),对话画布交给本库。
- **不是**:一个 markdown→HTML 组件(那是 react-markdown / Streamdown 的活)。普通体量、
  纯 DOM 的聊天,用 DOM 方案更省;本引擎的价值在**无边画布 + GPU 动效 + 规模下不卡**
  (见 [0011](spec/decision/0011-gpu-text-as-sdf-primitive.md) 的适用边界)。

## 交付形态

```
Rust 核心(流式/markdown 大脑) ──编译──▶ WebAssembly ──打包──▶ npm 包 ──▶ React / Vue 直接 import
```

- 浏览器 / wasm 是**固定主战场**。原生(Tauri 等)不作为当前目标约束。
- 图形层只有 **wgpu** 一个抽象:instance 开 `BROWSER_WEBGPU | GL` → **WebGPU 优先、WebGL2 自动兜底(同一份代码,已启用,待专测)**;**Canvas2D 不做**,极端无 GPU 交 a11y 的 DOM 镜像兜底。注意 WebGL2 无 compute,逐字 compute 特效为 WebGPU 专属([0003 §5](spec/decision/0003-fault-tolerance.md)、[0011 §3.4](spec/decision/0011-gpu-text-as-sdf-primitive.md))。

---

## 设计原则

### 三条定调(产品形态)

1. **可嵌入组件**:wasm 库,React / Vue 直接引入;前端框架无关,只暴露画布 + 少量配置。
   → 因此**包体要轻、依赖要省**(这是否决重型文字栈/框架的根本原因)。
2. **2D SDF 世界 + 无边画布**:文字是 **SDF 图元**(非 DOM、非位图),与矩形/图片 quad
   共用相机、视口裁剪、实例化;任意缩放清晰,支持 GPU shader 特效。
3. **场景 = LLM 对话,FSM 驱动事件**:对话事件流驱动一切;状态机贯穿回合收尾、标签区域、
   嵌入块生命周期。

### 你容易忘、但同等重要的原则(已写在各 ADR 里)

4. **content→layout→render 三层契约,且 layout/render 可替换**([0001](spec/decision/0001-canvas-architecture.md) §2.2)。
   语义角色(`StyledSpan`)进、像素/坐标出;换解析器/排版/渲染后端只动各自内部,契约不动。
   **这条是本项目所有"能换方案而不伤筋动骨"的根本**。
5. **效果是数据,不是分支**([0002](spec/decision/0002-event-driven-pipeline.md) §5.1);
   **插件 = 注册表项,不是代码分支**,可热加载([0006](spec/decision/0006-inline-tags-and-extensibility.md) §7)。
6. **流式正确性是一等公民**:平滑器(蓄水池匀速吐字,做法同网游平滑远端玩家)、
   **块冻结**(settled 块不重算,只动尾块)、**remend**(尾部主动补全防半截语法闪烁)、
   **GPU `spawn_time` 淡入**(逐字动画零 CPU 参与)。这套"流式大脑"是 Rust 核心最不可替代的价值。
7. **FSM 驱动的不止"事件"**:回合收尾看门狗(soft/hard 超时,"忘了 idle"兜底,[0005](spec/decision/0005-turn-aggregation-and-settlement.md))、
   标签区域([0006](spec/decision/0006-inline-tags-and-extensibility.md) §5)、嵌入块 Placeholder→Loading→Ready→Failed([0004](spec/decision/0004-markdown-and-embeds.md) §7.3)。
8. **容错 / 对账 / 可恢复**:catch-up vs live 双模、`resync_from_snapshot`、EventSource 自动重连、
   幂等快照、确定性重放([0003](spec/decision/0003-fault-tolerance.md))。**刷新不丢历史、弱网不丢不错**。
9. **降级与无障碍**:WebGPU→WebGL2→Canvas2D 兜底;canvas 对屏幕阅读器是黑盒,
   **作为可嵌入组件必须配一层 DOM 镜像**(否则部分接入方不可用,常比性能更早成否决项)。
10. **安全**:模型输出、插件注入的标签**一律当数据**,绝不执行、不当真 HTML 解析;
    未知标签默认原样显示绝不静默吞掉([0006](spec/decision/0006-inline-tags-and-extensibility.md) §4/§6)。
11. **让浏览器干重活**:浏览器解码图片、Canvas2D 做文字整形(→TinySDF 生成 SDF)、
    SVG/mermaid 交浏览器光栅化;wasm 只持元数据。
12. **自有引擎,不被框架锁定**:**借算法不借框架**——用 TinySDF/ESDT(一个算法),
    否决 AntV G / egui / cosmic-text / glyphon([0011](spec/decision/0011-gpu-text-as-sdf-primitive.md) §2)。
13. **GPU-driven**:实例化 + 视口裁剪 + 块冻结 → **规模下每帧成本平**(对比 DOM 节点爆炸的根本动机)。
14. **工程严谨可测 + 运行时可观测**:测试期——确定性重放、proptest、naga shader 构建期校验、native 测试为铁律
    ([testing-and-benchmark](spec/testing-and-benchmark.md));运行时——`?debug` 节流帧统计(fps/帧耗时/发射 glyph vs 总量/atlas 占用·淘汰),
    因为性能退化多是**运行时、数据相关、GPU/主线程**的,测试期抓不到(见 [TODO 可观测性](TODO.md))。
15. **字体自带、放宽 BR5**:打包自选字体(`@font-face` 供 Canvas2D 光栅),接受字形非系统字体
    (即"追平 GitHub 结构/配色可,但字形一致与自带字体互斥",[0009](spec/decision/0009-text-rendering-engine.md)→[0011](spec/decision/0011-gpu-text-as-sdf-primitive.md))。

---

## 关键技术选型(各有 ADR)

| 关切 | 选择 | 出处 |
|---|---|---|
| markdown 解析 | **pulldown-cmark**(经 vendored jcode-render-core),不手写 nom、不上 comrak | [0010](spec/decision/0010-markdown-parsing-strategy.md) |
| 自定义语法(`<thinking>`/`:::`/`@`/角标) | 走**标签层 segmenter + 注册表**,不动解析器 | [0006](spec/decision/0006-inline-tags-and-extensibility.md) / [0010](spec/decision/0010-markdown-parsing-strategy.md) |
| 文字渲染 | **SDF 图元**(移植 TinySDF/ESDT),逐字 compute/vertex/fragment 三层 | [0011](spec/decision/0011-gpu-text-as-sdf-primitive.md) |
| 数据结构 | 命令日志→派生缓存、CPU 树/GPU 扁平网格双索引、定长瓦片 page-pool 图集、GPU-SDF/CPU-盒双表示 | [0011](spec/decision/0011-gpu-text-as-sdf-primitive.md) §3.3 |
| 嵌入块(图片/mermaid) | 降格为异步纹理块,浏览器光栅化,wasm 只持元数据 | [0004](spec/decision/0004-markdown-and-embeds.md) §7 / [0007](spec/decision/0007-rich-media-embeds.md) |
| 多实例同步 | 见 ADR | [0008](spec/decision/0008-multi-instance-sync.md) |

---

## 仓库结构

```
crates/
├── core/      # 流式/markdown 大脑:content(解析→StyledSpan)、store、fsm、app、frame —— 引擎无关、可测
├── render/    # WebGPU 渲染:atlas / scene / shaders(将从位图升级到 SDF,0011 退役清单)
└── wasm/      # wasm 绑定:ChatCanvas、transport、layout_bridge、glyph_bridge
web/           # 浏览器侧 harness + JS 桥(pretext-bridge / glyph-raster,0011 起让位给 SDF)
vendor/        # jcode-render-core(后端中立 markdown 文档模型)
spec/          # 设计文档:decision(ADR 0000–0011)、plan、research、architecture
```

## 现状

- **Plan 2(F–J)已完成**:快照/过滤、滚动/视口裁剪/块冻结、markdown 角色化、回合收尾、
  弱网容错;46 个 native 测试。详见并经审核:[spec/plan/plan2_progress](spec/plan/plan2_progress.md)。
- **Plan 3 方向**:画布化(SDF 文字图元 + 相机 + 空间索引)、input/选区/hit-test、
  嵌入块、富 shader 特效、DOM 镜像无障碍。

## 文档地图

设计决策按编号读:[0000 总览](spec/decision/0000-overview.md) ·
[0001 架构契约](spec/decision/0001-canvas-architecture.md) ·
[0002 事件管线/FSM](spec/decision/0002-event-driven-pipeline.md) ·
[0003 容错降级](spec/decision/0003-fault-tolerance.md) ·
[0004 markdown 管线](spec/decision/0004-markdown-and-embeds.md) ·
[0005 回合收尾](spec/decision/0005-turn-aggregation-and-settlement.md) ·
[0006 标签层/扩展](spec/decision/0006-inline-tags-and-extensibility.md) ·
[0007 富媒体嵌入](spec/decision/0007-rich-media-embeds.md) ·
[0008 多实例同步](spec/decision/0008-multi-instance-sync.md) ·
[0009 文字渲染引擎](spec/decision/0009-text-rendering-engine.md) ·
[0010 markdown 解析策略](spec/decision/0010-markdown-parsing-strategy.md) ·
[0011 SDF 文字图元](spec/decision/0011-gpu-text-as-sdf-primitive.md) ·
[0012 调试器 GUI:HTML vs egui](spec/decision/0012-debugger-gui-html-vs-egui.md) ·
[0013 数学(LaTeX)渲染策略](spec/decision/0013-math-latex-rendering.md) ·
[0014 表格两趟布局](spec/decision/0014-table-two-pass-layout.md) ·
[0015 字形源解析与回退(Bitmap/TinySDF/MSDF)](spec/decision/0015-glyph-source-fallback.md) ·
[0016 streaming 形变渲染机制(past→current 双关键帧)](spec/decision/0016-streaming-morph-render-model.md) ·
[0017 markdown 流式落地(提交前沿 + 保守预测/和解)](spec/decision/0017-markdown-streaming-landing.md) ·
[0018 SDF 装饰/面板图元(参数化 shader 框 + 共享 storage buffer)](spec/decision/0018-sdf-panel-decoration-primitive.md)
