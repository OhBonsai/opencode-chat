# 决策记录 0015:字形源解析与回退链 —— Bitmap / TinySDF / MSDF(可调试切换)

- 日期:2026-06-14
- 状态:**已采纳 + 已落地**(2026-06-15;两轴模型 / 源解析器 / 回退链 / baked MSDF / 调试器切换与逐源统计均就位。唯 §2.5 的「回退 TinySDF 用 LXGW @font-face 子集」走「退而求其次」留尾,见 §4)
- 前置:0009(字体)、0011 §3.3/§3.5(quad/kind/三源)、0012 §4(调试器)、0013(MSDF)、0014
- 来源:需求——两套渲染方案共存可调试切换;MSDF 编译字体(LXGW 常用字,体积可控);未覆盖字 fallback 到 TinySDF/Canvas

## 1. 背景与目标

已有:**Plan 2 Canvas 位图** + **Plan 3 TinySDF**(运行时单通道,从位图)。新需求:

1. **两套方案共存,调试器切换**:位图 vs SDF,A/B 对比。
2. **MSDF 编译字体**:离线把 **LXGW(`lxgw-wenkai-v1.522/LXGWWenKaiMono-Light.ttf`)的常用字集**烘成 MSDF(大字/拐角锐),**整体体积可控**。
3. **回退链**:某字不在 MSDF 烘集里 → fallback 到 **TinySDF**(运行时)→ 再到 **Canvas 位图**。

各源能力(回顾):位图(1× 锐、不可缩放/特效)、TinySDF(可缩放但大字软、任意字、运行时)、MSDF(任意尺度锐含拐角、需轮廓、固定字集)。

## 2. 决策:两轴模型 + 源解析器 + 回退链

### 2.1 两根轴

- **轴 A — 渲染方案(调试器全局切)**:`Bitmap` ↔ `SDF`。这就是"两套共存可切换"。
- **轴 B — SDF 内的源(逐字,带回退)**:`MSDF(LXGW 烘集命中)` → `TinySDF(运行时回退)`。
- emoji/彩字:始终 `RGBA`(与轴正交)。

### 2.2 源解析器(每字一次,在 atlas alloc 处)

```
resolve(glyph, mode):
  Bitmap 模式            → 位图(Canvas2D 覆盖率, kind=0)
  SDF 模式(默认):
     glyph ∈ MSDF 烘集   → MSDF(取 baked atlas, kind=2)
     否则                 → TinySDF(运行时 Canvas2D→EDT, kind=1)   # 回退
  emoji/彩字             → RGBA(kind=3)
```

回退落地:**MSDF 命中走静态 baked 图集;未命中即生成 TinySDF**(和现有 cache-miss 路径一致)。"Canvas 位图"是另一套方案(轴 A),也是最终兜底——`Bitmap` 模式下全部走它。

### 2.3 资源:离线 MSDF(LXGW 常用字)

- 离线用 **`msdf-bmfont-xml`**(npm,**预打包各平台 msdfgen,免编译**;`scripts/bake-msdf.mjs`)对 **LXGW** 烘 **常用字集**(ASCII + GB2312 一级 ~3760 常用汉字 + 常用标点;`TextDecoder('gbk')` 生成)→ `lxgw-msdf.png(RGB,可能多页)` + `lxgw-msdf.json`(**BMFont 格式**:`chars[]`={id, x/y/w/h, xoffset/yoffset, xadvance, page}、`common.lineHeight/base`、`distanceField.fieldType/distanceRange`、`kernings[]`)。`type=msdf`(三通道;msdf-bmfont-xml 仅 msdf/sdf/psdf,无 mtsdf)。
- **coverage = `chars[].id` 集合**;metrics(advance/bbox)来自 BMFont chars。(canonical 的 msdf-atlas-gen 需源码编译,brew 无 formula,故选 npm 版。)
- 体积:~3500 字 × ~40px MSDF cell(RGB)≈ 数 MB,**bounded**;生僻字**零资产**(运行时 TinySDF)。
- ship `web/public/`,**懒加载**(SDF 模式首次需要时拉)。
- 加载时把 coverage(`chars[].id`)建成 Set/bitset → 解析器 O(1) 判命中。

### 2.4 渲染(复用 0011,不新增管线)

- tile `kind` ∈ {0 位图覆盖率, 1 TinySDF, 2 MSDF, 3 RGBA}(0011 §3.5),片元按 kind 分支:位图 `cov=r`;TinySDF `smoothstep`;MSDF `median(r,g,b)` 再 smoothstep;RGBA 直采。
- atlas:**MSDF baked = 静态页**(启动/懒加载灌入)+ **R8 动态页**(位图/TinySDF 运行时,LRU)+ RGBA 页;实例 `layer` 选页、`kind` 选采样。

### 2.5 度量一致性(实测 bug + 完整解法,2026-06-15)

**实测现象**:glyph mode=msdf 时英文字距乱("us er""to day")。**根因**:排版 advance 用**系统字体** `measureText`,渲染字形却是 **LXGW Mono**(等宽)→ 量宽字体 ≠ 渲染字体,逐字错位。两层具体坑:① `msdf.ts` 把 BMFont 的 `xadvance` **丢了**(没收进来),LXGW 步进根本拿不到;② `layout.advanceFor` 永远走系统字体,不跟随 glyph 源。

**核心不变量**:**逐字,advance 度量 == 渲染该字形的那个字体;且"该字走哪个源"只决策一次,layout(量宽)与 render(取 tile)共用同一决策**(两处分叉 = 新坑)。

**完整解法(系统,不留坑):**

1. **单一源决策**:`(glyphMode, coverage, weight)` → 该字的 source(bitmap / tinysdf / msdf)算一次。layout 据此选**量宽来源**,render 据此选 **tile**。二者用**同一 coverage 集 + 同一规则**(coverage 来自 baked json,JS/Rust 共享一份;规则文档化,改一处同步另一处)。
2. **逐源 advance**(量宽 == 渲染源):
   - **MSDF**(命中 + 常规字重)→ baked `xadvance * FONT_SIZE/bakedSize * roleScale`(LXGW,精确)。
   - **TinySDF / 位图**(未命中 / bold·italic·code / bitmap 模式)→ `measureText(对应字体)`(系统/preset),因为这些字形本就由该字体光栅。
   - 二者**逐字匹配各自源** → 混排不错位(同一行可混 LXGW 与系统字,各按各的步进,无重叠/空隙)。
3. **修数据丢失**:`msdf.ts` 收 `xadvance`,暴露 `msdfAdvancePx(cp)` + coverage(`chars[].id`)给 layout。
4. **MSDF 单字重**:baked 仅 LXGW-Light(无 bold/italic)→ **bold/italic/code 角色不走 MSDF**,落 TinySDF/系统(measureText 其字体)。coverage 判定含字重。
5. **基线/竖直**(实测 caveat 2:字偏高/低):MSDF 字的竖直用 baked `yoffset`/`base`/`lineHeight` 对齐行基线(render 侧 `msdf_instance` 的竖直项),勿与 TinySDF 的 cell 几何混用。
6. **quad 几何**:MSDF 字的 quad 用 **baked cell**(`width/height/xoffset/yoffset`),非 TinySDF 的 `TILE_PX` 方 cell。
7. **缓存失效**:切 glyph mode、或 MSDF 懒加载完成 → advance 变 → **全量 layout 缓存失效 + 重排**(复用 `refresh_fonts`;block-freeze 必须一起清)。

**No-loose-ends 清单**:
- [ ] `msdf.ts` 收 `xadvance` + 暴露 `msdfAdvancePx`/coverage(③)
- [ ] `layout.advanceFor` 逐源选量宽(②),`setLayoutGlyphMode` 跟随 glyph mode
- [ ] 切 mode / msdf 加载完 → `refresh_fonts` 重排(⑦)
- [ ] bold/italic/code 不走 MSDF(④)
- [ ] MSDF quad 用 baked cell + baseline 用 baked yoffset(⑤⑥,render 侧)
- [ ] layout 的源决策与 Rust resolver **同 coverage 同规则**(①,防分叉)

> 备注:此前"回退也统一 LXGW(@font-face 子集 woff2)"非必需——逐源匹配(MSDF→baked xadvance、回退→系统 measureText)已让**间距正确且零额外字体加载**;生僻回退字仅**字形**用系统字体(轻微、罕见),要连字形也统一再上 LXGW 子集 woff2(可选升级)。本条**纠正 0009/0011 的"正文系统字体"**为:**正文 = 当前 glyph 源对应字体,逐字一致**。

### 2.6 调试器(0012 / plan4 4C)

- setter `set_glyph_mode(mode)`:`Auto`(MSDF→TinySDF 回退)/ `Bitmap` / `ForceTinySDF`(禁 MSDF,验回退)/ `ForceMSDF`(看覆盖空洞)。
- `FrameStats` 加**逐源计数** `{msdf, tinysdf, bitmap, rgba}` → DOM 面板显示 **MSDF 命中率**,用来调烘集。

## 3. 不变量与影响

- **契约/kind/atlas/片元不变**(0011);本 ADR 只新增 **① 源解析策略 ② 全局 mode(调试器)③ LXGW MSDF baked 资源 + coverage 判定 ④ 逐源统计**。
- **体积可控**:MSDF 仅常用字集(数 MB,懒加载);生僻字零资产。
- **正文字体统一 LXGW**(2.5):比"系统字体栈"一致,但要打包 LXGW(MSDF atlas + 回退用子集 woff2);权衡见 2.5。

## 4. 落地拆解(实现状态 2026-06-15)

1. **离线 baking 工具** ✅:`scripts/bake-msdf.mjs`(`msdf-bmfont-xml`)→ LXGW 常用字集 → BMFont png+json,可重跑。烘焙产物默认 gitignore(~11MB,可复现),本地 dev 直接服务;要 out-of-box 翻 `.gitignore` 几行。
2. **资源加载** ✅:`web/src/msdf.ts` fetch BMFont json → 紧凑 typed array(ids/cells)+ 解码 PNG 页 → `ChatCanvas.load_msdf`;render `MsdfAtlas`(RGBA D2Array 静态页)灌入 + 重建 bind group。懒加载(切到 MSDF 模式或 `?msdf` 时拉)。
3. **源解析器 + 回退** ✅:`GpuSink::resolve` 按 mode/coverage 选源——MSDF 命中(单码点 ∈ 烘集)→ baked;否则 Auto 回退 TinySDF / ForceMSDF 留空洞;Bitmap 全位图。MSDF quad 几何由方格 + BMFont metrics 反推(`msdf_instance`)。
4. **回退 TinySDF 用 LXGW @font-face**(子集 woff2)❌ **留尾**:当前回退 TinySDF/位图仍用系统字体栈,advance 由 system measureText 给 → MSDF(LXGW)字形落在系统度量槽位(§2.5「退而求其次」;LXGW 等宽集影响小)。要精确一致需烘一份 LXGW 子集 woff2 + 正文统一指向它(fonttools subset,另起)。
5. **调试器** ✅:`set_glyph_mode`(auto/bitmap/tinysdf/msdf 循环)+ `stats()` 逐源计数 `{bitmap,tinysdf,msdf,rgba}` → 面板 `src B/T/M` 行(MSDF 命中率)。
6. **渲染管线** ✅:`GpuInstance.kind`(0 位图/1 TinySDF/2 MSDF/3 RGBA),glyph.wgsl 统一控制流采两源后按 kind 选(median(rgb) for MSDF)。`TILE_PX=128` 对 TinySDF 仍有效;MSDF 走 baked 不受此限。

## 5. 重新评估触发

- 烘集命中率太低(大量回退到软 TinySDF)→ 扩充烘集 / 改运行时 MSDF(fdsm + LXGW .ttf,0013)。
- 不想打包 LXGW(回到小包体优先)→ 退回"系统字体 + TinySDF",放弃 MSDF 锐度(0009/0011 原路)。

## 7. 彩色 emoji 落地 + 特殊字符路由(2026-06-16)

§2 设计里 `RGBA(kind=3)` 一直是"占位":动态图集是 R8、shader `case 3` 是桩、栅格器只取 alpha → **彩色 emoji 拿不到颜色,MSDF 模式下还直接消失**(ForceMSDF 留空洞)。本节定彩字落地 + 把"类图标 Unicode"分流说清。

### 7.1 三类"特殊/类图标字符"的归宿(策略)

1. **markdown 自身的结构标记**(列表 `•`/序号、引用符等)→ **改画成 SDF 形状图元**(走 [0018](0018-sdf-panel-decoration-primitive.md) 面板/形状,跟随文字色、任意缩放锐利),**不走字体字形**。属内容层决定(由 role 派生),不进字形管线。**留作 list 渲染时做**(本 ADR 不实现)。
2. **彩色 emoji**(🏆🎯🥇,Emoji_Presentation / ZWJ / 旗帜)→ **RGBA 彩色路**(§7.2)。
3. **其余单色符号 + 生僻 CJK**(▲ ● ★ • 巅…)→ 现有 **SDF 链**:MSDF 命中用 baked,否则 **Auto 模式回退 TinySDF**(运行时栅格,单色 + 文字色 tint,本就正确)。**注:ForceMSDF 是调试模式,故意留空洞看覆盖率;Auto(默认)早已回退,不会消失。**

### 7.2 彩色 emoji = 动态图集升 RGBA8(最小改面)

不另起图集/绑定:**把现有动态图集 `SdfAtlas` 的纹理格式 R8 → RGBA8**,emoji 复用现成 `Source::Raster` 路(kind=3):

- **栅格器**(`glyph-raster.ts`)统一返回 `TILE²×4` RGBA:kind 0/1 把覆盖率/SDF 值塞进 `.r`(`[v,v,v,255]`,shader 仍读 `.r`);**kind 3 直接返回 canvas 的彩色 `getImageData`**(emoji 字体本就彩,`fillStyle` 无关)。
- **图集**:`SdfAtlas` 纹理 `Rgba8Unorm`、`upload` 字节数 ×4、`bytes_per_row = TILE_PX*4`。绑定层不变(`texture_2d_array<f32>` 对 R8/RGBA8 通吃);内存上限 8MB→32MB(动态页小,可接受)。
- **shader**(`glyph.wgsl`)`case 3u` → `vec4(r8.rgb, r8.a * in.alpha)`(直采彩色,fade 走 alpha;ALPHA_BLENDING 直 alpha 合成)。kind 0/1/2 不变(读 `.r` / median)。
- **路由**:`GpuSink::resolve` 开头加 `if is_color_emoji(cluster) { return Raster(KIND_RGBA) }`(emoji 不进 MSDF)。逐源统计 `src_counts[3]` 已留位。
- **不新增**:无新绑定/新图集结构/新管线;`Source` 枚举不变(emoji = `Raster(3)`)。

### 7.3 快判定(省性能,不栅格)

emoji 判定走 **Rust 码点区间谓词**(在 `resolve` 热路径,O(码点数),**不先栅格再验色**):
```
is_color_emoji(cluster):
  对每个 scalar c:
    c == U+FE0F(VS16)        → true   # 显式 emoji 呈现
    c == U+200D(ZWJ)         → true   # emoji 连写序列
    U+1F000..=U+1FAFF 含 c    → true   # 象形/表情/符号扩展 plane(主力)
    U+1F1E6..=U+1F1FF 含 c    → true   # 区域指示符(旗帜)
  否则 false
```
- 覆盖主力 emoji + VS16/ZWJ/旗帜;**故意不收 ★(U+2605)/▲(U+25B2)/•(U+2022)**——它们无默认 emoji 呈现,留单色 SDF + 文字色 tint(否则会被当 RGBA 输出成灰、丢 tint)。
- **v1 缺口**:U+2600–26FF 区里少数默认彩 emoji(⛄⌚☔ 等)本谓词漏判 → 回退单色;需要再按 Emoji_Presentation 精确表补,代价是带张属性表。当前内容(🏆🎯🥇)全在 1F3xx,命中。

### 7.4 落地(2026-06-16)

- [x] `atlas.rs` `SdfAtlas` → `Rgba8Unorm` + upload ×4 / `bytes_per_row*4`。
- [x] `glyph.wgsl` `case 3u` 直采彩色。
- [x] `glyph-raster.ts` 统一返回 RGBA(emoji 彩、其余 splat `.r`)。
- [x] `lib.rs` `KIND_RGBA` + `is_color_emoji` + `resolve` emoji 分支。
- [ ] (留)markdown 结构标记 → SDF 形状(§7.1.1,list 渲染时做);U+2600 块彩 emoji 精确表(§7.3 缺口)。

## 6. 来源 / 链接

- 字体:`lxgw-wenkai-v1.522/LXGWWenKaiMono-Light.ttf`(OFL)
- 相关:0009(字体)、0011 §3.3/§3.5(kind/三源/atlas)、0012 §4(调试器)、0013(MSDF 离线/实时)、[TODO K′](../../TODO.md)、[design/thinking](../design/thinking.md)
- 工具:**`msdf-bmfont-xml`(npm,已用,`scripts/bake-msdf.mjs`)** → BMFont 输出;备选 msdf-atlas-gen(需编译)/ fdsm(运行时 MSDF,0013)
