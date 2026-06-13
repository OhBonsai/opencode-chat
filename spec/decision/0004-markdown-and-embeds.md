# 决策记录 0004:Markdown 语义层、布局接缝与嵌入块(图片/Mermaid)

- 日期:2026-06-13
- 状态:已采纳(原型验证前)
- 前置:0001(整体架构)、0002(事件管线 + 状态机 + 效果开关)、0003(容错 + 降级)
- 范围:markdown 解析层选型、与 pretext 的接缝、增量策略、代码高亮、图片与 Mermaid 嵌入

## 1. 背景

opencode 的消息正文是 markdown(text part / reasoning part 累积的原文)。
`/Users/wp/w/agentscode/jcode` 已实现 Rust 侧 markdown 渲染,含图片与 mermaid。
本决策确定如何把它融入 wasm+wgpu 画布,而不破坏 0001 的布局选型(pretext,
否决 cosmic-text)。

## 2. 三层分工

```
jcode 管"是什么"(语义)  →  pretext 管"多宽多高在哪"(度量)  →  wgpu 管"怎么显示"(效果)
```

这是各司其职、零冲突的关键划分。markdown 解析输出语义角色而非像素;
样式解析(role → 字体/颜色)、布局、效果分属后两层。

## 3. 采用 `jcode-render-core`(vendor 进来)

该 crate 纯净、wasm-safe(仅依赖 pulldown-cmark + serde + unicode-width,
零阻塞依赖)。入口:

```rust
pub fn parse_markdown(text: &str) -> Document

pub struct Document { pub blocks: Vec<Block> }
pub enum BlockKind {
    Paragraph, Heading{level:u8}, CodeBlock{language:Option<String>},
    BlockQuote, ListItem{ordered:bool, depth:usize}, Table,
    MathDisplay, ThematicBreak, Html,
}
pub struct StyledSpan { text:String, role:StyleRole, fill:FillRole, attrs:TextAttrs }
pub enum StyleRole { Text, Dim, Strong, Code, Link, Html, Reasoning, Math }
```

要点:输出**语义角色**(Strong/Code/Link)而非颜色坐标——与 0002 §5.1
"效果是数据"同构。0001/0002 里"markdown 按块增量解析(pulldown-cmark)"这一环
此前未具体化,jcode-render-core 即其成品,vendor 复用,省去重写。

**不复用**:`jcode-tui-markdown`(绑死 ratatui)、`jcode-tui-mermaid`
(std::fs + resvg + ratatui-image,wasm 不可用)。其布局(TUI 用 unicode-width、
desktop 用 cosmic-text)均不要——前者是终端列宽,后者违反 0001 否决 cosmic-text。

## 4. 与 pretext 的接缝(核心)

```
part.text(累积的 markdown 原文)
  → jcode-render-core parse → Document(语义块 + StyledSpan)
  → StyledSpan 映射成 pretext rich-inline fragment(role+attrs → font)
  → pretext layout → 带位置的 run
  → glyph instance + spawn_time → wgpu(0001 §2.3 管线)
```

- pretext 的 `rich-inline` 吃 `{text, font}` 分片,`break:'never'` / `extraWidth`
  处理 code chip 等原子项;`StyledSpan → {text, font}` 是天然映射
- **pretext 仍是唯一布局器**:jcode 不参与度量,role+attrs 仅决定字体选择
- 接进 0002 每帧管线第 3 步(排版)之前,先做 markdown parse

## 5. 增量策略(拿设计,弃代码)

jcode 的 `IncrementalMarkdownRenderer` 以"最后一个完整块为 checkpoint,只重渲染
其后"——与 0002"块边界=缓存边界、只重排尾部块"同一机制,但实现绑 ratatui。
故:

- 已完成块冻结:布局/glyph instance 进 GPU buffer,不再 parse
- 仅 parse + layout 正在生长的尾部块
- checkpoint = 最后一个完整 markdown 块边界(代码块闭合、段落结束等)

与 0003 一致:历史分页/重连用 catch-up 模式批量 parse,无动画。

### 5.1 尾部块的"主动补全"(借鉴 remend)

尾部生长块里,markdown 语法常半截:`**粗体` 缺尾、` ``` ` 没闭合、`[链接](`
没收尾。两种处理思路互补:

- **hold 区(0006 §3)**:解决"这串字符到底是不是标签/语法"的歧义——悬住等消解
- **主动补全(remend 式)**:解决"已确认是 markdown 但没写完"——不等闭合,
  **临时补一个闭合符**(`**` / ` ``` ` / `$$`)再解析,真闭合到了自然覆盖

Vercel Streamdown 的 `remend` 是这条路的成熟实现(文本级、零依赖、带上下文判断:
`isWithinCodeBlock` 等避免误补)。我们在 jcode parse 之前,对尾部块做一遍同类
补全,避免"原始语法字符闪现 → 闭合到达突然重排"。补全只作用于尾部块的临时副本,
不污染累积原文(原文仍按 SSE 全量对账,0003)。

参照:[industry 调研 §三](../research/industry-llm-chat-rendering.md)。

## 6. 代码高亮的 wasm 注意点

jcode 用 syntect,默认走 oniguruma(C 库,wasm 难编)。方案:

- syntect 启用 `fancy-regex`(纯 Rust)特性,或
- 换 tree-sitter / two-face

语法高亮输出样式 → 与 StyledSpan 同管线(role/attrs),渲染层不变。
按 (code_hash, language) 缓存高亮结果(jcode 同款 LRU 思路)。

## 7. 图片与 Mermaid:降格为"嵌入块"(embed block)

二者不是文字,而是异步产生纹理的块级实体。jcode 的 resvg 光栅化 + std::fs 缓存
+ ratatui-image 全部 wasm 不可用。按 0001"让浏览器做重活"原则改造:

### 7.1 图片

- 浏览器解码(本来就会)→ `copyExternalImageToTexture` 直接进纹理
- wasm 只持尺寸 + 位置;布局上是已知宽高的 opaque box

### 7.2 Mermaid

- `mermaid-rs-renderer`(wasm-safe,纯布局)→ 生成 **SVG 字符串**
- 交浏览器光栅化:SVG blob → `Image` / `OffscreenCanvas` → 纹理
- **不引入 resvg、不打包字体、不碰 fs**
- 按 source hash 缓存,渲染一次;放 Worker 更佳

### 7.3 embed block 的生命周期

- 先占估算高度的占位框(spinner 或已知宽高比)
- 纹理异步到达 → swap
- 通常位于流式前沿之下或预留高度,reflow 影响可控
- 套入 0003 live/catch-up:重连时 embed 用 catch-up 直接 settle 到"已渲染"态,
  不重放加载动画
- 是一种 block 级实体,带自己的 FSM(0002):Placeholder → Loading → Ready | Failed

## 7.4 字形 atlas 容量规划

0001 只定了"动态分配 + LRU 淘汰",这里补容量策略(参照 VS Code WebGPU 原型的
预分配 + 懒填,见 [industry 调研 §六](../research/industry-llm-chat-rendering.md)):

- **预分配固定尺寸纹理**(如 2048×2048 或 4096×4096),启动即开,懒填入字形,
  避免运行时频繁扩容/重建 atlas
- **CJK 大字符集的问题**:中文常用字数千,加粗/斜体/不同字号是不同字形,易撑满。
  按 (字体, 字号桶, 字重) 分页;字号用分桶 + GPU 缩放,避免每个字号一份位图
- **多页 atlas**:单纹理满了开第二页(纹理数组),不是淘汰可见字形
- **LRU 淘汰**:屏外且最久未用的字形先淘汰;可见字形钉住不淘汰
- emoji/彩色字形单独 RGBA 页(0001),与单通道字形分开

## 8. 数据流总结

```
SSE text.delta → part.text 累积(原始 markdown)
每帧(尾部脏块):
  parse_markdown(尾部块) → Document
  StyledSpan → pretext fragment → layout → run
  run → glyph instance(spawn_time)
  embed(image/mermaid)→ 占位框 + 异步纹理(浏览器光栅化)
  render(instanced draw + embed quad)
```

层次归属:
- markdown 语义(jcode-render-core):纯 Rust,wasm,vendor
- 布局(pretext):JS 侧,唯一布局器(0001 §2.2)
- 高亮(syntect/fancy-regex 或 tree-sitter):Rust,wasm
- 图片/mermaid 光栅化:浏览器(SVG/Image → 纹理),wasm 只持元数据
- 效果(WGSL profile):0002 §5.1
