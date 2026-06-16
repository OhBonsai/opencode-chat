# 决策记录 0021:JS / Rust 边界 + 可配置渲染样式(数据驱动 shader)+ pretext 复杂 layout 预留

- 日期:2026-06-16
- 状态:已采纳(划界原则 + 数据驱动样式定调;落地分相位,见 §8)
- 前置:[0001](0001-canvas-architecture.md)(画布架构 / §2.2 排版在 JS、measureText 为 ground truth、不用 cosmic-text)、[0009](0009-text-rendering-engine.md) / [0011](0011-gpu-text-as-sdf-primitive.md) / [0015](0015-glyph-source-fallback.md)(文字 = GPU SDF,字体度量/栅格走浏览器)、[0018](0018-sdf-panel-decoration-primitive.md)(面板参数走 storage buffer)、[0020](0020-content-node-identity-model.md)(节点身份)、`TableStyle`(Plan 6:表格渲染样式已 setter 化)
- 触发:两件事。① **shader 里颜色写死**(`glyph.wgsl` 的 `style_color` switch、`theme.rs` 常量)——要改观感得重编 wasm,且 web 层调不到。作者要求"**wgsl 可配置,而非写死**"。② 边界在往 JS 偏(style/input/layout 都进了 web 层),需要把**划界原则**钉清楚,并为**将来复杂 layout 重新启用 pretext** 预留缝。

---

## 1. 划界原则(一句话)

> **要用浏览器字体引擎 / 是快迭代的策略·UI → JS;是确定性引擎逻辑 / 跑 GPU → Rust。**

推论:
- **度量与栅格必须在 JS**:字形 advance / 折行 / 像素都依赖系统字体,`measureText` + Canvas2D 是 ground truth,且"零字体打包"是硬约束(0001 §2.2 / 0009)。这是 **layout 在 JS 的唯一正当理由**,也是唯一被特许越过 core 的环节。
- **样式/输入/配置在 JS**:观感 policy 与交互手感要快迭代、不重编 → web 层。
- **机制/编排/身份/动画 + GPU 执行在 Rust**:CR1 可测、确定性可重放、wgpu。

## 2. 现状清单(谁在哪)

| 关注点 | 位置 | 依据 |
|---|---|---|
| 文本测量 / advance / 折行 / CJK 断行 / 表格两趟 / 对齐 | **JS** layout-bridge·placeTable | measureText ground truth、零字体打包 |
| grapheme 切分、字形栅格化(atlas 源) | **JS** Intl.Segmenter / glyph-raster | 浏览器引擎 |
| style 配置 + UI、输入(滚轮/捏合/拖拽) | **JS** style-config/panel、input | policy/UI 快迭代 |
| markdown 解析、吐字节奏、store、块冻结、build_frame | **Rust core** | CR1 可测/确定性 |
| 装饰几何(代码底/引用/hr/标题线/chip)、表格面板组装 | **Rust core** | 由 layout 结果派生 |
| 相机、剔除、morph Scene / 节点身份(0020) | **Rust core** | 机制 |
| atlas 打包/UV、实例 buffer、draw、surface、**shader** | **Rust render** | GPU |
| **颜色值**:文字按角色(glyph.wgsl switch)、装饰(theme.rs) | **Rust(写死)** | ← 本 ADR 要改 |

**问题点**:最后一行——颜色写死在 shader/常量里,既不能 web 层调,也要重编才改。`TableStyle`(Plan 6)已把表格面板色/AO/线宽变成 **engine state + setter + 每帧读**,证明了模式;本 ADR 把它**推广成通则**。

## 3. 决策 A:可配置渲染样式(数据驱动 shader,不写死)

**所有渲染样式值(颜色/线宽/AO/圆角/fade 时长…)= 数据,不是 shader 常量。** core 持有一份 `RenderStyle` 状态(默认 = 现 theme 值),web 层经 wasm setter 实时改,shader 从 **uniform / storage buffer** 按索引取,而非 `switch`/常量。

### 3.1 调色板(文字角色配色)

```rust
/// 角色配色表:下标 = StyleRole::as_u32(),值 = RGBA。默认 = 现 theme/glyph.wgsl 配色。
pub struct Palette { pub role_color: [[f32; 4]; ROLE_COUNT] }
```
- `glyph.wgsl`:删 `style_color` 的 `switch`,改 `color = palette[style]`(小 uniform/storage buffer,~32×vec4)。
- core 持 `Palette`(engine state),wasm `set_palette` / `set_role_color(role, rgba)` setter;backend 把 palette 传 buffer,**改则重传**(小,廉价)。
- 文字色属**渲染类**(每帧采样)→ setter 改完**下一帧即生效,无需重排**。

### 3.2 装饰样式

- 表格:`TableStyle` 已 setter 化(Plan 6),并入本通则。
- 其余装饰(代码底/引用/Alert/hr/标题线/chip)的色/圆角同样收进 `RenderStyle`,`block_decorations` 每帧读(同 `TableStyle` 路径),不再引 `theme.rs` 常量。

### 3.3 两条生效路径(已成立,固化为约定)

| 改的是 | 路径 | 是否重排 |
|---|---|---|
| **布局类**(对齐、折行、间距、字体族 → 改 advance/位置) | 改 config → `refresh_fonts()` 作废排版缓存 | 重排(下帧) |
| **渲染类**(颜色、AO、线宽、圆角、fade) | 改 config → `set_*` setter 推 engine state | **不重排**,下帧即生效 |

判据:**变了字的位置/尺寸 = 布局类;只变像素外观 = 渲染类。**

## 4. 决策 B:pretext 复杂 layout 预留

`layout-bridge.ts` 就是当年给 pretext 留的排版缝(0001 §2.2);pretext 是 JS + measureText,与现管线**同构**。当年否它**是范围问题不是架构问题**(中英 LTR 用不上 BiDi/shaping,且省依赖)。本 ADR **保留这条逃生口并固化契约**,使将来引入只动一个模块。

- **排版契约稳定**(core 不感知实现):入 = 带角色的 run + 宽度(+ 表格 sidecar);出 = 每 grapheme `[x,y,w,h]` + 块高 + 表格面板几何(0018 #5)。pretext 或手搓实现都满足这份契约 → **可热替换,core/render 不动**(0001 §2.2 "排版模块接口可替换")。
- **重新引入触发器**(任一成立才值得):① **RTL / 复杂脚本**(阿拉伯/希伯来/印度系);② 想要维护良好的 **UAX#14 断行 + 词边界** edge case,不愿自维护禁则;③ **跨 WebView 度量一致性**(Tauri:WKWebView vs WebView2)。
- **不引入的代价**:现手搓 `placeTable` + 词折行够中英用;引入 = +依赖/+包体,换断行正确性 + i18n。**纯中英 markdown 默认不引;真做多语言再接,缝现成。**
- 引入时落点:在 `layout-bridge.layout()` 内部把"手搓折行/测量"换成 pretext 调用,**返回同一形状**;`placeTable` 的两趟列宽可复用 pretext 的 advance。其余一律不动。

## 5. 边界不变量(无论怎么偏,守住这些)

1. **content→layout→render 契约**(0001 §2.2)不破:扁平 run 入、定位 glyph + sidecar 出。
2. **AR10**:每帧最多一次跨界批调用(layout 一次、参数 buffer 增量)。
3. **core 确定性可重放**:时间/随机注入,逻辑不依赖 JS 时钟(R8/R9)。
4. **layout 在 JS = 唯一特许例外**;其余"内容/机制/身份/动画"留 core(CR1)。
5. **样式 = 数据**:任何可视样式值都该是 engine state / buffer,不写进 shader 常量(本 ADR §3)。

## 6. 决策小结

- **A**:渲染样式全数据驱动——`Palette`(角色配色)+ `RenderStyle`(装饰色/AO/线宽/圆角/fade),core 持状态、wasm setter、shader 从 buffer 取;`glyph.wgsl` 去 `switch`、`theme.rs` 常量降为**默认值**而非唯一来源。两条生效路径(布局=重排 / 渲染=实时)固化。
- **B**:`layout-bridge` 作为可替换排版缝,**契约固定**,pretext 列为复杂 layout 的预留实现,带明确触发器;默认不引。
- **原则**:§1 划界 + §5 不变量,作为后续"这段代码该放哪"的判据。

**理由**:把"样式写死在 shader"这个唯一的反例消掉后,边界变得自洽——**度量/栅格/policy/UI 在 JS,机制/GPU 在 Rust,样式值是流过边界的数据**;layout 的 JS 归属既被 measureText 护城河正当化,又通过稳定契约保留了 pretext 升级路,不锁死多语言未来。

## 7. 不决定的 / 非目标

- 调色板上 GPU 的具体形态(uniform vs storage buffer)留实现期按容量定(角色少,uniform 即可)。
- 主题切换 / 多主题持久化(本 ADR 只给"可配置"机制,主题预设是上层)。
- RTL / BiDi / 复杂脚本**当前非目标**(触发后才引 pretext)。
- 字体度量本身的 Rust 化(cosmic-text)= 永久否(0001 §2.2 护城河),非本 ADR 重开。

## 8. 落地清单(分相位)

- [ ] core `Palette`(角色配色,默认 = 现配色)+ `RenderStyle`(并入 `TableStyle` 与其余装饰色)+ setter。
- [ ] render:`glyph.wgsl` 去 `style_color` switch → 读 palette buffer;backend 传 palette、改则重传。
- [ ] core:`block_decorations` 非表格装饰色改读 `RenderStyle`(theme.rs 降为默认常量)。
- [ ] wasm:`set_palette`/`set_role_color`/`set_render_style` setter(渲染类,免重排)。
- [ ] web:style 面板加"文字配色 / 装饰配色"分节(实时,走 setter);布局类继续走 `refresh_fonts`。
- [ ] (预留,不实现)`layout-bridge` 契约注释标注 pretext 替换点 + 触发器(§4);`web/package.json` 不引依赖。

---

参考先例:[0001 §2.2](0001-canvas-architecture.md)(排版在 JS / 可替换接口 / measureText 护城河)· [0018](0018-sdf-panel-decoration-primitive.md)(参数 storage buffer,数据驱动 shader 的先例)· [0020](0020-content-node-identity-model.md)(身份层)· `TableStyle`(Plan 6:渲染样式 setter 化的样板)· [pretext](https://github.com/chenglou/pretext)(measureText 为准 + 逐浏览器校准的 TS 排版/shaping,复杂 layout 预留实现)。
