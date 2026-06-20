# 决策记录 0028:ShaderBox —— 矩形 shader 画板图元(内置程序化动效 / SDF 特效底座)

- 日期:2026-06-19
- 状态:已采纳(原型验证前)
- 前置:[0011 一切皆片元/SDF](0011-gpu-text-as-sdf-primitive.md)、[0018 SDF 面板装饰](0018-sdf-panel-decoration-primitive.md)、[0024 §4B raymarch 区域 quad](0024-3d-camera-and-raymarch-sdf.md)、[0026 模块化 shader](0026-modular-shader-organization.md)、[0025 anim](0025-sdf-node-animation-system.md)/[0016 morph](0016-streaming-morph-render-model.md)、[thinking §4 SDF 效果底座](../design/thinking.md)、[plan13 Taffy 叶子](../plan/plan13-chat-box-layout.md)、[plan14 §2.5 内置动效=程序化](../plan/plan14-image-embed.md)、[plan15 §2.7 copy 图标升级路](../plan/plan15-code-block-viewport.md)
- 范围:新增一个 shadertoy 式矩形图元——宽高 + 片元 shader + 背景 + 可选输入纹理 + time/resolution/params。

## 1. 背景 / 问题

- 已定:**内置图标/chrome 动效走"程序化 SDF/shader",不用 GIF/动画 SVG 文件**(plan14 §2.5 / plan15 §2.7)。但**缺一个承载任意片元 shader 的通用图元**。
- 现有图元全是**固定功能**:`glyph`(SDF 字)、`rect`、`panel`(0018 写死面板)、`widget`(0026 按 component-id 分派固定小 `fn`)。没有"给一块矩形配**一段任意 fragment** + iTime/iResolution/params"的画板。
- [0024 §4B] raymarch 区域 quad 其实就是这东西,但**未抽象成可复用组件**。
- 目标:一个 **`ShaderBox`** —— 内置**程序化动效 + SDF 特效的统一底座**:loading 转圈、copy→✓ `mix` morph、装饰、扫光/溶解、raymarch,都在这块画板上写 fragment。

## 2. 决策

**新增 `ShaderBox` 图元**:一块世界空间矩形(Taffy 盒/叶子),片元跑一段**命名内置 shader**(shader-id 分派),统一输入契约 + 背景 alpha 叠加。

**契约(uniforms,对应 shadertoy)**:
- `resolution: vec2`(盒 px = iResolution)、`time: f32`(引擎时钟 = iTime;可选 `time_delta`/`frame`)。
- `params: vec4×K`(**每实例参数 = "效果即数据"** thinking §4;走 0018 storage 数据通道复用)。
- `bg: vec4`(背景色)+ 可选 `bg_tex`;输出 fragment 色 + alpha → **over 背景/下层**(alpha 叠加)。
- 可选 `channel0..N: texture`(**输入纹理** = "内置动图作输入",少量;如喂静态 SVG 纹理做溶解/扫光)。

**作用域 v1 = 仅引擎内置**:shader 我们写、**编译期已知集**(`shaders/shaderbox/*.wgsl`,0026 拼接注册,shader-id 分派);**无运行时编译**。内容/markdown 开放(LLM 写 shadertoy)= **明确 defer**(需运行时编译 + 安全沙箱 + GLSL 转译,另案)。

**"内置动图"双解都给**:① 主 = **产出**(我们写 fragment 画动画);② 辅 = **输入**(box 喂纹理作 `channelN`)。

**性能 = 一等设计约束**(§4 详 + 五护栏):离屏 cull / 静态即冻 / 面积分辨率封顶 / 降帧节流 / 内置编译期 + 平台 caps;并入 `FrameStats` 度量。

**集成**:ShaderBox = Taffy 叶子(plan13,measure=宽高);render 新 `shaderbox` pipeline(复用 globals bind + quad mesh,**每 shader-id 一条 pipeline**);盒位变走 0016、静态 enter 走 0025,但**动态内容由 shader 自身 `time` 驱动**(不经 0016)。[0024 §4B] raymarch = ShaderBox 的一个 shader-id(收编)。

## 3. 备选与否决

- **并入 0026 widget 管线**(component-id `switch` 加分支):widget = 超轻固定小 `fn`(checkbox/rule);**任意/重 fragment(raymarch)塞一个巨 switch** → 分支发散、编译膨胀、无法按需。→ ShaderBox **独立图元、每 shader-id 一 pipeline**。分工:**widget = 超轻固定组件;ShaderBox = 带 time/任意 fragment 的画板**。轻静态 SDF 图标仍可走 widget。
- **运行时任意 shader(内容开放)**:首用 ms 级编译卡顿 + 安全(任意 shader 挂 GPU/越界采样)+ GLSL 转译。v1 否决,编译期内置集。
- **GIF/动画 SVG 文件做内置动效**:糊 / 不可重色 / 不可 morph / 每帧上传 / 毁冻结(plan14 §2.5 已否)。
- **CPU 画 → 纹理**:放弃 GPU 并行 + 缩放锐利,否。

## 4. 影响 + 性能账

**正**:内置动效/特效统一底座(thinking §4 落地);收编 0024 raymarch;缩放锐利 + 可重色 + 可 morph;"效果=数据"(params);比 GIF 省(无每帧上传)。
**负**:**首个有意的"永远活跃岛"图元**(动态 box 不冻);新 pipeline/shader 集维护;误配"大面积 × 重 shader"会掉帧(靠护栏 + 度量兜)。

### 性能成本模型(作者关注,单列)

```
每帧成本 ≈ Σ(屏上活跃 ShaderBox) box屏幕像素 × shader每像素代价 × fps
```
- **不随历史/文档长度涨**(离屏 cull = 0);**不破冻结命脉**(仅该 box 区逐帧,其余 settled 照 O(1) 跳)。加的是一项 ∝"屏上 shader 像素面积",**不是** ∝ 文档。CPU 侧每帧仅几个 float,O(活跃 box 数)。
- **量级**:48×48px 图标 × 便宜 SDF(~30 ALU/px)≈ 7 万 ALU/帧 → 占现代 GPU 预算 **~0.0004%**(测不出);全屏 2M px × 100 步 raymarch ≈ 10¹¹ ALU/帧 → **吃满掉帧**。**危险来自面积×步数,不来自"有 shader"**。

### 五护栏(把上限钉死,实现强制)

1. **离屏 cull**:box 不在视口 → 不画(0)。无限会话刚需。
2. **静态即冻**:shader 不引用 `time`(纯静态)→ **画一次当普通面板冻结**,不每帧重跑。只有用 time 的保持活跃。
3. **面积/分辨率封顶**:大 box 限最大渲染分辨率(低分渲染→放大,= shadertoy downscale)。
4. **降帧节流**:装饰动画挂**自带慢时钟**(30fps/按需,与主 rAF 解耦,同 reveal 调度器自时钟思路)。
5. **编译期内置集 + 平台 caps**:管线数有界可批、无运行时编译卡顿;WebGL2/移动端 march 步数封顶 + 降精度(0024 §5)。
+ **度量**:`FrameStats` 加"活跃 ShaderBox 数 + shader 像素/帧",`?debug` 与 fps 同看,超标即暴露。

## 5. 实现入口(file:符号)

- `crates/render`:新 `shaderbox` pipeline + `shaders/shaderbox/{common.wgsl(resolution/time/params/bg),<effect>.wgsl}`;params 走 0018 storage 数据通道。
- `crates/core/src/frame.rs`:`FrameShaderBox{ pos, size, shader_id:u32, params:[f32;K], bg:[f32;4], channels:Vec<u32>, dynamic:bool }`。
- `crates/core/src/app.rs`:ShaderBox 作 Taffy 叶子;build_frame 发 `FrameShaderBox` + cull/静态冻/节流;`FrameStats` 加计数。
- 平台 caps(0024 §5);落地清单 → [plan16](../plan/plan16-shaderbox.md)。
