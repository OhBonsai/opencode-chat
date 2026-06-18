# 研究:SDF 世界里的动画系统 —— IQ / Shadertoy 的优雅范式与可复用结构

- 日期:2026-06-17
- 状态:研究 / 设计探索(喂未来"SDF 节点动画系统" ADR;承本仓 reveal/morph 讨论)
- 触发:我们是"一切皆 SDF"的世界(字 = 采样距离场,面板 = 解析 `sdRoundBox`)。要引入**元素动画系统**,问:IQ(SDF 鼻祖 / "SDF golf")与 Shadertoy 社区有没有**优雅、可复用的范式**?其结构是什么?能不能不是"每个 shader 手写 iTime"的 ad-hoc?
- 结论先行:**Shadertoy/IQ 没有正式的"动画系统",但沉淀了一套高度一致、可抽象的范式**——"**场 = 参数(t) 的纯函数**" + "**动画 = tween 场的参数**(不是顶点/alpha)" + "**缓动 = 一组纯标量曲线**" + "**正交分解 shape × material × motion × easing**"。把这套 demoscene 纯函数范式 + 声明式控制层(GPUI/Flutter)+ per-instance GPU 求值结合,就是 SDF UI 的元素动画系统。

---

## 1. IQ 的技术族(动画相关,带出处)

IQ 的文章不是"动画系统",而是**可组合的算子 + 曲线**,正好对应动画系统的"值映射(tween)"和"缓动(curve)"两层。

### 1.1 域操作(animate the domain)= 运动的来源
对**采样点 `p`** 做变换再求 `sd(p)`:平移/旋转/缩放(`sd(M⁻¹p)`)、重复(`opRep`,对 `p` 取模)、域扭曲(domain warp:`p += f(p)`)。**动这些变换 = 运动**,且因为是变换坐标而非重采样,**任意缩放仍锐**。这是 SDF 相对 mesh 的根本红利。来源:[distance functions](https://iquilezles.org/articles/distfunctions/) · [domain warping](https://iquilezles.org/articles/warp/)。

### 1.2 smin / mix = 形状混合(morph 的来源)
- `min(a,b)` 并集会在交界产生导数不连续;**`smin(a,b,k)`(smooth minimum)** 平滑融合。IQ 给了 4+ 种实现(exp / power / root / polynomial),当下他最常用的高效形(Media Molecule《Dreams》同款,Dave Smith):
  `float h = max(k-abs(a-b),0)/k; return min(a,b) - h*h*k*0.25;`(二次,C1;三次版 `- h*h*h*k/6` 给 C2)。
- smin 还能返回 **blend factor**(`.y`)用于在融合区**插值两个形状的材质** —— 动画里直接拿来做"形变时 A 材质→B 材质"过渡。
- **`mix(sdA, sdB, t)`**:两个距离场线性插值 → 形 A 渐变成形 B(连"字 A→字 B"都能做)。
- 动画含义:**动 `k` 或 `t` = 形变 / 融合 / 分裂**。来源:[smooth minimum](https://iquilezles.org/articles/smin/)。

### 1.3 缓动 = IQ 的"useful functions"(纯标量曲线库)
IQ 专门一篇列了反复用到、但语言不自带的曲线——**这就是动画系统的 curve 库**(纯函数、无状态、可组合):
- `smoothstep`(自带)、`almostUnitIdentity(x)=x*x*(2-x)`(像 smoothstep 但末端导数=1)、`almostIdentity`(软裁剪不引入断点)。
- `integralSmoothstep(x,T)`:smoothstep 当**速度**时的**位置积分**(平滑加速到匀速,做入场位移很顺)。
- 冲量:`expImpulse`、`quaImpulse`/`polyImpulse`(快升慢降,做触发/包络)、`expSustainedImpulse`(可分别控攻击/释放)。
- `cubicPulse(c,w,x)`(替代 `smoothstep(c-w,c,x)-smoothstep(c,c+w,x)`,隔离特征 / 当廉价高斯)。
- `expStep(x,k,n)`(任意陡的 S 形,逼近 step)、`gain(x,k)`(两端扩张中间压缩的 S 形,RSL 经典)、`parabola`/`pcurve`(0→1 两端归零,做"出现-消失"包络/叶片眼睛形)、`sinc`(带回弹 bounce)。
来源:[useful functions](https://iquilezles.org/articles/functions/)。**要点:缓动不是一个 enum,而是一族可参数化的纯函数**——ease-in 只是其中一条(`pow`/`gain`/`smoothstep` 取一)。

### 1.4 距离带效果(distance-band)= 描边/辉光/AO/阴影 的"画上去"动画
SDF 的"边一圈"是 `abs(d)`/`d` 的带:outline = `smoothstep(w, 0, abs(d))`,glow/AO = 距离的指数衰减(本仓 `panel.wgsl` AO 已是),soft shadow = raymarch 沿途最近距离。**动带宽/强度 = 描边逐渐画上、辉光呼吸、AO 渐显**。

### 1.5 反走样 = 动画的"模糊味道"
`smoothstep(edge-w, edge+w, d)`,`w≈fwidth(d)` → 任意缩放锐。**把 `w` 动起来 = 真正的"场模糊→聚焦"淡入**(不同于 alpha 交叉淡);IQ 还有 filtered/积分版(`smoothstep` 积分、`fcos`)做更准的抗锯齿。

### 1.6 LYGIA:把 IQ/Shadertoy 这套**打包成可 `#include` 的多语言库**(含 WGSL)

[LYGIA](https://lygia.xyz)(Patricio Gonzalez Vivo)= 上面所有零件的"电池already-included"版:**极granular(一函数一文件)、多语言(GLSL/HLSL/MSL/**WGSL**/TSL/CUDA/OSL)、`#include` 组合、`#define` 可配**。对我们最相关的是它**有 WGSL + WebGPU resolver**(你正是 wgpu),且模块划分几乎就是本报告的分层:

| LYGIA 模块 | 对应本报告 | 内容 |
|---|---|---|
| `animation/`(easing) | §1.3 缓动层 | Penner 缓动全家桶(quad/cubic/expo/back/elastic/bounce…)——直接是 curve 库 |
| `sdf/` | §1.1/1.2 形状层 | 2D/3D 距离场原语 + 组合算子(`opUnion/opSmoothUnion/opSubtraction/opRound/opElongate/opOnion`,多承 IQ) |
| `space/`(rotate/scale/ratio) | §1.1 域操作 | 变换采样点 = 运动 |
| `draw/`(stroke/fill/digits/flow) | §1.4 距离带 | 描边/填充/流动 |
| `morphological/`(dilation/erosion/alpha·poisson fill) | §1.5 / 阈值 | 膨胀/腐蚀 = "墨水渗入/化开"那味的 fade |
| `generative/`(noise) | §1.1 warp / mask | 域扭曲、溶解 mask |
| `math/`、`filter/`(blur) | §1.3 / §1.5 | mix/map、模糊带 |

**两个必须注意(对落地很关键)**:
1. **许可 = 双授权:Prosperity License(非商业免费)+ Patron License(给 sponsor/contributor)**。即**商用要么赞助/贡献(给 PR 即自动获 Patron 商用授权)、要么买永久商用授权**,否则只能非商业/评估用。→ 沿用本仓既定准则:**LYGIA 借"技法"不抄"代码"**;真要进(可能商用的)产品,优先回到 **IQ 原文片段**(§1.2/1.3 已引,授权更宽、多为公开),或赞助/贡献换 Patron。
2. **WGSL 覆盖不如 GLSL 完整**:README 自承"跨语言 parity 是大挑战、需贡献"。所以**用前先确认该函数有 WGSL 版**,缺的得自己照 IQ 原理移植。

**定位**:LYGIA 给的是 **§1/§2 的叶子函数(尤其 WGSL 缓动 + sdf 算子)现成清单**——省去手抄 IQ;但它**和 Shadertoy 一样只是 per-pixel 着色原子,没有"元素动画系统"**(无 per-instance 描述、无身份、无声明式 profile、无控制层)。即 §3–§5 的"系统层"仍是你的。

## 2. Shadertoy 的范式(idiom,非正式系统)

把上面零件组装成动画,社区高度收敛到几条 idiom:

1. **一切皆 `f(p, t)` 纯函数、无状态**。`map(p)`/`scene(p)` 返回距离(+材质 id);动画 = 把 `iTime` 喂进 `scene`,**同一函数 t 前进**即动。没有"动画对象",没有 retained 状态——**声明式、确定性、可重放**(天然对齐你的 R8)。
2. **`iTime` 穿线 + `fract`/`mod` 循环**:`t = fract(iTime/T)` 做循环;`mod(p, c)` 做空间重复。
3. **分层合成 + 按最近着色**:对每个图元算 SDF,`d = min(d, di)` 合并,记录"最近那个"的 material id → 着色。动画 = 各图元的 transform/param 各自吃 `t`。
4. **统一渲染收口**:`col = mix(bg, shape_col, smoothstep(px, -px, d))`,`px=fwidth(d)`。**所有动画都落在"喂进 scene 的参数"上**,渲染收口不变。
5. **缓动用 §1.3 的纯函数**:`t2 = smoothstep/gain/pow(t)`。

**不优雅之处(关键缺口)**:Shadertoy 是**单场景手写**——`scene()` 里把每个形状的运动硬编码进去,**没有"元素/复用/声明每元素动画"的系统**。它是"一段 demo",不是"一个 UI 框架的动画子系统"。所以直接抄不能解决你"海量元素、各自声明动画"的需求。

## 3. 提炼出的"优雅范式"(可迁移的结构)

从 IQ + Shadertoy 抽出的、值得做成系统的不变式:

- **A. 场 = 参数的纯函数,无状态**:`field = sd(p; params)`,`params = g(now - start)`。动画状态 = 仅 `start`(+ 时长/曲线);其余每帧由纯函数重算。→ 确定性、可重放、可 seek(你的 transport / seek_reveal 正是这个)。
- **B. 动画 = tween 场参数,不是顶点/alpha**:可动的"属性"是 SDF 词汇——**采样点变换 / 阈值(isoline) / 带宽(羽化) / outline·glow·ao 带 / 形状 blend(smin·mix) / 裁剪 mask / 颜色**。
- **C. 正交分解**:`shape(SDF) × material(by gradient/id) × motion(params(t)) × easing(纯曲线)`。四者独立组合 → 加效果 = 加一条,不改管线。
- **D. 缓动 = 可参数化纯函数族**(IQ functions),不是单一 enum。
- **E. 组合算子是"动画基元"**:`min/max/smin/mix/warp/rep` 本身可被 t 参数化 → 融合、分裂、扭曲、生长都是"动算子参数"。

## 4. 落到 UI/文字的"元素动画系统"(把 demo idiom 变成系统)

桥接 Shadertoy 纯函数范式 ↔ 你要的"每元素声明动画" ↔ GPU 实例化:

**核心映射**:Shadertoy 的 `scene(p, iTime)` 是**一个**全局函数;UI 要的是**每元素一份** `field(p; params_i(now - start_i))`。把"穿线的 iTime + 手写参数"换成 **per-instance 动画描述**,在 shader 里求值:

```
每实例(挂 0020 节点 key)携:
  { start, dur, curve_id, 目标属性, from, to }   // 控制层:抄 GPUI/Flutter 的声明式
shader 内:
  t   = clamp((now - start)/dur, 0, 1)
  e   = ease(curve_id, t)                          // §1.3 曲线库,GPU 实现
  param = mix(from, to, e)                          // tween
  应用到 SDF 求值:变换 p / 移阈值 / 调带宽 / blend / mask / 颜色
```

- **控制层**(何时/多久/什么曲线/绑哪)= GPUI/Flutter 那套声明式,按 **0020 node key** 续身份(你是保留模式,不需要 egui/GPUI 的 id-side-table hack)。
- **值层** = §3.B 的 SDF 参数(不是 mesh 的顶点+alpha)。
- **缓动** = §1.3 曲线库 → 一张 `curve_id` 小表(smoothstep / gain / pow / expImpulse / cubicPulse …),shader 端实现。
- **编排/交错**(header 先于 body、逐行)= 每实例不同 `start`(= 你 `resolve` 算的 `delay_ms`);per-element 动画 = 上面那条。**"何时(stagger)" 与 "怎么动(profile)" 分离**,和 Flutter staggered/Interval 同构。
- **统一现有零件**:`glyph.wgsl` 的 alpha-fade、`panel.wgsl` 的 AO 带 + `reveal` mask、`0016 morph` 的解析参数 lerp —— **都是这套的特例**,收编进同一 per-instance 求值。

**SDF 特有约束(必须设计进去)**:
- **采样场(字)vs 解析场(面板)算子集不同**:字支持 变换/阈值/带宽/outline/mask,`mix(sdfA,sdfB,t)` 还能做字↔字 morph;**面板**支持全套解析 `smin/mix` 形变。系统按"源类型"约束合法属性集。
- **参数必须到片元**:per-instance 顶点属性(轻量)或 storage buffer(变长,像 panel params)。
- **GPU 求值而非 CPU 重建**:Shadertoy/GPUI/Flutter 都每帧 CPU 重算;你海量字应把 tween 放 shader(per-instance 描述 → GPU evaluate),CPU 只在内容变化时更新描述。

## 5. 推荐架构(给后续 ADR 的骨架)

一套 **SDF 节点动画系统**:
1. **可动属性集**(SDF 词汇,分 sampled/analytic):`transform(p) / threshold / band / outline / glow / ao / blend(k|mix t) / mask / tint`。
2. **动画描述** `Anim{ target, from, to, start, dur, curve_id }`,挂 0020 node key;一个节点可叠多条(opacity+scale+...)。
3. **曲线库**(§1.3,GPU 实现)+ `curve_id`。
4. **声明式 profile**:reveal 风格 / 节点 kind → 一组进场 `Anim`(像 Flutter AnimatedX / 你 0019 "加一条");编排交错 = `resolve` 给 `start`。
5. **per-instance → shader 求值**:描述进 instance/storage;`glyph.wgsl`/`panel.wgsl` 内 `t→ease→mix→SDF 参数`。统一 0016 morph(几何参数)+ reveal(mask/threshold)+ 装饰(band)。

**取舍 / 风险**:
- 别做成通用关键帧引擎——先固定**小属性集 + 几条曲线**(AnimatedX 用这点覆盖 90%)。
- 字的 `mix(sdfA,sdfB,t)` morph 需要两个字形在同一 atlas/可同时采样,成本高,**后置**。
- 确定性:`start` 走注入时钟(R8),曲线纯函数 → seek/重放稳。
- storage vs instance 属性:变长效果(多条 Anim)用 storage,定长用 instance。

> 一句话:**IQ/Shadertoy 没给"动画系统",但给了它的两层灵魂——"场 = 参数的纯函数(无状态、可重放)"和"一族可组合的算子(smin/mix/warp)+ 缓动曲线"。优雅范式 = 正交分解 shape×material×motion×easing,动画只 tween 场参数。** 把它 + 声明式控制层(按 0020 key)+ per-instance GPU 求值合起来,就是 SDF 世界里的元素动画系统;你的 glyph fade / panel AO·reveal / 0016 morph 都是它的特例。

---

参考:IQ [useful functions(缓动库)](https://iquilezles.org/articles/functions/) · [smooth minimum(smin/morph)](https://iquilezles.org/articles/smin/) · [2D distance functions](https://iquilezles.org/articles/distfunctions2d/) · [domain warping](https://iquilezles.org/articles/warp/) · [distance functions(域操作/组合)](https://iquilezles.org/articles/distfunctions/) · [Shadertoy](https://www.shadertoy.com/) · [LYGIA(多语言含 WGSL,Prosperity+Patron 双授权)](https://lygia.xyz) · [LYGIA license](https://lygia.xyz/license) · GPUI/Flutter 控制层见对话(立即/保留模式动画)。落点:统一 [0016 morph](../decision/0016-streaming-morph-render-model.md) + reveal([0019](../decision/0019-reveal-gating-and-choreography.md)/Plan 9)+ 装饰([0018](../decision/0018-sdf-panel-decoration-primitive.md))为一套节点动画系统。
