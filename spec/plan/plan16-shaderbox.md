# Plan 16 — ShaderBox 落地:矩形 shader 画板 + 五护栏 + 度量

- 日期:2026-06-19
- 前置:[0028 ShaderBox](../decision/0028-shaderbox-primitive.md)(决策)、[0026 模块化 shader](../decision/0026-modular-shader-organization.md)、[0018 storage 数据通道](../decision/0018-sdf-panel-decoration-primitive.md)、[plan13 Taffy 叶子](./plan13-chat-box-layout.md)、[plan15 §2.7 copy→✓ morph](./plan15-code-block-viewport.md)、[0024 §5 平台 caps](../decision/0024-3d-camera-and-raymarch-sdf.md)
- 一句话:落地 0028——`ShaderBox` 图元(宽高 + 内置 WGSL shader + 背景 + time/resolution/params + 可选输入纹理),作内置程序化动效/特效底座;**性能五护栏 + FrameStats 度量随图元一起落**(不是事后补)。

> 作用域 v1:**仅引擎内置** shader(编译期已知集)。内容/markdown 开放任意 shader = defer(运行时编译 + 安全 + GLSL,另案)。

---

## 0. 现状

图元全固定功能:glyph/rect/panel(0018 写死)/widget(0026 固定小 fn)。无"任意 fragment + time/params"画板。0024 §4B raymarch 区域 quad 未抽象。

## 1. 数据流

```
core:ShaderBox 节点 → Taffy 叶子(measure=宽高)→ build_frame:
   · 离屏 → cull(护栏1)
   · dynamic=false(shader 不用 time)→ 画一次后冻结(护栏2)
   · dynamic=true → 按节流时钟(护栏4)发 FrameShaderBox{shader_id,params,bg,channels,time}
render:每 shader_id 一 pipeline → quad + 片元跑该 shader(globals + uniforms/storage)→ over 背景
FrameStats:活跃 box 数 + shader 像素/帧(护栏度量)
```

## 2. 关键机制

### 2.1 图元 + 契约
- `FrameShaderBox{ pos, size, shader_id:u32, params:[f32;K], bg:[f32;4], channels:Vec<u32>(纹理id), dynamic:bool }`。
- uniforms:`resolution=size`、`time`(引擎/节流时钟)、`params`(走 0018 storage,增量 update 不每帧重传)、`bg` + 可选 `bg_tex`、`channel0..N`。
- 输出:fragment `rgba` → **over 背景/下层**(alpha 混合)。

### 2.2 shader 集(编译期,0026 拼接)
- `shaders/shaderbox/common.wgsl`:公共 uniform 结构 + helper(uv、sd_* 借 `base/sdf.wgsl`)。
- `shaders/shaderbox/<effect>.wgsl`:每效果一个 `fn shade(uv, u) -> vec4`;shader-id → 对应 pipeline(`backend.rs` build 期注册,同 `with_sdf` 拼接 + wgsl 校验)。
- **每 shader-id 一 pipeline**(任意 fragment 不塞一个巨 switch,0028 §3)。

### 2.3 五护栏(强制,随图元落)
1. **cull**:box 世界 rect ∉ 视口 → 不发(spatial grid 已有,app.rs)。
2. **静态即冻**:shader 声明 `dynamic:false`(不用 time)→ 结果可冻结复用,不进每帧重发(同 settled)。
3. **面积/分辨率封顶**:`size` 超阈 → 渲到上限分辨率离屏纹理再放大(downscale);先留钩子,大 box 才启。
4. **降帧节流**:dynamic box 挂节流时钟(默认 30fps / 可配),与主 rAF 解耦(reveal 调度器自时钟同思路)。
5. **平台 caps**:WebGL2/移动端 → raymarch 类 shader 步数封顶 + 降精度(0024 §5);caps 探测降级。

### 2.4 度量
- `FrameStats` 加 `shaderbox_active:usize` + `shaderbox_pixels:u64`(Σ 屏上 box 像素);`?debug` 每秒一行与 fps 同打(现有节流帧统计)。

### 2.5 内置 animated icon 库(PixelSpiritDeck 整盘移植 + SDF 工具箱)

> **★ 许可决定(2026-06-20,作者拍板,改 §2.5 原案)**:本项目为**个人 / 非商用 wasm+webgpu test**,作者**接受版权风险**,**逐字移植** [PixelSpiritDeck](https://github.com/patriciogonzalezvivo/PixelSpiritDeck) 整盘 shader(SDF 工具箱 + 50 icon switch)作内置 animated icon 库。卡造型 LICENSE 原文仍是 "cannot use this Work in any commercial **or** non-commercial product"——**风险记录在册,本项目一旦转商用 / 公开分发必须先重核此条并很可能整库重画**(见 §6)。SDF helper 数学部分本身已并入 [LYGIA](https://lygia.xyz)(已 vendored `/lygia`,Prosperity/Patron),低风险;高风险在 50 张**卡造型组合**。

- **机制(编译期 WGSL,符合 0028)**:
  - 整盘 GLSL → **一个 `shaders/shaderbox/icons.wgsl`**:SDF 工具箱(§2.5a)+ `switch(icon_id)`(§2.5b,50+ 支)→ **一条 pipeline**、`params[0]=icon_id`(per-box **uniform 分支**,coherent、只走选中支,廉价、符合护栏)。源 GLSL 见 **附录 A** 作翻译基准。
  - **GLSL→WGSL 契约**:`iResolution→resolution`、`iTime→time`、`iMouse` defer(TODO Q,源里 hover 预览用,v1 不要)、`uv=fragCoord/resolution`、`#define S/C`→`fn`、三元/`switch`/`for` 照译、`mat2(...)` 列主序核对。单 ShaderBox = **一个** icon 满格(源里 10×5 网格仅作 contact-sheet 预览,产品按 icon_id 取单格)。
  - **AA**:源已用 `sstep`(smoothstep ±.005)抗锯齿 → 缩放锐利(护栏不冲突)。
- **静态 vs 动画(护栏 2)**:**50 中 46 个 animated**(`S=1+sin(t)/4`、`C=1+cos(t)/4` 呼吸,或 `iTime` 直驱旋转/morph)→ 默认 `dynamic=true` 挂节流时钟(30fps,§2.3 护栏4)。**4 个纯静态**(`void`/`the temple`/`the hermit`/`enlightenment`)→ `dynamic=false` 画一次即冻(护栏2)。每 icon 的 `dynamic` 标志由注册表(§2.5c)携带,build_frame 据此决定冻 / 发。
- **morph 钩子**:copy→✓ = `mix(icon_a, icon_b, t)`(Plan 10 §4 / plan15 §2.7);整盘任意两 icon 间可同法插值。

#### 2.5a SDF 工具箱(并入 `base/sdf.wgsl`,LYGIA 标准数学,WGSL 直译)

`sstep`(AA smoothstep)、`stroke`(04)、`circleSDF`(08)、`fill`(09)、`rectSDF`(10)、`crossSDF`(11)、`flip`(12)、`vesicaSDF`(14)、`triSDF`(16)、`rhombSDF`(17)、`rotate`(19)、`polySDF`(26)、`hexSDF`(27)、`starSDF`(28)、`raysSDF`(30)、`heartSDF`(34)、`bridge`(35)、`spiralSDF`(47)、`scale`、`flowerSDF`;宏 `S`/`C`(呼吸)。括号内为源卡编号(技法出处)。

#### 2.5b icon 目录(`icon_id` = 源 case 号;★=animated)

| id | 名 | 技法 | id | 名 | 技法 |
|---|---|---|---|---|---|
| 0 | void | 空(静态) | 25 | ripples ★ | rect×4 循环偏移 |
| 1 | justice ★ | 竖分割 sstep | 26 | the empress ★ | polySDF(5)+环旋 |
| 2 | strength ★ | 余弦边界 | 27 | bundle ★ | hexSDF×4 |
| 3 | death ★ | 对角分割 | 28 | the devil ★ | circle+starSDF(5) |
| 4 | wall ★ | stroke 呼吸 | 29 | the sun ★ | star(16)+8 三角光芒旋 |
| 5 | temperance ★ | 三横线波动 | 30 | the star ★ | rays(8)+starSDF(6)×2 |
| 6 | branch ★ | 斜 stroke | 31 | judgement ★ | rays(28)旋+rect |
| 7 | the hanged man ★ | 双对角 stroke | 32 | wheel of fortune ★ | polySDF(8)+rays 旋 |
| 8 | the high priestess ★ | 圆环呼吸 | 33 | vision ★ | vesica×2+rays(50)旋 |
| 9 | the moon ★ | 圆减偏移圆 | 34 | the lovers ★ | heartSDF+triSDF |
| 10 | the emperor ★ | rect 描边+填 | 35 | the magician ★ | 双圆 bridge |
| 11 | the hierophant ★ | rect+cross 条纹流动 | 36 | the link ★ | 双 rect bridge 旋 |
| 12 | the tower ★ | rect flip 对角 | 37 | holding together ★ | 双 rect bridge |
| 13 | merge ★ | 双圆 flip | 38 | the chariot ★ | rect+菱 bridge |
| 14 | hope ★ | vesicaSDF flip | 39 | the loop ★ | rect×5 bridge 链 |
| 15 | the temple | 三角减三角(静态) | 40 | turning point ★ | 双三角 bridge 旋 |
| 16 | the summit ★ | circle 描边+三角 | 41 | trinity ★ | 三三角 bridge |
| 17 | the diamond ★ | rhombSDF 描边×2 | 42 | the cauldron ★ | vesica×12 环 |
| 18 | the hermit | tri flip rhomb(静态) | 43 | the elders ★ | vesica×6 环 |
| 19 | intuition ★ | 三角比值旋 | 44 | the core ★ | star(8)+菱×8 |
| 20 | the stone ★ | rect 旋+十字切 | 45 | inner truth ★ | 极坐标菱形网格 |
| 21 | the mountain ★ | rect×3 叠 | 46 | the world ★ | flowerSDF(5)+star 旋 |
| 22 | the shadow ★ | rect 错位 | 47 | the fool ★ | spiralSDF 旋 |
| 23 | opposite ★ | rect flip 对置 | 48 | enlightenment | 全白(静态) |
| 24 | the oak ★ | rect 嵌套描边 | 49 | elements ★ | 三圆+三角+环 旋 |

#### 2.5c 注册表(Rust)

`enum IconId { Void=0, Justice=1, … Elements=49 }`(50 项,值=源 case 号)→ `params[0]`。每项携 `dynamic: bool`(46 ★ = true,4 静态 = false)。**功能别名**(聊天复用):`Copy/Check/Spinner/ChevronDown/Close/Plus…` 先映射到最贴近的盘内 icon(如 spinner→弧旋类、merge→link),缺的用工具箱在同一 switch 末尾追加自画支(id ≥ 50),不破整盘移植。

### 2.6 Agent 回复 logo(动画 glow-orb ShaderBox)

- **用途**:assistant 消息的**身份 logo / 头像**(plan13 AsstBox 头像位,原 Scope 标"后续",此处填)——一个**动画发光环/球**;流式("思考中")时加快脉冲作 busy 指示。
- **图元**:一个 **dynamic ShaderBox**(小盒 ~32–40px)。技法 = **noise 调制的发光环**(噪声驱动半径 + 角向高光 + 径向衰减),参考 shadertoy `4sc3z2`。
- **★ 许可**:该 shadertoy 默认 **CC BY-NC-SA 3.0(非商用)**;其中 `snoise3/hash33` simplex 噪声是公开通用实现(可借,LYGIA 亦有 `lygia/generative/snoise`)。→ **借技法、用 LYGIA 噪声、自写我们的 logo**(发光环是标准效果),**不 verbatim 抄**入产品(同 0024「shadertoy 仅借技术」)。要原样用须核该页许可/取授权。
- **数据驱动(0021)**:环色(`color1/2/3`)、`innerRadius`、`noiseScale`、脉冲速度走 `params`/Palette → 亮暗主题 + 品牌色可配,不写死。
- **性能**:小盒 dynamic = 永远活跃岛但**面积极小**(~40²px × 便宜片元)→ 按成本模型 ≈ 可忽略;护栏:离屏 cull(消息滚出即停)、降帧节流(脉冲 30fps 足)。N 条可见 assistant = N 个小岛仍小,离屏全停。静止态可降静态(护栏2),仅 streaming/hover dynamic。
- **落点**:plan13 AsstBox 加**头像位**(Taffy 固定小方 leaf)→ 内填该 ShaderBox。

## 3. 相位

| 相位 | 交付(file:符号) | 验证 |
|---|---|---|
| **① 图元 + 管线** | `FrameShaderBox`(frame.rs)+ `shaderbox` pipeline + `common.wgsl` + 一个示例 shader(SDF 圆/纯色) | 沙箱:wgsl 解析过;**GPU 人工**:一块画板出图 + params 改色 |
| **② 时钟 + 护栏 1/2/4** | core 驱动 time;离屏 cull;静态即冻;节流时钟(§2.3) | cargo:cull/冻判定/节流;`FrameStats` 计数;**GPU 人工**:离屏 0 耗、静态冻、节流 30fps |
| **③ 首批内置 shader** | loading 弧旋转 + **copy→✓ `mix` morph**(接 plan15)+ 一个装饰 | **GPU 人工**:动效、缩放锐利、接 0021 色 |
| **③′ 内置 icon 库(§2.5)** | SDF 工具箱 → `base/sdf.wgsl`;`icons.wgsl`(整盘 50 支 switch `icon_id`,附录 A 译);`IconId` 注册(50 项 + dynamic 标志);功能别名映射 | 沙箱:wgsl 解析(50 支全过);**GPU 人工**:contact-sheet 50 格上屏、46 呼吸/旋、4 静态冻、缩放锐利 |
| **③″ Agent logo(§2.6)** | dynamic ShaderBox glow-orb(**自写** + LYGIA 噪声)放 AsstBox 头像位;params 色/脉冲;streaming 加速 | **GPU 人工**:logo 在 assistant 旁、呼吸、离屏停、缩放锐利 |
| **④ 输入纹理 channel** | box 喂静态纹理(plan14)作 channel0,示例溶解/扫光 | tsc;**GPU 人工**:纹理输入特效 |
| **⑤ 收编 raymarch(留位)** | 0024 §4B raymarch 作一个 shader-id(`raymarch.wgsl`)+ 平台 caps(护栏5) | **GPU 人工**:小区域 raymarch;WebGL2 降级不崩 |
| **⑥ 度量 + 面积封顶 + 卡口** | `FrameStats` shaderbox 计数 + 护栏3 downscale 钩子;基准 | 全卡口绿;基准(活跃 box × 像素 vs fps)入册 |

> 沙箱可验:图元/cull/冻/节流/度量计数(cargo)+ wgsl 解析 + tsc。**出图、动效、raymarch、降级须人工 GPU。**

## 4. 测试用例提纲

- [ ] 正常:静态 ShaderBox(SDF 圆)→ `dynamic=false` → 画一次后不再每帧发(冻结)。
- [ ] 正常:dynamic ShaderBox(loading 弧)→ 按节流时钟推进 time,旋转。
- [ ] 护栏:box 滚出视口 → cull,`FrameStats.shaderbox_active` 减;滚回 → 复现。
- [ ] 护栏:节流时钟 = 30fps → time 步进 ≈ 33ms,不随主 rAF 60fps。
- [ ] params:改 `params` → 下一帧颜色/形状变(0018 storage 增量,不重建管线)。
- [ ] 度量:N 个 box → `shaderbox_pixels` = Σ 屏上像素;离屏不计。
- [ ] 平台:WebGL2 路 raymarch shader 步数封顶,不崩(护栏5)。

## 5. Scope · 不做什么

- ❌ **内容/markdown 开放任意 shader**(运行时编译 + 安全沙箱 + GLSL 转译)——v1 仅内置编译期集。
- ❌ 多 pass / ping-pong buffer / 反馈(先单 pass);复杂 channel 链后续。
- ❌ CPU 回退渲染(Canvas2D);ShaderBox 无 GPU 时不显(占位/降级另议)。
- ❌ 收编 0018 panel(panel 已有,正交保留;不强制并入)。
- ❌ 鼠标 `iMouse` 交互(接 TODO Q 后再加)。
- ❌ 运行时 GLSL(图标 = 编译期手写 WGSL,非运行时转译)。
- ❌ **商用 / 公开分发本 icon 库**(§2.5 卡造型 LICENSE 禁商用+非商用;本项目限个人 test 自用,转商用/分发前须重核并很可能整库重画 → §6)。
- ⚠️ **整盘移植 PixelSpiritDeck 卡造型**:**v1 采用**(作者接受风险),非"不做";风险与边界见 §2.5 / §6。agent logo(§2.6)仍走自写 + LYGIA 噪声,不 verbatim 抄 shadertoy。

## 6. Risk / Open

- **每 shader-id 一 pipeline → 管线数**:内置集小(handful)可控;若膨胀,按需懒建 + 上限。
- **节流时钟协调**:dynamic box 自时钟与主 rAF/reveal 调度器/锚底的协调(别各跑各的导致抖);统一一个"动效时钟源"。
- **面积封顶 downscale**:离屏纹理 + 放大的实现成本 vs 收益;先留钩子,基准超标才启。
- **always-active 与冻结命脉**:这是首个有意不冻图元——靠护栏 1/2/4 把"活跃"压到最小(离屏不算、静态冻、降帧);需基准守"屏上活跃 box × 像素"上限。
- **许可(2026-06-20 改判,记结论 + 风险)**:作者拍板对个人/非商用 test 项目**整盘 verbatim 移植 PixelSpiritDeck**(SDF 工具箱 + 50 卡 icon)。**已知风险**:卡造型 LICENSE 明禁商用**和**非商用产品使用——本结论是**有意带风险采纳**,非"已合规"。**触发重审条件**(任一即停):转商用、公开仓库/分发二进制、对外发布含该库的产品。届时:① SDF helper(LYGIA 数学)可留;② 50 卡造型须**整库自画重绘或取授权**。deny.toml/许可清单标记此项为"vendored-with-risk, non-commercial test only"。agent logo(§2.6)仍走自写 + LYGIA 噪声。
- **Open**:① 节流时钟默认值(30fps?)与可配?② params 布局(固定 K vec4 vs 变长 storage)?③ raymarch 收编是相位⑤还是另案?④ ~~`IconId` 注册表形态~~ → **定:枚举,值=源 case 号,携 `dynamic` 标志**(§2.5c)。⑤ ~~功能图标 v1 集~~ → **定:整盘 50 移植**;聊天功能图标先映射盘内 icon,缺的工具箱自画追加(id≥50)。⑥ agent logo 静止态默认静态还是常驻呼吸(耗 vs 活泼)?⑦(新)整盘 50 dynamic 默认 30fps 节流——多个 icon 同屏(如 contact-sheet/工具条)的活跃岛总像素是否超护栏阈?基准守(§6 always-active)。

## 7. Done

`ShaderBox` 图元落地(宽高 + 内置 WGSL shader + 背景 + time/resolution/params + 可选输入纹理);**五护栏随图元强制**(离屏 cull / 静态即冻 / 面积封顶钩子 / 降帧节流 / 平台 caps)+ `FrameStats` 度量;首批内置 shader(loading / copy→✓ morph / 装饰)上屏;**内置 animated icon 库**(§2.5:SDF 工具箱 → `base/sdf.wgsl` + `icons.wgsl` 整盘 50 支 switch + `IconId` 注册 + 功能别名映射;46 animated / 4 静态;附录 A 源)+ **Agent logo**(§2.6:glow-orb ShaderBox 放 AsstBox 头像位,自写)落地;0024 raymarch 收编为一个 shader-id;卡口(cargo/clippy native+wasm、wasm-pack、tsc、wgsl 解析)全绿;性能基准(活跃 box × 像素 vs fps)入册。

## 8. 关联

- decision:[0028](../decision/0028-shaderbox-primitive.md)(主)/ [0026](../decision/0026-modular-shader-organization.md)(shader 拼接)/ [0018](../decision/0018-sdf-panel-decoration-primitive.md)(storage 数据通道)/ [0024](../decision/0024-3d-camera-and-raymarch-sdf.md)(raymarch 收编 + 平台 caps)/ [0025](../decision/0025-sdf-node-animation-system.md)·[0016](../decision/0016-streaming-morph-render-model.md)(enter/盒位,动态内容由 shader time 驱动)/ [thinking §4](../design/thinking.md)(效果底座);plan13(Taffy)/ plan14(channel 纹理)/ plan15(copy→✓ morph)。
- Code 入口:`crates/core/src/frame.rs`(`FrameShaderBox`)·`app.rs`(Taffy 叶子 + build_frame + cull/冻/节流 + FrameStats)·`crates/render`(`shaderbox` pipeline + `shaders/shaderbox/*`)·`backend.rs`(shader 注册/拼接)。

---

## 附录 A — 源 GLSL(翻译基准 / vendored-with-risk,见 §2.5 / §6)

> PixelSpiritDeck 整盘 contact-sheet shader。`icons.wgsl` 按此**逐字 WGSL 直译**:工具箱 → `base/sdf.wgsl`,`draw()` 的 `switch` → `icon_id` 分派,`mainImage` 网格仅作预览(产品取单格)。`iMouse` hover 分支 v1 不译(§2.5 契约)。

```glsl
// 0→1 test 项目内置 animated icon 库源(PixelSpiritDeck contact sheet)
// 许可:见 §2.5 / §6 —— 个人/非商用 test 自用,带风险采纳
#define S (1. + sin(iTime) / 4.)
#define C (1. + cos(iTime) / 4.)

// smoothstep 抗锯齿包装(源:卡用 step,这里多数换 sstep 防移动锯齿)
float sstep(float a, float b) { return smoothstep(a - .005, a + .005, b); }

const float PI = 3.14159;
const float TAU = PI * 2.;
const float QTR_PI = PI / 4.;

// === SDF 工具箱(括号=源卡编号) ===
float stroke(float x, float s, float w) { // 04
    float d = sstep(s, x + w / 2.) - sstep(s, x - w / 2.);
    return clamp(d, 0., 1.);
}
float circleSDF(vec2 st) { return length(st - 0.5) * 2.; } // 08
float fill(float x, float size) { return 1. - sstep(size, x); } // 09
float rectSDF(vec2 st, vec2 s) { // 10
    st = st * 2. - 1.;
    return max(abs(st.x / s.x), abs(st.y / s.y));
}
float crossSDF(vec2 st, float s) { // 11
    vec2 size = vec2(0.25, s);
    return min(rectSDF(st, size.xy), rectSDF(st, size.yx));
}
float flip(float v, float pct) { return mix(v, 1. - v, pct); } // 12
float vesicaSDF(vec2 st, float w) { // 14
    vec2 offset = vec2(w * .5, 0.);
    return max(circleSDF(st - offset), circleSDF(st + offset));
}
float triSDF(vec2 st) { // 16
    st = (2. * st - 1.) * 2.;
    return max(abs(st.x) * 0.866025 + st.y * .5, -st.y * .5);
}
float rhombSDF(vec2 st) { return max(triSDF(st), triSDF(vec2(st.x, 1. - st.y))); } // 17
vec2 rotate(vec2 st, float a) { // 19
    st = mat2(cos(a), -sin(a), sin(a), cos(a)) * (st - .5);
    return st + .5;
}
float polySDF(vec2 st, int V) { // 26
    st = st * 2. - 1.;
    float a = atan(st.x, st.y) + PI;
    float r = length(st);
    float v = TAU / float(V);
    return cos(floor(.5 + a / v) * v - a) * r;
}
float hexSDF(vec2 st) { // 27
    st = abs(st * 2. - 1.);
    return max(abs(st.y), st.x * 0.866025 + st.y * .5);
}
float starSDF(vec2 st, int V, float s) { // 28
    st = st * 4. - 2.;
    float a = atan(st.y, st.x) / TAU;
    float seg = a * float(V);
    a = ((floor(seg) + 0.5) / float(V) + mix(s, -s, step(.5, fract(seg)))) * TAU;
    return abs(dot(vec2(cos(a), sin(a)), st));
}
float raysSDF(vec2 st, int N) { // 30
    st -= .5;
    return fract(atan(st.y, st.x) / TAU * float(N));
}
float heartSDF(vec2 st) { // 34
    st -= vec2(.5, .8);
    float r = length(st) * 5.;
    st = normalize(st);
    return r - ((st.y * pow(abs(st.x), 0.67)) / (st.y + 1.5) - 2. * st.y + 1.26);
}
float bridge(float c, float d, float s, float w) { // 35
    c *= 1. - stroke(d, s, w * 2.);
    return c + stroke(d, s, w);
}
float spiralSDF(vec2 st, float t) { // 47
    st -= .5;
    float r = dot(st, st);
    float a = atan(st.y, st.x);
    return abs(sin(fract(log(r) * t + a * 0.159)));
}
vec2 scale(vec2 st, vec2 s) { return (st - .5) * s + .5; }
float flowerSDF(vec2 st, int N) {
    st = st * 2. - 1.;
    float r = length(st) * 2.;
    float a = atan(st.y, st.x);
    float v = float(N) * .5;
    return 1. - (abs(cos(a * v)) * .5 + .5) / r;
}

// === icon switch(icon_id = 源 case 号) ===
float draw(vec2 st, vec2 tileXY, vec2 count) {
    int cardNumber = int(tileXY.x + (-tileXY.y + count.y - 1.) * count.x);
    float color = 0.;
    switch (cardNumber) {
    case 0: { color = 0.; break; } // void
    case 1: { color = sstep(0.5 * S, st.x); break; } // justice
    case 2: { color = sstep(0.5 + cos(st.y * PI + iTime/2.) * 0.25, st.x); break; } // strength
    case 3: { color = sstep(0.5, (st.x * S + st.y * C) * 0.5); break; } // death
    case 4: { color = stroke(st.x, 0.5, 0.15*S); break; } // wall
    case 5: { // temperance
        float offset = cos(st.y * PI + iTime) * 0.15;
        color = stroke(st.x, .28 + offset, 0.1);
        color += stroke(st.x, .5 + offset, 0.1);
        color += stroke(st.x, .72 + offset, 0.1);
        break;
    }
    case 6: { // branch
        float offset = 0.5 + (st.x - st.y) * 0.5;
        color = stroke(offset, 0.5, 0.1 * S);
        break;
    }
    case 7: { // the hanged man
        float sdf = 0.5 + (st.x - st.y) * 0.5;
        color = stroke(sdf, 0.5, 0.1 * C);
        float sdf_inv = (st.x + st.y) * 0.5;
        color += stroke(sdf_inv, 0.5, 0.1 * C);
        break;
    }
    case 8: { color = stroke(circleSDF(st), 0.5 * S, 0.05 * C); break; } // the high priestess
    case 9: { // the moon
        color = fill(circleSDF(st), 0.65);
        vec2 offset = vec2(0.1, 0.05);
        color -= fill(circleSDF(st - offset * S), 0.5);
        break;
    }
    case 10: { // the emperor
        float sdf = rectSDF(st, vec2(1.));
        color = stroke(sdf, .5 * C, .125);
        color += fill(sdf, .1 * S);
        break;
    }
    case 11: { // the hierophant
        float rect = rectSDF(st, vec2(1));
        color = fill(rect, .5);
        float cross = crossSDF(st, 1.);
        color *= sstep(.5, fract(cross * 3. + iTime));
        color *= sstep(1., cross);
        color += fill(cross, .5);
        color += stroke(rect, .65, .05);
        color += stroke(rect, .75, .025);
        break;
    }
    case 12: { // the tower
        float rect = rectSDF(st, vec2(.5, 1.));
        float diag = (st.x * C + st.y * S) * .5;
        color = flip(fill(rect, .6), stroke(diag, .5, .01));
        break;
    }
    case 13: { // merge
        vec2 offset = vec2(.15 * S, 0);
        float left = circleSDF(st + offset);
        float right = circleSDF(st - offset);
        color = flip(stroke(left, .5, .05), fill(right, 0.525));
        break;
    }
    case 14: { // hope
        float sdf = vesicaSDF(st, .2 * S);
        color = flip(fill(sdf, .5), sstep((st.x + st.y) * .5, .5));
        break;
    }
    case 15: { // the temple
        st.y = 1. - st.y;
        vec2 ts = vec2(st.x, .82 - st.y);
        color = fill(triSDF(st), .7);
        color -= fill(triSDF(ts), .36);
        break;
    }
    case 16: { // the summit
        float circle = circleSDF(st - vec2(.0, .1));
        float triangle = triSDF(st + vec2(.0, .1));
        color = stroke(circle, .5 * C, .1);
        color *= sstep(.55, triangle);
        color += fill(triangle, .45);
        break;
    }
    case 17: { // the diamond
        float sdf = rhombSDF(st);
        color = fill(sdf, .425 * S);
        color += stroke(sdf, .5 * S, .05);
        color += stroke(sdf, .6 * C, .03);
        break;
    }
    case 18: { color = flip(fill(triSDF(st), .5), fill(rhombSDF(st), .4)); break; } // the hermit
    case 19: { // intuition
        st = rotate(st, radians(-25.) * S);
        float sdf = triSDF(st);
        sdf /= triSDF(st + vec2(0., .2 * C));
        color = fill(abs(sdf), .56);
        break;
    }
    case 20: { // the stone
        st = rotate(st, radians(45.));
        color = fill(rectSDF(st, vec2(1.)), .4);
        color *= 1. - stroke(st.x, .5 * S, .02);
        color *= 1. - stroke(st.y, .5 * C, .02);
        break;
    }
    case 21: { // the mountain
        st = rotate(st, radians(-45.));
        float off = .12 * S;
        vec2 s = vec2(1.);
        color = fill(rectSDF(st + off, s), .2 * C);
        color += fill(rectSDF(st - off, s), .2 * C);
        float r = rectSDF(st, s);
        color *= sstep(.33, r);
        color += fill(r, .3);
        break;
    }
    case 22: { // the shadow
        st = rotate(vec2(st.x, 1. - st.y), radians(45.));
        vec2 s = vec2(1.);
        color += fill(rectSDF(st - .025 * S, s), .4);
        color += fill(rectSDF(st + .025, s), .4);
        color *= sstep(0.38, rectSDF(st + .025, s));
        break;
    }
    case 23: { // opposite
        st = rotate(st, radians(-45.));
        vec2 s = vec2(1.);
        float o = .05 * S * 1.5;
        color += flip(fill(rectSDF(st - o, s), .4), fill(rectSDF(st + o, s), .4));
        break;
    }
    case 24: { // the oak
        st = rotate(st, radians(45.));
        float r1 = rectSDF(st, vec2(1.) * S);
        float r2 = rectSDF(st + .15 * S, vec2(1.));
        color += stroke(r1, .5, .05);
        color *= sstep(.325, r2);
        color += stroke(r2, .325, .05) * fill(r1, .525);
        color += stroke(r2, .2, .05);
        break;
    }
    case 25: { // ripples
        st = rotate(st, radians(-45.)) - .08;
        for (int i = 0; i < 4; i++) {
            float r = rectSDF(st, vec2(1.) * S);
            color += stroke(r, .19, .04);
            st += .05;
        }
        break;
    }
    case 26: { // the empress
        float d1 = polySDF(st, 5);
        vec2 ts = vec2(st.x, 1. - st.y);
        float d2 = polySDF(ts, 5);
        color = fill(d1, .75) * fill(fract(d1 * 5. - iTime/2.), .5);
        color -= fill(d1, .6) * fill(fract(d2 * 4.9 - iTime/2.), .45);
        break;
    }
    case 27: { // bundle
        st = st.yx;
        color = stroke(hexSDF(st), .6 * C, .1);
        color += fill(hexSDF(st - vec2(-.06, -.1) * S), .15);
        color += fill(hexSDF(st - vec2(-.06, .1) * S), .15);
        color += fill(hexSDF(st - vec2(.11, 0.) * S), .15);
        break;
    }
    case 28: { // the devil
        color += stroke(circleSDF(st), .8 * C, .05);
        st.y = 1. - st.y;
        float s = starSDF(st.yx, 5, .1);
        color *= sstep(.7 * C, s);
        color += stroke(s, .4 * S, .1);
        break;
    }
    case 29: { // the sun
        float bg = starSDF(st, 16, .1 * S);
        color += fill(bg, 1.3);
        float l = 0.;
        for (float i = 0.; i < 8.; i++) {
            vec2 xy = rotate(st, QTR_PI * i+iTime/4.);
            xy.y -= .3;
            float tri = polySDF(xy, 3);
            color += fill(tri, .3);
            l += stroke(tri, .3 * S, .03);
        }
        color *= 1. - l;
        float c = polySDF(st, 8);
        color -= stroke(c, .15, .04);
        break;
    }
    case 30: { // the star
        color = stroke(raysSDF(st, 8), .5, .15 * C * 2.);
        float inner = starSDF(st.xy, 6, .09 * S);
        float outer = starSDF(st.yx, 6, .09 * S);
        color *= sstep(.7, outer);
        color += fill(outer, .5);
        color -= stroke(inner, .25, .06);
        color += stroke(outer, .6, .05);
        break;
    }
    case 31: { // judgement
        color = flip(stroke(raysSDF(rotate(st, -iTime/8.), 28), .5, .2), fill(st.y, .5));
        float rect = rectSDF(st, vec2(1) * S);
        color *= sstep(.25, rect);
        color += fill(rect, .2);
        break;
    }
    case 32: { // wheel of fortune
        float sdf = polySDF(rotate(st.yx, C), 8);
        color = fill(sdf, .5);
        color *= stroke(raysSDF(rotate(st, C), 8), .5, .2);
        color *= sstep(.27, sdf);
        color += stroke(sdf, .2, .05);
        color += stroke(sdf, .6, .1);
        break;
    }
    case 33: { // vision
        float v1 = vesicaSDF(st, .5);
        vec2 st2 = st.yx + vec2(.04, .0);
        float v2 = vesicaSDF(st2, .7);
        color = stroke(v2, 1., .05);
        st = rotate(st, iTime/2.);
        color += fill(v2, 1.) * stroke(circleSDF(st - vec2(.05)), .3 , .05);
        color += fill(raysSDF(st, 50), .2) * fill(v1, 1.25) * sstep(1., v2);
        break;
    }
    case 34: { // the lovers
        color = fill(heartSDF(st), .5 * C * 1.2);
        color -= stroke(polySDF(st, 3), .15 * S * 1.1, .05);
        break;
    }
    case 35: { // the magician
        st.x = flip(st.x, step(.5, st.y));
        vec2 offset = vec2(.15 * S, .0);
        float left = circleSDF(st + offset);
        float right = circleSDF(st - offset);
        color = stroke(left, .4 * S, .075);
        color = bridge(color, right, .4 * S, .075);
        break;
    }
    case 36: { // the link
        st = st.yx;
        st.x = mix(1. - st.x, st.x, step(.5, st.y));
        vec2 o = vec2(.1, .0);
        vec2 s = vec2(1.) * C;
        float a = radians(45.) + iTime/2.;
        float l = rectSDF(rotate(st + o, a), s);
        float r = rectSDF(rotate(st - o, -a), s);
        color = stroke(l, .3, .1);
        color = bridge(color, r, .3, .1);
        color += fill(rhombSDF(abs(st.yx - vec2(.0, .5))), .1);
        break;
    }
    case 37: { // holding together
        st.x = mix(1. - st.x, st.x, step(.5, st.y));
        vec2 o = vec2(.05, .0);
        vec2 s = vec2(1.);
        float a = radians(45.);
        float l = rectSDF(rotate(st + o, a * S), s);
        float r = rectSDF(rotate(st - o, -a * S), s);
        color = stroke(l, .145, .098);
        color = bridge(color, r, .145, .098);
        break;
    }
    case 38: { // the chariot
        float r1 = rectSDF(st, vec2(1.));
        float r2 = rectSDF(rotate(st, radians(45.)), vec2(1.));
        float inv = step(.5, (st.x + st.y) * .5);
        inv = flip(inv, step(.5, .5 + (st.x - st.y) * .5));
        float w = .075 * S * 1.2;
        color = stroke(r1, .5, w) + stroke(r2, .5, w);
        float bridges = mix(r1, r2, inv);
        color = bridge(color, bridges, .5, w);
        break;
    }
    case 39: { // the loop
        float inv = sstep(.5, st.y);
        st = rotate(st, radians(-45.)) - .2;
        st = mix(st, .6 - st, sstep(.5, inv));
        for (int i = 0; i < 5; i++) {
            float r = rectSDF(st, vec2(1.));
            float s = .25;
            s -= abs(float(i) * .1 - .2);
            color = bridge(color, r, s, .05 * S);
            st += .1;
        }
        break;
    }
    case 40: { // turning point
        st = rotate(st, radians(-60.) + iTime/4.);
        st.y = flip(st.y, step(.5, st.x));
        st.y += .25;
        float down = polySDF(st, 3);
        st.y = 1.5 - st.y;
        float top = polySDF(st, 3);
        color = stroke(top, .4, .15 * S);
        color = bridge(color, down, .4, .15 * S);
        break;
    }
    case 41: { // trinity
        st.y = 1. - st.y;
        float s = .25 * C*1.3;
        float t1 = polySDF(st + vec2(.0, .175), 3);
        float t2 = polySDF(st + vec2(.1, .0), 3);
        float t3 = polySDF(st - vec2(.1, .0), 3);
        color = stroke(t1, s, .08) + stroke(t2, s, .08) + stroke(t3, s, .08);
        float bridges = mix(mix(t1, t2, step(.5, st.y)), mix(t3, t2, step(.5, st.y)), step(.5, st.x));
        color = bridge(color, bridges, s, .08);
        break;
    }
    case 42: { // the cauldron
        float n = 12.;
        float a = TAU / n;
        for (float i = 0.; i < n; i++) {
            vec2 xy = rotate(st, a * i);
            xy.y -= .189;
            float vsc = vesicaSDF(xy, .3);
            color *= 1. - stroke(vsc, .45 * S, .1) * sstep(.5, xy.y);
            color += stroke(vsc, .45 * S, .05);
        }
        break;
    }
    case 43: { // the elders
        float n = 3.;
        float a = TAU / n;
        for (float i = 0.; i < n * 2.; i++) {
            vec2 xy = rotate(st, a * i);
            xy.y -= .09;
            float vsc = vesicaSDF(xy, .3);
            color = mix(
                color + stroke(vsc, .5, .1*S),
                mix(color, bridge(color, vsc, .5, .1*S), step(xy.x, .5) - step(xy.y, .4)),
                step(3., i)
            );
        }
        break;
    }
    case 44: { // the core
        float star = starSDF(st, 8, .063);
        color += fill(star, 1.22);
        float n = 8.;
        float a = TAU / n;
        for (float i = 0.; i < n; i++) {
            vec2 xy = rotate(st, 0.39 + a * i);
            xy = scale(xy, vec2(1., .72) * S);
            xy.y -= .125;
            color *= sstep(.235, rhombSDF(xy));
        }
        break;
    }
    case 45: { // inner truth
        st -= .5;
        float r = dot(st, st);
        float a = atan(st.y, st.x) / PI;
        vec2 uv = vec2(a, r);
        vec2 grid = vec2(5., log(r) * 20. * S);
        vec2 uv_i = floor(uv * grid);
        uv.x += .5 * mod(uv_i.y, 2.);
        vec2 uv_f = fract(uv * grid);
        float shape = rhombSDF(uv_f);
        color += fill(shape, .9) * sstep(.75, 1. - r);
        break;
    }
    case 46: { // the world
        color = fill(flowerSDF(rotate(st, -iTime/4.), 5), .25*C);
        color -= sstep(.95, starSDF(rotate(st, 0.628 - iTime/4.), 5, .1*S));
        color = clamp(color, 0., 1.);
        float circle = circleSDF(st);
        color -= stroke(circle, .1, .05);
        color += stroke(circle, .8, .07);
        break;
    }
    case 47: { color = sstep(.5, spiralSDF(rotate(st, iTime/2.), .13 * S)); break; } // the fool
    case 48: { color = 1.; break; } // enlightenment
    case 49: { // elements(作者追加,近似)
        st = rotate(st, -iTime/4.);
        float d = .15;
        float r = .3 * S;
        color = fill(circleSDF(st - vec2(cos(TAU / 3.), sin(TAU / 3.)) * d), r);
        color += fill(circleSDF(st - vec2(cos(TAU / 3. * 2.), sin(TAU / 3. * 2.)) * d), r);
        color += fill(circleSDF(st - vec2(d, 0.)), r);
        st = st.yx;
        st.y = 1. - st.y;
        color *= 1. - fill(triSDF(st-vec2(0, .02)), .13);
        color += stroke(circleSDF(st), .8, .08);
        break;
    }
    }
    return color;
}

// 网格预览(产品按 icon_id 取单格,见 §2.5 契约)
void mainImage( out vec4 fragColor, in vec2 fragCoord ) {
    vec2 uv = fragCoord / iResolution.xy;
    float coordAspectRatio = iResolution.y / iResolution.x;
    vec2 count = vec2(10, 5);
    float tileW = iResolution.x / count.x;
    float tileH = iResolution.y / count.y;
    float tileAspectRatio = tileH / tileW;
    vec2 tileXY = floor(uv * count);
    vec2 st = vec2(
        uv.x * count.x - tileXY.x,
        (uv.y * count.y - tileXY.y - 0.5) * tileAspectRatio + .5
    );
    vec2 gridBars = clamp(cos(uv * TAU * count) * 10. - 9.8, 0., 1.);
    float grid = max(gridBars.x, gridBars.y);
    float color = draw(st, tileXY, count);
    color = clamp(color + grid, 0., 1.);
    // iMouse hover 预览分支 v1 不译(§2.5)
    fragColor = vec4(color);
}
```
