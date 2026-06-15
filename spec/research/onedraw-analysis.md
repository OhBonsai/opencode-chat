# 研究:OneDraw —— GPU-driven SDF 2D 渲染器(实现分析 + 对本项目参考)

- 日期:2026-06-15
- 对象:[Geolm/onedraw](https://github.com/Geolm/onedraw)(本地 `./onedraw`)+ 其博客(`doc/part1–5.md` = dev.to 系列镜像)
- **许可:zlib**(宽松,**允许商用 + 可改可分发**,仅"勿冒称原作 + 保留声明");**可直接借代码/手法**(与 lygia Prosperity / shadertoy CC-BY-NC 不同——这点很关键)。
- 关联:[0018](../decision/0018-sdf-panel-decoration-primitive.md)(SDF 面板图元)、[0011](../decision/0011-gpu-text-as-sdf-primitive.md)(SDF quad 图元)、`design/thinking.md §4`(SDF 效果层底座)。

---

## 1. 它是什么

**单 draw call 的 GPU-driven SDF 2D 渲染器**(Metal/MetalCPP,C99 API)。一切皆 SDF 形状(box/oriented/disc/ellipse/arc/pie/triangle/blurred-box/textured-quad/char),**不走三角形 tessellation**;默认 AA、曲线天生平滑。目标:把活尽量丢给 GPU、draw call 降到 1、透明度随便用不掉性能。

实测(2440p,659 命令):GPU **1.38ms**,显存 ~59MB(定长 buffer,偏大,可调)。

## 2. 管线(全 GPU,博客 part1–3)

```
CPU: 只 push 命令(8B)+ 参数(float)+ 颜色 + 量化 AABB
 ↓
compute ①:逐区域(REGION 16×16 tiles)层级 binning —— predicate + exclusive scan 保命令序
compute ②:逐 tile(16×16 px)建命令链表(AABB 预筛 + SAT/距离精筛)→ 活跃 tile 列表
 ↓
indirect draw:每个活跃 tile = 1 实例(triangle strip 4 顶点),覆盖该 tile
 ↓
fragment:按 tile 链表**逐命令算 SDF + 着色器内 alpha 混合**(无 framebuffer 读写)
```

## 3. 数据模型(对我们最直接的参考)

- **命令流**:`draw_command` = **8 字节**(`data_index` + 打包 type(6b)/fillmode(2b)/clip_index/extra)。
- **参数**:扁平 `float* draw_data`,命令用 `data_index` 索引;**`colors[]` / `quantized_aabb[]` 分开存**(避免 cache thrashing)。
- **量化 AABB**:每命令一个 4 字节(tile 分辨率 16px 量化)→ binning 预筛极快。
- **tile 数据**:`head[] + tile_node{next,cmd_index} 链表 + tile_indices[]`;`counters` 原子分配。
- **clip / group**:`clip_shape`(rect/disc)+ `begin_group/end_group` + `sdf_operator(overwrite/blend)`。

> 这就是 0018 §2 的"**命令 + 扁平 storage buffer + 索引**"数据驱动模式的成熟形态——OneDraw 是它的满血版,我们 0018 是它的最小子集(一个 panel 图元)。

## 4. 关键技术 + 洞察

- **tile binning(16×16)+ 层级 region(16×16 tiles)**:粗筛 region → 细筛 tile,避免 1440p 下 14400 tiles × 65536 命令 ≈ 9.4 亿次裸测试。
- **predicate + exclusive scan 保序**:thread-per-command 并行会丢 2D 绘制顺序 → 用 predicate(可见性 0/1)+ exclusive scan 压实索引保序。Apple Silicon 上 65k 命令 + 已知 SIMD 宽 → **单 pass `simd_prefix_exclusive_sum`**(wave intrinsics)。
- **保守 binning 扩 AABB**:AA 宽度 / smin 的 k / group 全局盒,都把 tile 测试盒**外扩**,防边缘漏 shade(part1)。
- **着色器内 alpha 混合(无 FB 读)**:per-tile 按序混合 → **重度透明零额外成本**(我们 streaming 淡入 + 叠层装饰受益)。
- **形状运算 = 效果层**:`group` + `smin`(IQ 二次多项式版,**连颜色一起 blend**)+ 布尔 + `outline`(整组描边)+ `fillmode`(solid/outline/hollow/gradient)+ clip。**"很多效果从一个图元出"**正是 0018/thinking §4 的愿景。
- **AA**:per-tile 像素级 → **无需 ddx/ddy 算像素长**;过渡约 1px → 用**线性插值而非 smoothstep**;outline 两边都用 AA。
- **Bézier 教训(part3,值钱)**:cubic SDF 数学复杂 + 屏幕空间 float 精度不稳 + bbox 过大难 cull → **放弃 SDF 曲线,改 tessellate 成 capsule**(最便宜 SDF + 天然 cap 无缝);层级 binning 让每 tile 每曲线只评几个 SDF。**结论:曲线别硬上 SDF,拆成廉价 SDF 段**。
- **字体**:pre-build 用 stb_truetype 烘 atlas → BC4 压缩 → 头文件;运行时 `od_draw_char` 推 quad + 字符索引,fragment 采 atlas。**baked atlas**(类我们 bitmap/MSDF),非 SDF 字(作者认为 stb 的距离值有 artifact、无标准)。

## 5. 对本项目的参考(映射到 ADR)

| OneDraw | 我们 | 取舍 |
|---|---|---|
| 命令流 + 扁平 `draw_data` + `data_index` 索引 | **0018 §2 storage buffer + 实例索引** | **直接照搬数据模型**(zlib 可抄);先小后大 |
| `colors`/`aabb`/`data` 分 buffer 防 cache thrash | 我们装饰参数 buffer | 布局借鉴 |
| tile binning + 层级 region | 0018 §11 "tile-based 终极解" | **infinite session 海量装饰的蓝图**;WebGPU-only |
| 着色器内 alpha 混合(无 FB 读) | 叠层装饰 / 淡入 | 值得借(尤其多透明层) |
| group + smin + outline + fillmode + clip | 0018 §6 / thinking §4 效果层 | **"一个图元长出所有效果"的实证**;选中/发光/容器都走它 |
| IQ SDF 库 + 线性 AA + capsule 曲线 | panel.wgsl 的 SDF/AA | 手法照搬(IQ;capsule 曲线教训) |
| baked font atlas(BC4) | 0009/0011 我们已 bitmap/MSDF | 同路,印证 |

**最大收获**:OneDraw = 我们 0018 + §11 tile-based 设想的**完整、可读、zlib 可借**的参考实现。短期不用它那么重,但**数据模型(命令 + 扁平 buffer + 索引)直接照搬**,中长期若装饰/效果爆炸,**tile binning 管线有现成蓝图**。

## 6. 不能直接搬 / 注意

- **Metal 专属**:wave intrinsics(`simd_prefix_exclusive_sum`)、indirect draw、MetalCPP。我们是 **wgpu**:WebGPU 有 compute/indirect/subgroup(部分)→ 可移植但要重写;**WebGL2 无 compute/indirect → OneDraw 全管线不可用**,只能退回**实例化 quad**(= 我们现路 / 0018 v1)。故:**WebGPU 走 tile-binning 满血,WebGL2 走实例化兜底**。
- **文字不同层**:它 baked-atlas 画字;我们有 streaming morph(0016)+ markdown(0017),正交——OneDraw 不解决我们的揭示/动画,只解决"形状/装饰"。
- **定长 buffer → 显存偏大**(59MB);我们要按需裁。
- **2D-only、无我们的 past→current 过渡**:动画/补间仍归 0016;OneDraw 只管"这一帧画什么形状"。

## 7. 采纳建议(分期)

- **短期(0018 v1)**:不上 binning。一个 `panel.wgsl` 实例图元 + 小 storage buffer(命令 + 扁平参数,**照搬 OneDraw 数据模型**)+ IQ `sdRoundBox` + 网格 + 线性 AO/AA。表格框先用。
- **中期**:把零散 `FrameRect` 全收进这个数据驱动图元(0018 §6);效果(选中/发光/group/outline/smin)按 OneDraw 的 fillmode/operator 加参数。
- **长期(WebGPU,infinite session 海量装饰)**:照 OneDraw **tile binning + 层级 region + predicate/scan + 着色器内混合 + indirect**,做"单 draw call 画全部装饰/效果";WebGL2 留实例化兜底。
- **可直接借**:数据模型、IQ SDF/smin/AA、capsule 曲线方案、binning 算法(zlib 许可,标注出处)。

---

参考:[OneDraw repo](https://github.com/Geolm/onedraw)(zlib) · 博客 `./onedraw/doc/part1–5.md` · [IQ 2D SDF](https://iquilezles.org/articles/distfunctions2d/) / [smin](https://iquilezles.org/articles/smin/) · 关联 [0018](../decision/0018-sdf-panel-decoration-primitive.md) / [0011](../decision/0011-gpu-text-as-sdf-primitive.md) / `design/thinking.md §4`。
