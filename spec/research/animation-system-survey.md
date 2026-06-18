# 调研:动画系统(UI 引擎范式 × SDF)—— 用于本系统的动画系统设计

- 日期:2026-06-17
- 状态:研究 / 设计依据(供后续"SDF 节点动画系统" ADR;合并对话中 egui/GPUI/Flutter 与 IQ/Shadertoy/LYGIA 两轮调研)
- 取向(重要):**不追求动画系统的完整性**。目标是回答——**在"一切皆 SDF"的约束下,用什么结构与接口能搭出一个动画系统,并明确它的能力边界与性能边界**。即:小、SDF-native、贴合本仓架构(0020 节点身份 / 0016 morph / reveal / GPU 实例化 / content→layout→render 契约 / R8 确定性),够用即止。
- 深挖附录:SDF 侧 IQ/Shadertoy/LYGIA 细节见 [`sdf-animation-system.md`](./sdf-animation-system.md);本篇是合并 + 设计 + 边界。

---

## 1. 一句话结论

动画 = **(时钟 × 曲线 × 值映射)绑定到有稳定身份的元素**。两层独立:
- **控制层**(何时/多久/什么曲线/绑哪)—— 抄 GPUI/Flutter 的**声明式**,按 **0020 node key** 续身份。
- **值层**(动什么)—— **SDF 词汇**:tween 的是"场的参数"(变换/阈值/带宽/outline/blend/mask/color),不是顶点+alpha。
- **求值** 放 **GPU per-instance**(不像 egui/GPUI/Flutter 每帧 CPU 重建)——这是性能边界的根。

本仓已有零件就是这套的特例:`glyph.wgsl` alpha-fade、`panel.wgsl` AO 带 + `reveal` mask、`0016 morph` 几何参数 lerp。动画系统 = 把它们统一成"每节点可动 SDF 参数 × 曲线 × per-instance GPU 求值"。

## 2. 控制层调研:三种 UI 引擎范式

| | 范式 | 动画状态存哪 | 用法 | 取舍 |
|---|---|---|---|---|
| **egui** | 纯立即模式 | `Memory` 里 `Id→值`(side-table) | `animate_*` 返回 t,**手动**改属性 | 最裸;布尔/单值为主,无 timeline |
| **GPUI**(Zed) | 立即+保留混合 | 元素 arena 里 `ElementId→start` | `with_animation(id, Animation{dur,curve,repeat}, \|el,delta\| el.modify(delta))`:声明时长/曲线 + 闭包应用 | 优雅、可组合;仍每帧 CPU 重算重应用 |
| **Flutter** | 保留模式(三棵树) | State 上的 `AnimationController` 对象 | 隐式 `AnimatedX` / 显式 `Controller×Curve×Tween`,listener→脏子树重绘 | 最声明、最重;Element/Key 续身份 |

**共同骨架(可迁移)**:`时钟(driver) × 归一进度 t∈[0,1] over duration × 曲线(curve) × 值映射(tween from→to) → 目标属性 of 稳定身份元素,由触发(进/出/属性变)启动`。
**对本系统的取舍**:你是**保留模式**(0020 key 是天然身份)→ **不需要** egui/GPUI 的"Id side-table" hack;直接抄 Flutter/GPUI 的**声明式**(per-kind/per-style 声明一组动画),身份用 node key。但**三者都在 CPU 每帧重算重应用**——这点不抄(见 §5 值层 / §7 性能)。

## 3. 值层调研:SDF 里"动什么"(IQ / Shadertoy / LYGIA)

- **mesh/DOM**:动顶点 + 合成 alpha。可动属性 = transform + opacity + color。
- **SDF**:形状是逐像素求 `sd(p)`,动画 = **tween 距离求值的参数**。SDF-native 可动属性:

| SDF 参数 | 动它的效果 | 出处/零件 |
|---|---|---|
| 采样点变换 `sd(M⁻¹p)` | scale/rotate/translate,**任意缩放仍锐** | IQ 域操作;LYGIA `space/` |
| 阈值(isoline)`d - off(t)` | 字"长出来/化开"、变粗细 | LYGIA `morphological/`(膨胀腐蚀) |
| 羽化带宽 `w(t)` | **模糊→聚焦**淡入(真场模糊,非 alpha) | smoothstep+fwidth |
| outline/glow/ao 带 | 描边/辉光"画上去" | 本仓 panel AO;IQ |
| 形状 blend `smin(k)`/`mix(sdA,sdB,t)` | 形变 / 融合 / 字↔字 morph | IQ smin;LYGIA `sdf/` |
| 裁剪 mask `max(sd, sd_mask(t))` | wipe/揭示 | 本仓 panel `reveal` |
| tint/fill | 颜色 | LYGIA `color/` |

**SDF 范式三条不变式**(从 Shadertoy 抽出,正合本仓):①**场 = 参数的纯函数、无状态**(`params=g(now-start)`)→ 确定性、可重放、可 seek(你的 transport 正是);②**动画 = tween 场参数**(非顶点+alpha);③**正交分解** `shape×material×motion×easing`,加效果=加一条。
**缓动 = 一族纯标量曲线**(IQ "useful functions":smoothstep/gain/parabola/pcurve/expImpulse/cubicPulse/almostUnitIdentity…;LYGIA `animation/` = Penner 全家桶,**有 WGSL**)。
**LYGIA 注意**:多语言(含 **WGSL**)、`#include` 现成叶子函数;但**双授权 Prosperity(非商业)+ Patron(sponsor/contributor)**——商用需赞助/贡献/买授权,故**借技法别抄码**,优先回 IQ 原文(授权更宽);且 WGSL 覆盖不全,用前确认。LYGIA/Shadertoy **都只是 per-pixel 原子,没有"元素动画系统"**(无身份/声明/控制层)——那是本系统要搭的。

## 4. 本系统的动画系统:结构

四层,自上而下;**控制层抄 §2,值层用 §3,求值在 GPU**:

```
① 声明层(policy/data)     每 node-kind / reveal 风格 → 一组 Anim(进场/出场/态变)。声明式,像 Flutter AnimatedX / 0019 "加一条"。
        ↓
② 编排层(resolve, CPU)     0020 节点树 + 就绪门(Plan 9) → 每节点具体 Anim + 绝对 start(交错/骨架先行 = 各自 start)。
        ↓
③ 编码层(seam)            每 glyph/面板实例 → flat 描述(定长进 instance 属性 / 变长进 storage,像 panel params)。
        ↓
④ 求值层(shader, GPU)      t=(now-start)/dur → e=curve(id,t) → param=mix(from,to,e) → 作用于 SDF 求值(变换 p/阈值/带宽/blend/mask/color/alpha)。
```

**与现有统一**:`0016 morph`(几何参数 tween)、`reveal`(mask/阈值 + 进场触发)、装饰(band)= 此系统在不同属性上的特例,收编进 ④ 同一求值。**编排层就是 Plan 9 的 `resolve`**(它已给 `delay_ms`=每节点 start;只需再带动画 profile)。

## 5. 接口设计(结构 + 契约,语言中立)

**5.1 可动属性(值层词汇,按源分)**
```
AnimTarget = Alpha | Scale | Translate | Rotate | Threshold | Band
           | Outline | Glow | BlendK | MixT | MaskProgress | Tint
```
- **字(采样场)合法子集**:Alpha/Scale/Translate/Rotate/Threshold/Band/Outline/MaskProgress/Tint(+ MixT 仅当两字形同 atlas 可同采,成本高,后置)。
- **面板(解析场)超集**:再加 BlendK/Glow/全套 smin 形变。
- 系统按"源类型"校验合法 target(SDF 约束直接进类型)。

**5.2 曲线库**(纯函数,GPU 实现,移植 IQ/LYGIA)
```
CurveId = Linear | SmoothStep | EaseIn | EaseOut | EaseInOut | Gain | ExpImpulse | CubicPulse  (…后续可加 Back/Elastic/Spring)
```

**5.3 动画描述**(挂 0020 key;一节点可叠 N 条,N 有界,见 §7)
```
Anim { target: AnimTarget, from: f32(或小 vec), to: f32, start_ms: f32, dur_ms: f32, curve: CurveId, repeat: bool }
```

**5.4 声明 profile**(声明层):`reveal 风格 / node-kind → Vec<Anim>`(进场)。"行框 header:Alpha 0→1 + Scale 0.8→1,0.5s,EaseIn" = header 节点两条 Anim,**无特例代码**。出场/态变同理。

**5.5 GPU 求值契约**(④):shader 读 `globals.time_ms` + 实例的 Anim 块,在**现有 SDF coverage/着色之前**算出 `p 变换 / 阈值偏移 / 带宽 / blend / mask / 颜色 / alpha`。定长(≤N 条)走 instance 属性;变长走 storage(复用 panel params 通道)。

**5.6 数据流与契约位置**:content→layout→render 不变。**动画属于 render/policy 层**(在 0020 节点上挂 Anim),不污染 content/layout;layout 仍出世界 px,几何变化仍走 measure/relayout + 0016。R8:start 走注入时钟,曲线纯函数 → seek/重放稳。

## 6. 能力边界(明确"能/不能")

**能**:
- 每元素**进场/出场/态变**,作用于 §5.1 SDF 属性集。
- **交错编排**(header 先于 body、逐行)= 每元素不同 start(Plan 9 已给)。
- **任意缩放仍锐**的 transform 动画(SDF 红利)。
- SDF-native 多味淡入(alpha / 阈值墨入 / 带宽模糊)、outline/glow draw-on、面板 smin/mix 形变、mask wipe。
- **确定性 + 可 seek**(纯 `f(now-start)`)→ 直接吃 transport/重放。

**不能 / v1 不做**:
- **不是关键帧时间线**:一条 Anim = 单段 `from→to`;多段靠多条 Anim/stage 串(够编排,不做通用 timeline 编辑器)。
- **无物理/弹簧状态**(无速度态);如需弹性,用**解析阻尼弹簧当一条 curve**(无状态),不引入 stateful 物理。
- **字↔字 shape morph**:需两字形同 atlas 同采(成本/atlas 压力)→ 后置。
- **布局驱动动画**(reflow 补间)仍是 **0016 morph 的活**(几何),本系统管 render 参数;二者统一但**布局变更照走 measure/relayout**,不在 shader 里改版面。
- **跨元素约束 / FLIP / 共享元素(Hero)** 超出 0016 reflow 的范围 → 不做。
- **3D / raymarch 特效**属 [0024](../decision/0024-3d-camera-and-raymarch-sdf.md),不在本系统。
- **每元素并发 Anim 条数有界**(slot 限制,见 §7),不支持无限叠加。

## 7. 性能边界(明确"代价与上限")

**核心命题(决定边界)**:**声明一次,GPU 按时间求值;CPU 只在描述变化时更新。** 这把动画成本压到≈现有渲染成本,而非像 egui/GPUI/Flutter 每帧 CPU 重建。

- **GPU 求值**:每片元几条 mul/add + 一次 curve ≈ **近免费** → **可同时动所有可见字**(万级)。
- **代价驱动项(要盯)**:
  - **fill-rate**:扩大覆盖的效果(glow/band 加宽、scale>1 暂态)→ 多覆盖像素 / overdraw。
  - **多采样**:字↔字 morph = 2× atlas 采样(故后置);mask/threshold/transform = 0 额外采样。
  - **带宽**:per-instance 多 Anim 属性 / storage 描述 → 适度增量;**定长 slot(≤N,建议 N≤4)** 控上限,超出走 storage。
- **CPU 代价**:
  - 编排 `resolve` 现在**每帧每活动 view 跑一次**(Plan 9)——这是真正的 CPU 瓶颈。边界 = **缓存 resolve,仅在内容/门变化时重算**;settled 块描述不变 → CPU 不再碰,GPU 独自按时间推进(= "declare-once" 的关键)。
  - 注意已知项:NodeSpawn/换行 glyph 的 spawn 处理会让活动 view 不冻结(见 Plan 9 review #1 旁注)→ 修掉才能让 settled 块真正 CPU-idle。
- **内存**:per-instance N 个 Anim slot × glyph 数,或 storage buffer;有界。
- **确定性代价**:seek = 按 now 重算(纯函数,便宜);任何随机(微定时)必须 seeded(守 R8/R9)。
- **边界一句话**:**"声明式 + GPU 按时间求值 + settled 块 CPU-idle"可扩到万级动画元素;一旦退回"每帧 CPU 重 resolve/重建"就是上限所在**——所以系统的性能成败在于 resolve 缓存与冻结,而非 shader。

## 8. 推荐(最小可行 + 分期)

- **结构**:§4 四层;**编排层复用 Plan 9 `resolve`**(加 Anim profile);求值层扩 `glyph.wgsl`/`panel.wgsl`。
- **接口**:§5 的 `AnimTarget`/`CurveId`/`Anim`/profile;定长 slot(N≤4)+ storage 兜底。
- **分期**:① 收编现有(alpha-fade + reveal mask + 0016 几何)进统一求值 + 加 CurveId(先 SmoothStep/EaseIn/Out)。② 加 Scale/Translate/Threshold/Band(进场最常用)。③ 出场 + outline/glow draw-on。④ 面板 smin 形变。⑤(后置)字↔字 mix morph。
- **守界**:小属性集 + 几条曲线(覆盖 90%,Flutter AnimatedX 同理);resolve 缓存/冻结是性能命脉;布局动画交 0016,不混入。
- **与 Plan 9 关系**:Plan 9 给了"何时(start/stagger)+ reveal"。本系统是其**"怎么动(per-element SDF 属性 × 曲线)"的正交补全**,共用 resolve + 0020 身份 + GPU 求值。

> 一句话:**控制层抄 GPUI/Flutter 声明式(按 0020 身份)、值层用 SDF 场参数(IQ/LYGIA 叶子)、求值放 GPU per-instance;能力 = 固定 SDF 属性集上的单段进/出/态变 + 交错编排(确定可 seek),不做通用时间线/物理/布局动画;性能 = "声明一次 + GPU 按时间求值 + settled 块 CPU-idle" 可达万级,瓶颈在 resolve 缓存与冻结而非 shader。**

---

参考:控制层 — [GPUI animation example](https://github.com/zed-industries/zed/blob/main/crates/gpui/examples/animation.rs)、egui `animate_*`、Flutter `AnimationController/Tween/Curve`(对话);值层 — [IQ functions](https://iquilezles.org/articles/functions/)、[IQ smin](https://iquilezles.org/articles/smin/)、[Shadertoy](https://www.shadertoy.com/)、[LYGIA(含 WGSL,Prosperity+Patron)](https://lygia.xyz)/[license](https://lygia.xyz/license);深挖见 [`sdf-animation-system.md`](./sdf-animation-system.md)。落点:统一 [0016 morph](../decision/0016-streaming-morph-render-model.md) + reveal([0019](../decision/0019-reveal-gating-and-choreography.md)/Plan 9)+ 装饰([0018](../decision/0018-sdf-panel-decoration-primitive.md))为一套 SDF 节点动画系统。
