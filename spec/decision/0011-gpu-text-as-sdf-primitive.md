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
- 受影响接口缝:0001 §2.2;现管线 `crates/render/{atlas,scene}.rs`、`crates/render/src/shaders/glyph.wgsl`、`web/src/{pretext-bridge,glyph-raster}.ts`
- 大脑:`crates/core/src/{content,store,fsm,app}.rs`
- 相关 ADR:0004(markdown 管线)、0006(标签层 = 自定义语法落点)、0007(SDF 富特效 / 像素对齐)、0009(被本 ADR 演进)、0010(解析沿用 pulldown-cmark)
