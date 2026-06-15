# 决策记录 0018:SDF 装饰/面板图元 —— 参数化 shader 框(网格 / AO / 选中)+ 共享 storage buffer

- 日期:2026-06-15
- 状态:已采纳(方向 + 数据传输分档定调;落地分相位,见 §10)
- 前置:0011(文字/装饰 = GPU quad + SDF,借算法不借框架)、0016(past→current 过渡机制)、0014/plan5 §5F(表格像素两趟,产 `colX/rowY`)、`crates/render/src/shaders/rect.wgsl`(现有圆角矩形 SDF 图元)
- 触发:表格框现用 **N 条 `FrameRect`**(外框 + 每行线 + 每列线)画——平、实例多、且 #5 连续竖线难(需 colX)。作者提出:**"table 就是一个带参数的方框,框/横线/竖线/AO/选中全用 fragment shader + SDF 算"**,参数 = 行列(占比)/宽高/选中…。本 ADR 把它定为通用图元。
- 定位:**这是装饰层 + 后续效果层的统一底座**——不止表格,代码块底/引用条/Alert/选中/hover/发光等都从这条路走(设计展开见 `design/thinking.md §4`)。

---

## 1. 决策:新增「SDF 面板图元」

一个 **quad + fragment shader**,按参数**程序化画**:圆角外框 + 横/竖网格线 + AO(内阴影/rim)+ 选中/hover 高亮 + 底色。全用 SDF(`sd_round_box` + 到最近网格线距离 + smoothstep),**任意缩放清晰**(无边画布硬需求)。

- **文字不进 shader**:字仍走 glyph 管线(表格走 plan5 §5F 的 `placeTable`)。**shader 画容器,glyph 画字**,共用同一 `colX/rowY`(同源)→ 严丝合缝,#5 连续竖线天然解决。
- 是 `rect.wgsl` 的升级泛化:rect = 这个图元的退化情形(无网格、无 AO)。

## 2. 数据传输(关键,分两档)

行列分隔是**变长**数据,塞不进定长 vertex 属性。分档:

### v1 — uniform 打包(定长,够表格用)
- 把**行列占比**(归一化 `[0,1]`,= 列边界/行边界占框宽高的比例)打进定长 uniform:如 `mat4`(16 float)存最多 16 个边界,或几个 `vec4` 数组。**给一个最大容量上限**(如 ≤16 列、≤32 行,超出降级/截断)。
- 占比而非绝对 px → 分辨率无关、resize 不必重传(框 quad 的 size 变,shader 按比例算)。
- 优点:零新管线复杂度(uniform 而已),先把表格漂亮化跑通。

### 升级 — 共享 storage buffer(变长、大容量、增量更新)
- 后续**很多效果都要喂数据**(每块的网格/AO/选中/发光参数)→ 用**一个共享 storage buffer**(WebGPU)装全部面板的参数,**`update` 增量改 buffer、不每帧重传**(脏区写入);图元实例携带"在 buffer 里的偏移/长度"索引。
- 容量不再受 uniform 上限;是装饰/效果层的统一数据通道。
- **WebGL2 兜底**(0011:无 SSBO/compute):storage buffer → **data texture 编码**(把参数写进纹理,shader 采样);uniform v1 路在 WebGL2 直接可用。

> 决策(经 §11 deep research 校准):**v1 直接走小 storage buffer + 增量 `update`**(uniform 装数组有上限、非惯用法,且升级本就走 storage buffer → 避免"先 uniform 再重写";`@builtin(instance_index)` 取每面板参数 = idiomatic 数据驱动);**uniform/data texture 仅作 WebGL2 兜底**。uniform-mat4 占比方案降级为"WebGL2 兜底里的极简实现"。

## 3. 参数契约(effects = 数据,0002 §5.1)

每个面板实例:
```
box(pos, w, h) · radius · colRatios[] · rowRatios[]      // 几何 + 网格
· lineW · lineColor · fillColor · headerFill            // 线/底
· aoRadius · aoStrength                                  // AO
· selCell(r,c) / hoverCell · selColor                    // 选中/hover(SDF 子矩形)
· kind/flags                                             // 退化:纯 rect / 带网格 / …
```
选中/hover/发光全是**参数**(不是代码分支):cell index → SDF 子矩形 → 高亮/抬起/AO。

## 4. fragment 算法(自写,借手法不抄)

- 外框:`sd_round_box(local, halfsz, radius)` + `fwidth` AA(沿用 rect.wgsl)。
- 网格线:`min over colRatios(|x - col*w|)` / 行同理 → `smoothstep(lineW,0,d)` 成线。
- **AO**:到边/线距离做内阴影 darken + 一点 rim light(参考 shadertoy `NX23DV` 的 SDF→AO 手法)。
- 选中:选中 cell 的子矩形 `sd_round_box` → 填充/发光/抬起。
- ⚠ **许可**:shadertoy(默认 CC-BY-NC)/ lygia(Prosperity 非商用)**只借手法、自己写几行**,不直接拷(同 plan5 lygia 约定)。

## 5. 与 0016 过渡同步(框几何也要补间)

列宽随 streaming 揭示变 → 字走 [0016] morph,**框的 `colRatios/box` 也得补间**(否则字滑、框 snap)。两条路:
- (a) shader 收 **past + current 两套参数 + t**,自插值(与 0016 §4.5 路 A 同精神);
- (b) **CPU 每帧把插值后的参数 `update` 进 buffer**(与 0016 路 B/CPU mix 同精神,v1 推荐)。
节奏与 0016 对齐:框过渡 = 0016 的"非 glyph 通道"。

## 6. 推广(装饰层收敛 —— 本 ADR 的真正价值)

不止表格。**现在零散的 `FrameRect`(代码块底 / 引用左条 / Alert 底/条 / hr / 标题线 / 选中)统一迁成「SDF 面板图元」**:每个块装饰 = 一个参数化面板(圆角/边/AO/底色/左条/网格)。后续**发光/hover/选中/容器特效**都从这条路加(加参数 + 几行 SDF,不加图元类型)。这是 0011「装饰 = SDF quad」的收口,也是效果层的入口(`design/thinking.md §4`)。

## 7. 取舍 vs 现 FrameRect 网格

| | 现 FrameRect | SDF 面板图元 |
|---|---|---|
| 观感 | 平(线/底) | AO/圆角/发光/选中,漂亮 |
| 实例数 | 多(每线一 rect) | 一个 quad/面板 |
| 选中/hover/动效 | 难 | SDF 参数,顺手 |
| #5 连续竖线 | 需 colX 额外画 | 天然 |
| 工程 | 已有 | 新图元 + 变长数据(uniform→SSBO)+ WebGL2 兜底 + 0016 过渡接 |

## 8. 边界 / 非目标

- **文字不进 shader**(glyph 管线照旧;shader 只画容器/网格/AO/底)。
- 不做 3D/复杂光照;AO = 2D SDF 近似。
- WebGL2 兜底走 uniform/data texture(无 SSBO/compute)。
- AO/图案手法自写(许可)。

## 9. 决策小结

采纳「**SDF 面板图元**」:quad + 参数化 fragment(网格/AO/选中/圆角/底),**v1 uniform 占比、升级共享 storage buffer + 增量 update**,文字分离、与 0016 过渡同步、WebGL2 兜底;**从表格起步,逐步收编所有块装饰 → 成为装饰/效果层统一底座**。

## 10. 落地清单(分相位)

- [ ] **图元管线**:`shaders/panel.wgsl`(rect.wgsl 升级:网格 + AO + 选中 SDF)+ `PanelInstance`(几何 + 参数索引)。
- [ ] **v1 数据**:行列**占比**写**小 storage buffer**(+ `writeBuffer` 增量改脏区,§11 校准;非 uniform-mat4);表格首用(colRatios/rowRatios 来自 §5F 的 colX/rowY 归一化)。→ 解决 #5 + 表格 AO/圆角观感。WebGL2 兜底走 uniform/data texture。
- [ ] **AO/选中** fragment(自写,借 shadertoy/lygia 手法)。
- [ ] **升级**:共享 storage buffer(WebGPU)+ `update` 增量;WebGL2 data texture 兜底。
- [ ] **0016 过渡接**:框几何 past→current 补间(CPU update 或 shader 双态)。
- [ ] **收编装饰**:代码块底/引用/Alert/hr/选中 迁到面板图元。

## 11. 先例与备选(deep research,2026-06-15)

**本方案不是孤例,而是业界主流"GPU-driven 2D UI"模式**——校准如下:

- **Zed 编辑器(Rust + GPU,最强佐证)**:整套 UI = **参数化 quad 图元 + 实例化 + 每实例参数**,每种图元(quad / shadow / 圆角矩形 / underline / glyph)一个 shader,SDF 画圆角与阴影,120fps。**几乎就是本 ADR**(参数化 SDF 图元 + 数据驱动),且是**已出货产品**验证。[Zed: Leveraging Rust and the GPU…](https://zed.dev/blog/videogame)
- **OneDraw(zlib,可借代码)**:Metal GPU-driven SDF 2D 渲染器 —— 命令流 + 扁平 `draw_data` + 索引 + tile binning + 着色器内混合 + group/smin/outline。**= 本 ADR 的满血参考实现**,数据模型可直接照搬。**详析见 [research/onedraw-analysis](../research/onedraw-analysis.md)**。[repo](https://github.com/Geolm/onedraw) · [nical: GUIs on the GPU](https://nical.github.io/drafts/gui-gpu-notes.html)
- **数据传输**:storage buffer + `@builtin(instance_index)` 取每形参数 = **idiomatic 数据驱动模式**;实例 vertex buffer 适合重复几何;**uniform 不适合数组(有大小上限)**。[webgpufundamentals: storage buffers](https://webgpufundamentals.org/webgpu/lessons/webgpu-storage-buffers.html)

**据此校准你的三点(我不照单全收):**

1. **uniform mat4 v1 → 直接上小 storage buffer 更稳**:uniform 装数组有上限、非惯用法;wgpu 里建个 storage buffer + 实例索引并不更难,且**就是升级要走的同一条路**——建议**跳过 uniform-mat4,直接 storage buffer**(v1 容量给小、`writeBuffer` 改脏区),避免"先 uniform 再重写"。
2. **单 quad 内 fragment 循环 colX[] vs 实例化网格线**:两条都行——
   - **单 panel quad + 片元循环**(你的想法):实例少,但每像素 O(cols) 循环 + 需 tight quad 防 overdraw([overdraw 警告](https://randygaul.github.io/graphics/2025/03/04/2D-Rendering-SDF-and-Atlases.html));表格列少 → 完全够。
   - **Zed 式:线/格各一实例**:无每像素循环,实例多。
   - 选**单 panel quad**(表格场景列数小、且要整面 AO);效果复杂后再拆。
3. **AO/圆角 SDF**:用 Inigo Quilez 的精确 `sdRoundBox` + 距离场做软阴影/AO(成熟公式),自写(许可)。[IQ SDF / GM Shaders SDF](https://mini.gmshaders.com/p/sdf)

**两个"更大的解法"(你说的"别限制自己",记着但暂不上):**

- **tile-based GPU-driven(WebRender/Vello 式)**:compute 把屏幕分 16×16 tile、每 tile 建绘制链表 → 砍 overdraw、扩到百万图元。**这是 infinite session + 海量装饰的终极扩展答案**;**WebGPU 专属**(WebGL2 无 compute)。[SDF tiles](https://gamedev.net/forums/topic/706561-using-sdf-rendering-with-large-world/) · [nical](https://nical.github.io/drafts/gui-gpu-notes.html)
- **Vello(buy 而非 build)**:Rust/wgpu 的 **compute 中心全向量 2D 渲染器**(路径/渐变/文字)。若装饰复杂度爆炸(任意矢量/渐变),可整体外包给它。但:它是**框架**(0011 取向是"借算法不借框架")、体积大、会变成第二个渲染器(与我们 SDF-glyph 管线并存),**默认不引,作复杂度失控时的逃生口**。[Vello](https://github.com/linebender/vello) · [Raph Levien: Fast 2D rendering on GPU](https://raphlinus.github.io/rust/graphics/gpu/2020/06/13/fast-2d-rendering.html)

**结论(校准后)**:方向被 Zed 出货验证,**采纳**;唯一改动是 **v1 直接用小 storage buffer**(别 uniform-mat4)。tile-based 留作扩展、Vello 留作逃生口。§2/§10 据此微调:v1 = 小 storage buffer。

---

参考:[0011](0011-gpu-text-as-sdf-primitive.md)(SDF quad 图元 / WebGL2 无 compute)· [0016](0016-streaming-morph-render-model.md)(框几何过渡)· [0014](0014-table-two-pass-layout.md)/[plan5 §5F](../plan/plan5-streaming-markdown.md)(colX/rowY 来源)· **Zed**(GPU UI 出货验证)· **nical / OneDraw**(数据驱动)· **Vello / Pathfinder**(compute 向量,逃生口)· shadertoy `NX23DV` / IQ SDF / lygia(AO 手法,**自写不抄**,许可)。设计展开见 `design/thinking.md §4`(SDF 效果层底座)。
