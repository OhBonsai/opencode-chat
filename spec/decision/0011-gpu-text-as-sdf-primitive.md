# 决策记录 0011:游戏式自有 WebGPU 引擎,文字即 SDF 图元

- 日期:2026-06-14
- 状态:已采纳(方向定调;落地随 Plan 3 画布化推进)
- 前置:0001(架构 / §2.2 content→layout→render 契约)、0004(markdown 管线)、0007(富媒体嵌入 / 像素对齐)、0009(文字渲染引擎,本 ADR 演进其结论)、0010(markdown 解析)
- 来源:本轮关于"如何追平 markdown 观感 / 是否换文字引擎 / 是否上无边画布"的连续讨论

## 1. 背景与定调

经过本轮讨论,三个此前模糊的前提被钉死,它们共同改写了 0009:

1. **交付形态 = 浏览器内嵌组件**(给 Vue/React 用),**wasm/WebGPU 是固定主战场,不变**。原生/Tauri 读系统字体的路线对当前目标**作废**。
2. **接受打包字体**:用户要固定使用自己挑选的字体、不被改。故 0009 的核心约束 **BR5(零字体打包、用系统字体)放宽为"打包自带字体"**。
3. **实现方式 = "像做游戏一样"做 chat 画布**(注意:是方法论,不是要做游戏)。即 GPU 驱动、无边画布(相机平移缩放)、帧循环、实例化、视锥裁剪;**文字是场景里的一种图元,而非另起一套子系统**。

定调一句话:**保留自有 WebGPU 引擎,用游戏式手法实现 chat 画布;文字做成 SDF 图元,与矩形/图片 quad 同管线、同相机、同裁剪、同实例化。**

## 2. 被否决的替代方案(及理由)

| 方案 | 否决理由 |
|---|---|
| **cosmic-text / glyphon** | 真整形但:位图 atlas 在无级缩放下糊/或按缩放级重栅;**位图锁死 SDF 特效**;`spawn_time` GPU 淡入要降 CPU;包体重(rustybuzz+swash+字体)。浏览器里浏览器自己就是整形器,无需再背一套。 |
| **egui** | 即时模式与块冻结/帧缓存对冲;非排版引擎(epaint 文字弱);与主文字栈双渲染器→观感不一致;重依赖。 |
| **AntV G / infinite-canvas-tutorial 框架** | 是**可视化框架**,与"自有、可控、游戏式引擎"目标拧着,框架锁定 + 体积。**只取其算法,不取其框架**(见 §3)。 |
| **原生/Tauri 读系统字体** | 浏览器是固定主战场,此路对当前目标作废。 |

## 3. 决策

**自有 WebGPU 引擎,文字 = SDF 图元。** 具体:

1. **移植一个算法,而非框架**:把 **TinySDF / ESDT**(Mapbox 的"Canvas2D 逐字光栅 → 距离变换 → SDF tile";tutorial 仓库内 `tiny-sdf.ts`/`sdf-edt.ts`/`sdf-esdt.ts` 即此,自包含、MIT、数百行)移植进我们现有 atlas。**不引入任何渲染框架。**
2. **atlas 存 SDF tile**,glyph shader 改为读距离场(`smoothstep(边缘)`),由此:
   - **无级缩放清晰**(无边画布任意 zoom)——这是上 SDF 的硬需求,不是锦上添花;
   - **富特效**(发光/描边/溶解,0007)在 SDF shader 里加几行即可;
   - `spawn_time` GPU 淡入**保留**(自有 shader)。
3. **图元集专用、不过度通用**(因为只做 chat 画布,不是通用游戏):**文字 quad / 矩形(面板·代码块底·表格网格·圆角)quad / 图片 quad**;无边画布若做卡片连线再加 line/curve。三类图元共用相机 + 视锥裁剪 + 实例化。
4. **字体**:打包自带字体,经 `@font-face` 供 Canvas2D 逐字光栅(浏览器顺手做整形 + CJK + 回退);中文若要指定字体则再 `@font-face` 一个 CJK 字体并**懒加载**(别压首包)。
5. **Rust 核心保持为"流式-markdown 大脑"**:`content.rs`(parse→StyledSpan/块)、`store/fsm/app`(块冻结、remend、回合 FSM、对账、重放)不变,输出 **StyledSpan / 块增量** 驱动引擎把文字 run 当 quad 实例提交。

### 3.1 大脑 / 身体的语言边界(待定,倾向 A)

- **A(倾向):Rust-wasm 当大脑 + 自有引擎当身体**。Rust 输出 StyledSpan/块增量 → 薄适配 → 文字 quad。**保住已验证的流式逻辑(46 测、确定性重放、对账),边界只传尾块,开销可忽略。** 代价:跨 wasm↔JS。
- B:全 TS 重写流式逻辑。单语言更简单,**代价:丢掉 Rust 那套测过的正确性**。

### 3.2 逐字 GPU 动效能力(实例化的红利)

"文字即实例化图元"直接换来**逐字 compute / vertex / fragment 三层全开**(术语:Three.js 叫 InstancedMesh,我们在 wgpu 里是实例化绘制 + storage buffer 顶点拉取,控制更底层更自由)。每字一个实例,per-instance 数据(变换、atlas UV、颜色、spawn_time、自定义参数)放 storage buffer:

- **compute**:渲染前对实例缓冲跑一遍——物理 / 弹簧 / 布局动画 / 裁剪 / 排序,写回同一 buffer 供 VS 读(GPU-driven)。
- **vertex**:逐字/逐顶点形变——位移、缩放、旋转、倾斜、波动、沿曲线弯。
- **fragment(SDF)**:发光 / 描边 / 溶解 / 字重(0007),片元层免费。
- 三层可叠:compute 移动 → vertex 形变 → fragment SDF 上色。

约束与纪律(避免踩坑):

1. **图元用 quad,不用重 mesh**:每字 2 三角即可;需平滑形变(沿曲线弯 / 液态)再把 quad 细分成 N×N 小网格给 VS 顶点推。几千上万字无压力(同粒子系统)。
2. **基础布局是权威,动效是其上的 delta**:layout 出的位置为真值,抖动/形变仅表现层叠加。**hit-test / 选区 / 无障碍一律用 base 位置**,不用正在动的那个(呼应 §4 a11y)。
3. **与块冻结的张力**:**无状态、时间驱动**的效果(淡入、按 `time` 波动)写成纯 VS 函数——不重写 buffer,**兼容冻结**,最省;**有状态逐帧物理**才用 compute,且**先裁剪只算可见字**。不对全场每帧 compute。
4. **WebGPU 无 geometry / mesh shader**:单实例顶点拓扑固定,VS 不能凭空增减几何;**动态字数走 compute + indirect draw**。"任意"= 数学/数据任意 + indirect 覆盖动态数量,但每实例顶点预算是定的。

### 3.3 数据结构层(借鉴 Turitzin 动态 SDF 引擎)

参考 Mike Turitzin《I'm making a game engine based on dynamic SDFs》(2026-01)的数据结构层。他是 3D 动态体积世界,远比本项目复杂,但**核心结构降维后直接适用**。借鉴四条(标注:文字用 / 画布用 / 不搬):

1. **权威"命令日志" → 派生、可重生成的 GPU 缓存(架构,文字+画布)**:他场景 = 有序 SDF edit 列表(权威,CPU),brick atlas 只是按区域失效重算的**派生缓存**。我们对应:markdown 源 / 块模型为权威(`content.rs`/`store`),**glyph atlas + 布局 quad 皆派生缓存**。块冻结已是雏形 → 显式建模成"命令日志 → 派生缓存 + 脏区失效",为画布的对象编辑/撤销铺路。
2. **CPU 树 + GPU 扁平网格:两消费者两套索引(画布,关键)**:他**不用 octree**,GPU 侧用扁平指针网格(着色器 O(1) 取样、无树遍历),CPU 侧用 BVH(编辑/raycast/脏区)。**别让一个结构同时服务 CPU 编辑与 GPU 采样。** 画布升级时:**CPU quadtree/AABB 管对象**(视口裁剪、hit-test、脏区失效)+ **GPU 扁平 tile 网格/哈希格采样**,后者由前者重生成。这是从"单消息 1D 块冻结"扩到"画布多对象 2D 空间管理"最该补的结构。
3. **两级稀疏间接:稳定 key 表 → 定长瓦片进 page-pool 图集(文字)**:他 brick map(指针网格)+ 定长 8³ brick + 纹理池。我们 glyph atlas 应同形:**glyph-key → UV 槽 → 定长 SDF tile 进纹理页池,满了开页**;可变字号**分桶到几种固定 tile 尺寸**(shelf/slab),换 O(1) 分配、零碎片、LRU 极简。现 `atlas.rs` 单页 → 明确升级方向。
4. **双表示:GPU SDF(渲染)+ CPU 基础盒模型(命中/选区/无障碍)(文字+画布)**:他渲染用 SDF、物理另存 marching-cubes mesh,两套同源不互读。我们对应:**hit-test/选区/复制/a11y 走 CPU 端 glyph 盒 / 基础布局**,与 GPU 表现同源派生,**不回读 GPU、不用正在动的 SDF**(呼应 §3.2、§4 a11y)。

配套细节:**载荷紧凑**(R8 单通道 SDF tile、固定尺寸、多页、LRU);**索引表从一开始设计成 GPU 可读 storage buffer**(CPU/GPU 共享,免镜像);**LOD 当数据结构的降维版**(几档按 zoom 的 tile 尺寸 + 一个"占位框"档,不搬整套 clipmap)。

**不搬**:3D brick、完整几何 clipmap、**逐帧重算/脏-brick 增量机器**(我们字形静态、只追加,没有他"对既有几何任意编辑后快速重算"的最难问题——故对**文字**不搬,仅画布多对象用 ②③ 的空间索引)。

### 3.4 后端特效分级(WebGPU 全 / WebGL2 无 compute)

图形层只有 wgpu 一个抽象;instance 开 `Backends::BROWSER_WEBGPU | Backends::GL`,**WebGPU 优先、WebGL2(wgpu GL 后端)自动兜底,同一份代码**(0003 §5)。但两后端能力不同,特效要分级:

| 能力 | WebGPU | WebGL2(兜底) |
|---|---|---|
| SDF 文字(fragment 采样距离场) | ✅ | ✅ |
| `spawn_time` 淡入、发光/描边/溶解(纯 fragment/vertex) | ✅ | ✅ |
| **逐字 compute 动效(§3.2:物理/布局/排序)** | ✅ | ❌ **WebGL2 无 compute shader** |
| 动态字数 indirect draw | ✅ | ⚠️ 受限 |

落地纪律:**核心可读性(SDF 文字 + 时间驱动 VS 淡入)在两后端都保**;**compute 路做成可选增强**,WebGL2 下静默降级为 vertex+fragment,不报错、不缺内容。Canvas2D **不实现**;极端"无 WebGPU 也无 WebGL2"交给 a11y 的 DOM 镜像(§4)兜底。

## 4. 不变量与影响

- **不变量保持**:0001 §2.2 的 content→layout→render 契约(StyledSpan/角色、平铺位置)**不动**;`content.rs` 与解析器一行不改。换的是 layout 桥 + render 后端(atlas/scene/shader)。
- **退役清单**(无边画布用途下被取代):`crates/render` 的**位图** atlas / scene / `glyph.wgsl` 升级为 SDF;两个 JS 桥 `pretext-bridge.ts` / `glyph-raster.ts` 逐字位图路径让位给 SDF tile 生成。
- **演进 0009**:0009 当初为 BR5 选"系统字体位图桥",其唯一核心理由(BR5)已放宽 → **0009 的"保留位图桥"被本 ADR 取代**;0009 备案的 glyphon 升级路径也**作废**(改用自有 SDF)。
- **0007 升级**:SDF 由"富特效可选项"变为**承重项**(无级缩放必需);0007 的特效片元直接建在本 ADR 的 SDF shader 上。
- **a11y**:canvas 对屏幕阅读器是黑盒;作为给别人嵌入的组件,**需配一层可见内容的 DOM 镜像**(否则部分接入方不可用,常比性能更早成为否决项)。
- **LOD**:无边画布缩到很远时文字 sub-pixel → 渲染成占位矩形,**只对可读字号做光栅**,控 atlas 与开销。

## 5. 为什么这条路的性能站得住

SDF atlas + 实例化 quad 是**游戏渲染文字的标准做法**:海量字形、任意缩放、平移裁剪都是平的开销(TinySDF 在 Mapbox 渲染海量标签验证过)。无边画布语境下 **DOM/react-markdown 进不了画布**(无法把成千上万节点变换到可无级缩放画布),不在候选内。markdown 解析开销相对画布真正成本(裁剪/atlas/LOD/批处理)可忽略。

## 6. 重新评估触发条件

- 需要**极端 zoom 下笔画拐角锐利** → 普通 SDF(从位图)会圆角,改 **MSDF**(需矢量轮廓:`fdsm`/`ttf-parser` 在 wasm,或 cosmic-text)。
- 需要**跨浏览器逐像素一致**(浏览器整形器各家有差异) → 改用 wasm 内整形(cosmic-text)做布局,自有 MSDF 做光栅。
- **放弃浏览器、转原生(Tauri)** → 重开系统字体路线(0009 原生分支)。

## 7. 来源 / 链接

- TinySDF/ESDT:`infinite-canvas-tutorial/packages/core/src/utils/glyph/{tiny-sdf,sdf-edt,sdf-esdt}.ts`(注:源自 Mapbox tiny-sdf / use.gpu / acko.net subpixel distance transform)
- 数据结构层借鉴:Mike Turitzin《I'm making a game engine based on dynamic SDFs》(YouTube `il-TXbn5iMA`,2026-01;命令日志/brick map+atlas/BVH/clipmap/Jolt),其引用根技术与本 ADR 同源(Valve 2007 SDF、NVIDIA 2022 SDF grid、Losasso&Hoppe 2004 clipmap)
- 受影响接口缝:0001 §2.2;现管线 `crates/render/{atlas,scene}.rs`、`crates/render/src/shaders/glyph.wgsl`、`web/src/{pretext-bridge,glyph-raster}.ts`
- 大脑:`crates/core/src/{content,store,fsm,app}.rs`
- 相关 ADR:0004(markdown 管线)、0006(标签层 = 自定义语法落点)、0007(SDF 富特效 / 像素对齐)、0009(被本 ADR 演进)、0010(解析沿用 pulldown-cmark)
