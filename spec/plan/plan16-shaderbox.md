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

## 3. 相位

| 相位 | 交付(file:符号) | 验证 |
|---|---|---|
| **① 图元 + 管线** | `FrameShaderBox`(frame.rs)+ `shaderbox` pipeline + `common.wgsl` + 一个示例 shader(SDF 圆/纯色) | 沙箱:wgsl 解析过;**GPU 人工**:一块画板出图 + params 改色 |
| **② 时钟 + 护栏 1/2/4** | core 驱动 time;离屏 cull;静态即冻;节流时钟(§2.3) | cargo:cull/冻判定/节流;`FrameStats` 计数;**GPU 人工**:离屏 0 耗、静态冻、节流 30fps |
| **③ 首批内置 shader** | loading 弧旋转 + **copy→✓ `mix` morph**(接 plan15)+ 一个装饰 | **GPU 人工**:动效、缩放锐利、接 0021 色 |
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

## 6. Risk / Open

- **每 shader-id 一 pipeline → 管线数**:内置集小(handful)可控;若膨胀,按需懒建 + 上限。
- **节流时钟协调**:dynamic box 自时钟与主 rAF/reveal 调度器/锚底的协调(别各跑各的导致抖);统一一个"动效时钟源"。
- **面积封顶 downscale**:离屏纹理 + 放大的实现成本 vs 收益;先留钩子,基准超标才启。
- **always-active 与冻结命脉**:这是首个有意不冻图元——靠护栏 1/2/4 把"活跃"压到最小(离屏不算、静态冻、降帧);需基准守"屏上活跃 box × 像素"上限。
- **Open**:① 节流时钟默认值(30fps?)与可配?② params 布局(固定 K vec4 vs 变长 storage)?③ raymarch 收编是相位⑤还是另案?④ shader-id 注册表形态(枚举 vs 字符串名)?

## 7. Done

`ShaderBox` 图元落地(宽高 + 内置 WGSL shader + 背景 + time/resolution/params + 可选输入纹理);**五护栏随图元强制**(离屏 cull / 静态即冻 / 面积封顶钩子 / 降帧节流 / 平台 caps)+ `FrameStats` 度量;首批内置 shader(loading / copy→✓ morph / 装饰)上屏;0024 raymarch 收编为一个 shader-id;卡口(cargo/clippy native+wasm、wasm-pack、tsc、wgsl 解析)全绿;性能基准(活跃 box × 像素 vs fps)入册。

## 8. 关联

- decision:[0028](../decision/0028-shaderbox-primitive.md)(主)/ [0026](../decision/0026-modular-shader-organization.md)(shader 拼接)/ [0018](../decision/0018-sdf-panel-decoration-primitive.md)(storage 数据通道)/ [0024](../decision/0024-3d-camera-and-raymarch-sdf.md)(raymarch 收编 + 平台 caps)/ [0025](../decision/0025-sdf-node-animation-system.md)·[0016](../decision/0016-streaming-morph-render-model.md)(enter/盒位,动态内容由 shader time 驱动)/ [thinking §4](../design/thinking.md)(效果底座);plan13(Taffy)/ plan14(channel 纹理)/ plan15(copy→✓ morph)。
- Code 入口:`crates/core/src/frame.rs`(`FrameShaderBox`)·`app.rs`(Taffy 叶子 + build_frame + cull/冻/节流 + FrameStats)·`crates/render`(`shaderbox` pipeline + `shaders/shaderbox/*`)·`backend.rs`(shader 注册/拼接)。
