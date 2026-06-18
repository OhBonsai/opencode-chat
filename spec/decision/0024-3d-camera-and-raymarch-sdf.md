# 决策记录 0024:3D 相机与混合 SDF 渲染(光栅 SDF quad + raymarch SDF)—— 自研薄抽象 vs 引擎,跨平台后端

- 日期:2026-06-17
- 状态:**探索 / 草案**(模型方向定调;实现未排期。源自一组想法,先记录)
- 前置:0001(画布架构 / content→layout→render 契约 / CR1 native 可测)、0011(装饰 = SDF / "一切皆距离场"世界观)、[0015](0015-glyph-source-fallback.md)(字形 SDF 源)、[0018](0018-sdf-panel-decoration-primitive.md)(参数化 SDF 面板图元)、[0020](0020-content-node-identity-model.md)(节点身份 / 场景图)、[0022](0022-dom-overlay-layer.md)(相机同步的 DOM 叠加)、`crates/core/src/camera.rs`(现 `Camera2D`)
- 触发:现相机是纯 2D(`pan`/`zoom`,变换在各 shader 内联)。目标是**完整 3D**:既要把 SDF quad 摆进真实 3D 空间(倾斜 / 翻牌 / 景深),也要 **raymarch**——片元里 march 三维距离函数,**相机退化为 ray 的发生器 / 变换**。看中引擎的两点:① 对 3D 的**抽象**(camera / viewport / 投影);② **跨平台兼容**(WebGPU / WebGL2 / Canvas,经 wgpu)。本 ADR 取这两点之**利**,而不引入整套引擎(与"只有 SDF 的世界"冲突,见 §2)。

---

## 1. 决策:自研薄 3D 抽象(glam)+ 统一相机 → 两条 SDF 通路,**不引入整套 3D 引擎**

- **数学**:引入 [glam](https://github.com/bitshifter/glam-rs)(SIMD f32、wgpu 生态标准、wasm 友好)做向量 / 矩阵 / 相机。**glam 不是引擎,对 SDF 零冲突**——SDF 管线本就需要相机矩阵。
- **相机**:`Camera2D` → **统一 3D 相机**,产 `view_proj: Mat4`(+ 其逆 `inv_view_proj`,给 raymarch 生成 ray)。正交默认(像素级等价当前 2D),透视可切(见 §3)。
- **渲染**:同一相机喂**两条 SDF 通路**(§4):
  - **A. 光栅 SDF quad in 3D**:现有字 / 面板 / 装饰 quad,加每节点 model 变换 + `view_proj` → clip;片元仍求**解析距离场**(任意缩放清晰 + glow/outline/dissolve 不变)。
  - **B. raymarch SDF 场景**:全屏(或区域)quad,片元里按相机生成 ray(origin = eye,dir = 反投影),march 三维距离函数。**相机 = ray 变换**。
- **引擎的两点利,自研拿到**:3D 抽象 = 我们自己的薄 `Camera`/`Viewport`(借 [kiss3d](https://github.com/dimforge/kiss3d) 的 arc-ball **写法**,不引入它);跨平台 = **wgpu 后端**已抽象 WebGPU/WebGL2/native(§5),Canvas2D 另作极简回退。

## 2. 为什么不引入引擎(与"只有 SDF 的世界"的冲突)

| | 引入引擎(rend3 / three-d / kiss3d) | 自研薄抽象(本 ADR) |
|---|---|---|
| 原语 | **三角网格 + 材质 + 贴图 + 灯光**(光栅 mesh)——与"一切皆距离场"对立 | **SDF**(字/面板/装饰/raymarch)原语不变 |
| 帧图主权 | 引擎掌管 frame(相机 / render graph / pass) | 自己掌管(atlas / instance / morph / reveal / 裁剪 / 契约) |
| 我们的内容 | 退化成 mesh 里的贴图 quad(丢 SDF 清晰 + 特效),或硬塞自定义材质跟引擎对着干 | 原生 SDF,一等公民 |
| 场景图 | 引擎 ECS / scene-graph 与 [0020] 节点树抢位 | [0020] 即场景图,统一 |

**结论:3D 与 SDF 正交(3D 是变换,SDF 是原语;raymarching 本就是"一切皆 fragment 距离函数"的 3D 范式),冲突的是引擎不是 3D。"只有 SDF"恰是不引擎的最强理由**——SDF 是护城河(任意缩放清晰、GPU 特效、数据极小),通用 mesh 引擎稀释的正是这个差异点。引擎仅作**相机/视口写法的参考**(kiss3d arc-ball)。守 0→1 准则:朝目标一次做对,不为兼容旧 2D 路径妥协。

## 3. 统一相机模型(正交默认 + 透视可切)

- **持有**:`eye`/`target`/`up`、投影参数(ortho:可视高 + near/far;perspective:fov + aspect + near/far)、`viewport`。
- **产出**:`view_proj: Mat4`(光栅通路用)、`inv_view_proj: Mat4` + `eye`(raymarch 生成 ray 用)。
- **正交 = 当前 2D 的精确超集**:`pan`/`zoom` 映射为正交相机的平移 + 可视范围缩放,**像素级不回归**(DoD)。
- **透视**:启用后有真实近大远小 → 倾斜 / 翻牌 / 景深视差。注:平铺文字在透视下会随深度变形,**默认视角须保证文字像素清晰**(z=0 平面 1:1 映射的经典摆法)。
- **取代** `Globals` 里的 `cam_pan: vec2 + cam_zoom: f32` 为单个 `view_proj: mat4x4`(+ 必要时 `inv_view_proj`/`eye`/`viewport` 给 raymarch 与 fwidth AA)。三 shader(`glyph.wgsl`/`rect.wgsl`/`panel.wgsl`)同改。

## 4. 两条 SDF 通路(共享相机 / 视口 / 深度)

**A. 光栅 SDF quad in 3D**
- 顶点:世界坐标升为 `vec3`(z = 每节点深度,默认 0);`clip = view_proj * model * vec4(world, 1)`。`model` = 每节点 3D 变换(倾斜 / 翻牌 / 位移),默认单位阵 → 等价 2D。
- 片元:不变——求 2D 距离场(字 SDF / `sdRoundBox` 面板),smoothstep + `fwidth` AA(透视下 `fwidth` 仍随屏幕导数自适应)。
- 来源:[0015] 字、[0018] 面板、装饰 rect。

**B. raymarch SDF 场景**
- 几何:一个全屏(或锚定区域)quad。
- 片元:由相机生成 ray(`origin = eye`,`dir = normalize(inv_view_proj 反投影像素 - eye)`),sphere-trace 三维距离函数(`sdScene(p)`),命中后法线 / 着色 / 软阴影 / AO 全在距离场里算。**相机即 ray 变换**(本 ADR 触发点)。
- 合成:与 A 通路按**深度**混合(raymarch 命中点的世界深度 → 写 `clip.z`,与光栅 quad 同一 depth buffer 排序);或分层 + order 显式合成。
- 借鉴技术(非代码):IQ 距离场函数 / soft shadow / 法线估计(0011 已沿用 OneDraw zlib;shadertoy CC-BY-NC 仅借技术,见既有约束)。

**共享**:同一 `Camera` 驱动 A 的 `view_proj` 与 B 的 ray;同一 `viewport`;同一 depth 语义。混合渲染(2D 文字海量 + 偶发 3D raymarch 场景)在同一相机下自洽。

## 5. 跨平台后端(看中的第二点)

- **wgpu 已抽象**:WebGPU(首选)/ WebGL2(回退)/ native(Metal/Vulkan/DX)。一套 wgsl + 后端切换。
- **WebGL2 约束(须设计回退)**:
  - 无 compute;storage buffer 受限 → [0018] 面板的 storage 参数在 WebGL2 走 **UBO 回退**(或纹理打包)。
  - raymarch 是**片元密集**:WebGL2 可跑,但循环步数 / 精度 / 分支受限 → march 步数封顶 + 早停 + 降精度档。
  - 纹理数组 / 半精度等特性按 caps 探测降级。
- **Canvas2D**:**另一条极简回退**(非 wgpu;无 SDF/raymarch,仅近似文本),仅作"GPU 全不可用"兜底,不追三端等价。
- **取向**:WebGPU 全功能(含 raymarch);WebGL2 等价光栅 SDF + 受限 raymarch;Canvas2D 兜底。**不强求三端像素等价**,只保内容可读。

## 6. 对现有的影响(改面)

- `camera.rs`:`Camera2D` → 3D 相机(glam `Mat4`);`pan_by_screen`/`zoom_at`/`screen_to_world` 重表为相机操作 + 反投影。`visible_world_rect`(2D AABB 裁剪)在**正交平面**仍有效;透视 / raymarch 需 frustum 或保守包围,**分相位**。
- `frame.rs` / `Globals` / 三 shader:`cam_pan`+`cam_zoom` → `view_proj`(+ raymarch 所需)。
- 顶点输入:`pos: vec2` → 世界 `vec3`(z 默认 0);可选每节点 `model`。
- **契约不变**:content→layout→render([0001])稳。layout 仍出 2D 世界 px;**3D 的 model / 深度属 render/scene 层**(在 [0020] 节点上挂变换),不污染 content/layout。CR1:相机数学纯算、native 可测。
- 与 [0016] morph:几何补间在世界空间,3D 下仍成立(端点几何 + lerp 不变)。
- 与 [0022] DOM 叠加:DOM box 的相机同步须统一到**同一 `view_proj`**(透视下用 CSS3D `matrix3d`,正交下退化为现 `translate+scale`)。

## 7. 取舍 / 风险

- **像素对齐**:正交默认必须与现 2D 逐像素等价,否则文字 AA / 锚底回归(加回归对拍)。
- **透视下文字清晰**:默认视角需 1:1 摆位;倾斜大角度时文字 SDF 仍清晰但需验证 `fwidth` 行为。
- **raymarch 成本**:全屏 march 开销高 → 限区域 / 限步数 / 仅按需启用;与海量文字共帧的预算分配。
- **深度合成**:光栅 quad 与 raymarch 命中点共用 depth buffer 的正确性(透明文字的排序 / 混合)。
- **WebGL2 退化**:storage→UBO、march 受限——是落地复杂度的主要来源,须早探测。
- **范围蔓延**:raymarch 是大特性;先打地基(相机 + 光栅 3D),raymarch 后置。

## 8. 落地分相位(草案,未排期)

1. **glam + Camera3D(正交等价)**:引 glam;`Camera2D`→3D 相机产 `view_proj`;`Globals`/三 shader 改 `view_proj`;顶点 `vec3`(z=0)。**跑通,默认 2D 像素不回归**(对拍)。
2. **透视可切 + 每节点 model 变换**:相机正交↔透视切换;[0020] 节点挂 `model`(倾斜 / 翻牌 / 位移)。
3. **raymarch 通路**:全屏(区域)SDF quad + 相机 ray 生成 + sphere-trace `sdScene`;与光栅按深度合成。
4. **跨端回退**:WebGL2(storage→UBO、march 限档)、Canvas2D 兜底;caps 探测降级。
5. **frustum 裁剪**:替/补 2D AABB,服务透视 / raymarch。

## 9. 备选(不选)

- **整套 3D 引擎(rend3 / three-d / kiss3d)**:抢渲染主权、mesh 原语与 SDF 世界观对立、ECS/scene-graph 与 [0020] 抢位(§2)。**仅借相机写法**(kiss3d arc-ball)作参考代码。
- **nalgebra 代替 glam**:更通用但更重、graphics 场景更慢;无物理 / 矩阵分解需求时不选。
- **保留 2D 相机 + 旁挂 3D**:违 0→1(双路难调)。统一相机一次做全。

---

参考:[glam](https://github.com/bitshifter/glam-rs) · [mathbench 横评](https://github.com/bitshifter/mathbench-rs) · [kiss3d 复活(glam+wgpu,arc-ball 相机)](https://dimforge.com/blog/2026/01/09/reviving-kiss3d-a-simple-3d-and-2d-graphics-engine/) · [three-d](https://github.com/asny/three-d) · [rend3](https://github.com/BVE-Reborn/rend3) · IQ 距离场技法(技术借鉴,见 0011 / OneDraw)。落点:相机 [`camera.rs`]、shader [0018]/[0015]、场景 [0020]、叠加 [0022]。
