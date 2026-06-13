# 业界 LLM 对话渲染调研报告

- 日期:2026-06-13
- 目的:为 Rust + wasm + wgpu 自研对话渲染引擎,梳理业界现状与我们的差异化
- 方法:多角度检索 + 抓取一手来源(官方文档、GitHub issue、工程博客)

---

## 一句话结论

业界几乎**全部在 DOM 上做 LLM 对话**,过去一年的工程进步集中在三件事:
**流式平滑**(把网络突发整流成匀速)、**流式 markdown**(处理未闭合语法的闪烁)、
**重渲染治理**(memo + 批处理 + 虚拟化)。它们正在逼近 DOM 的天花板——而**没有
任何公开的生产/研究案例用 Canvas/WebGL/wasm 渲染流式对话**。这既是我们的机会,
也是警示(前人没走有其原因:选择/无障碍/字体)。

---

## 二、流式渲染:业界主流做法

### 网络层

SSE 是事实标准。关键工程坑被反复记录:Nginx/Cloudflare 默认缓冲会"杀死"流式,
需 `X-Accel-Buffering: no` + `Cache-Control: no-cache, no-transform`;长流需心跳
注释保活;需处理背压。([Chrome 最佳实践](https://developer.chrome.com/docs/ai/render-llm-responses)、
[Streaming 指南](https://dev.to/pockit_tools/the-complete-guide-to-streaming-llm-responses-in-web-applications-from-sse-to-real-time-ui-3534))

opencode 的 SSE 头(`X-Accel-Buffering: no`、心跳)与此完全一致——印证我们 0001
读到的协议设计是业界标准做法。

### 流式平滑(与我们 0002 的"平滑器"同一问题)

**业界已明确认知"按 token 直接上屏会抽搐",并给出标准解法**:

- **缓冲 + 批处理**:token 进 `useRef` 不触发渲染,每 ~50ms flush 一次,把
  100+ 次/秒的重渲染降到 ~20 次/秒。50ms 是公认的"实时感 vs 性能"平衡点。
  ([Akash Kumar](https://akashbuilds.com/blog/chatgpt-stream-text-react))
- **Vercel AI SDK `smoothStream()`**:服务端 TransformStream,按 `delayInMs`
  (默认 10ms)+ `chunking`(word/line/自定义)释放;**支持 `Intl.Segmenter`
  做 CJK 分词**(按词分块对中文会失败,这点对我们重要)。
  ([AI SDK 文档](https://ai-sdk.dev/docs/reference/ai-sdk-core/smooth-stream))
- **前端 rAF 逐字动画**:维护 `parts`(原始)+ `stream`(动画显示)双状态,
  `requestAnimationFrame` + 时间戳按固定速率吐字(常用 ~5ms/字 ≈ 200 字/秒)。
  ([Upstash](https://upstash.com/blog/smooth-streaming))
- 体感数据:流式 UI 被感知为比缓冲快 **~40%**(同样的墙钟时间)。

**对我们的意义**:我们的"平滑器"(0002)正是业界共识的正确方向,但业界是在
JS/DOM 层做 50ms 批处理(粗粒度,本质是"少渲染"),**我们在 GPU 上做逐字符
匀速 + spawn_time 动画(细粒度,本质是"渲染得起")**——这是我们能明确超越的点。
业界的 50ms 批处理是"为了不卡的妥协",我们不需要妥协。

### 打字机/动效库

FlowToken、llm-ui 提供逐字动画 + 平滑;Streamlit 内置 `st.write_stream`。
但都是 DOM/CSS 级动画,**无法做逐字符 shader 效果**——一条 2000 字回复 = 2000 个
动画节点,这正是我们 0000 文章里"问题 2"描述的 DOM 上限。

---

## 三、流式 Markdown:业界最活跃的战场

这正是我们 0004 处理的问题,业界过去一年密集产出方案:

### 核心问题(与我们 0004 §5 / 0006 §3 完全一致)

token 流到一半,markdown 语法未闭合:`**粗体` 缺尾、代码围栏没闭合、`[链接](`
没收尾。传统渲染器(react-markdown/marked/markdown-it)要求完整输入,导致:
原始语法字符闪现、闭合到达时突然重排、悬挂括号/星号。Shiki 高亮 CPU 重,300+ 行
代码块每字符重高亮会冻结 UI。([Streamdown](https://streamdown.ai/docs/termination)、
[HN 讨论](https://news.ycombinator.com/item?id=44182941))

### 主流方案

- **Streamdown(Vercel,事实主导方案)**:react-markdown 的 drop-in 替代,专为
  流式重写。核心:`remend` 预处理器**自动补全未闭合语法**再解析;**块级增量解析,
  只重渲染变化块**;React.memo 块级记忆;后台 worker 对已完成代码块做高亮(零感知
  延迟)。支持 GFM/KaTeX/Mermaid/Shiki。([Streamdown](https://streamdown.ai/docs/memoization)、
  [Vercel 公告](https://vercel.com/changelog/introducing-streamdown))
- **shiki-stream / react-shiki(Anthony Fu)**:增量高亮,`CodeToTokenTransformStream`
  + recall 机制(上下文变化时通知回滚 token);React 18 `startTransition` 把高亮标
  为非紧急。([shiki-stream](https://github.com/antfu/shiki-stream))
- **llm-ui**:帧率级平滑 + 停顿平滑 + 自定义组件 token(`【{type:"buttons"...}】`)。
  ([llm-ui](https://llm-ui.com/))
- **传统解析器**:marked(#3657 承认不支持)、markdown-it、micromark 均**无原生
  partial 模式**(截断时自动闭合 AST),是长期 feature request。

### 与我们方案的对照

| 维度 | 业界(Streamdown) | 我们(0004) |
|---|---|---|
| 未闭合处理 | `remend` 文本级自动补全 | hold 区 + 块边界 checkpoint(0006 §3) |
| 增量粒度 | 块级 memo,只重渲染变化块 | 块级冻结进 GPU buffer,只重排尾部块 |
| 高亮 | 后台 worker / shiki-stream | syntect(fancy-regex)按 (hash,lang) 缓存 |
| 解析器 | pulldown 系(JS) | jcode-render-core(Rust,pulldown-cmark) |

**关键洞察**:业界的"块级 memo + 只重渲染变化块"和我们 0002/0004 的"块边界冻结"
是**同一个收敛**——独立得出同一答案,说明这条路是对的。差异在执行层:它们 memo
React 组件树,我们冻结 GPU buffer。`remend` 自动补全是个我们可以借鉴的具体技巧
(比我们的 hold 区更激进:不等闭合,先补全渲染)。

---

## 四、主流开源对话框架的渲染架构

| 框架 | 技术栈 | 流式 | Markdown | 虚拟化 | 备注 |
|---|---|---|---|---|---|
| **Vercel AI SDK** (useChat) | React | useChat/Server Actions | 外部(荐 Streamdown) | 无 | 需手动 memo,长对话瓶颈 |
| **AI Elements** | React | Streamdown 集成 | **Streamdown(流式原生)** | 无 | 20+ 组件 |
| **assistant-ui** | React + Radix | 可插拔 runtime | 组件化 | 无 | runtime/UI 分层 |
| **Lobe Chat** | React/Next | AgentRuntime.step() | 外部 | 无 | 三层消息实体,富媒体反范式化 |
| **Open WebUI** | **Svelte** | Socket.IO | MarkdownTokens(响应式) | 无 | 细粒度响应式免 memo |
| **HF chat-ui** | **SvelteKit** | OpenAI API + SSR | 外部 | 无 | Svelte 5 runes |
| **LibreChat** | React + Node | **SSE + 可恢复流** | v0.8.4 后转静态 HTML | 无 | Redis 多标签同步,断线续传 |
| **Chatbot UI** | React/Next | useChat | 外部 | 无 | 起步模板,几乎无优化 |
| **Continue** | React webview | IDE 协议 | 行内极简 | 无 | IntelliJ 用 JCEF 离屏渲染 |

**几个跨框架的事实**:

1. **九个框架没有一个做列表虚拟化**。原因被分析为:对话长度通常没到需要虚拟化的
   阈值、且虚拟化的懒渲染伤滚动手感——它们改用 memo + 按需渲染。**但这正是
   长对话卡顿的根因(见第五节)**,所以这是"还没解决"而非"不需要"。
2. **Streamdown 和 Open WebUI 的 MarkdownTokens 是仅有的两个"流式原生"渲染器**,
   其余都靠外部库 / 后处理 / 静态 HTML。
3. **Svelte 系(Open WebUI、HF chat-ui)靠编译期细粒度响应式,天然规避 React 的
   "每 token 全列表重渲染"问题**——React 系则要靠 memo 苦战。这对我们是旁证:
   React 的重渲染模型不适合高频流式,而我们把渲染搬出 React/DOM 正是更彻底的解法。
4. **LibreChat 的"可恢复流"(断线自动重连续传 + Redis 多标签同步)** 是我们 0003
   容错的现成参照,且比 opencode 更进一步(多标签)。
5. **LibreChat 从 react-markdown 退回静态 HTML** 说明流式 markdown 的可靠性在生产
   中仍是痛点,大项目宁可牺牲交互也要稳定。

---

## 五、性能问题:业界公认痛点

### 1. 长对话卡顿(最普遍,且官方未解)

- ChatGPT 长对话变极慢([OpenAI 社区](https://community.openai.com/t/chatgpt-gets-extremely-slow-in-long-browser-chats-any-fix-coming/1133247))
- **Claude Code 官方 issue #24146:为长对话做虚拟滚动/懒加载**——
  说明 Anthropic 自己也还没解决([#24146](https://github.com/anthropics/claude-code/issues/24146))
- 根因:全部消息常驻 DOM,无虚拟化;1000+ 消息撑爆标签页;RAM 无界增长
- 解法:虚拟滚动(react-window/virtuoso/TanStack Virtual),只渲染可见 ~50 条

**对我们**:这是业界**最大的未解痛点**,而 GPU 渲染 + 视口裁剪(0002 §6)天然
解决——我们的"长对话"边际成本接近零,因为屏外内容只是 buffer 里的数字,不是 DOM
节点。**这是我们最硬的差异化。**

### 2. 流式重渲染风暴

每 token 触发整列表 React 重渲染,30 token/秒时 reconciliation 跟不上。解法:
50ms 批处理、`React.memo` 拆分流式消息与历史、`experimental_throttle`、rAF。
([Akash Kumar](https://akashbuilds.com/blog/chatgpt-stream-text-react)、
[SitePoint](https://www.sitepoint.com/streaming-backends-react-controlling-re-render-chaos/))

**对我们**:我们没有 reconciliation,每帧只画可见 instance,这类问题不存在。

### 3. 滚动跟随抖动(我们 0002 §6 / 0005 已设计)

对话是**尾锚定**而非首锚定:末条消息流式增长时,scrollHeight 变但 offset 不变,
用户漂离底部;或被强行拽回底部打断阅读。TanStack Virtual 2026 年专门写了
[《Chat UIs Are Lists Until They Aren't》](https://tanstack.com/blog/tanstack-virtual-chat):
`anchorTo:'end'` + 稳定 `getItemKey` + `followOnAppend` + `scrollEndThreshold:80`
(仅当已在底部才跟随)。`use-stick-to-bottom` 用弹簧动画 + ResizeObserver,并指出
**CSS `overflow-anchor` 在 Safari 不可靠**。([use-stick-to-bottom](https://github.com/stackblitz-labs/use-stick-to-bottom))

**对我们**:opencode 桌面端的"仅底部跟随 + 手势区分"我们已采纳(0002 §6)。
业界与我们结论一致。而且我们的高度是**同步精确**的(0002 §6),不像 DOM 异步测量
需要 90 帧锚底 hack——这是结构性优势。

### 4. Markdown/高亮重解析成本

`marked` 假设完整文档,每 token 重解析整段;`innerHTML +=` 触发整子树重渲染。
解法:流式解析器(streaming-markdown,append 而非 replace,5KB 消息 <5ms)、
块级 memo、`useMemo` 延迟解析、用 `appendChild` 不用 `innerHTML`。
([Chrome](https://developer.chrome.com/docs/ai/render-llm-responses)、
[Vercel cookbook](https://ai-sdk.dev/cookbook/next/markdown-chatbot-with-memoization))

### 5. 虚拟化 × 流式的根本冲突

虚拟列表需测量 item 高度,流式时末条高度持续增长→虚拟化失效。react-virtuoso 的
`VirtuosoMessageList` 专为此设计(变高 + 流式)。但**虚拟化 + 动态高度 + 流式三者
叠加是公认难点**。([Kissflow](https://culture.kissflow.com/chat-virtualization-and-performance-optimization-enhancing-the-user-experience-80b35678a25))

**对我们**:这个"三难"在我们架构里消解——我们的布局高度渲染前就精确已知(pretext
同步测量),不存在 DOM 的异步测量竞态。

---

## 六、Canvas/WebGL/wasm 渲染文字:有人做,但没人做对话

### 谁在 GPU 上渲染文字

- **Figma**:C++ → wasm + WebGL,React 只做 UI 面板。场景图 + 变换矩阵,delta
  更新而非 setState,"百万级状态变化不崩"。被总结为
  ["Figma is a game engine, not a web app"](https://medium.com/@nike_thana/figma-is-a-game-engine-not-a-web-app-how-c-and-wasm-broke-the-react-ceiling-8ed991bea48f)
  ——**这正是我们 0000 的核心论点,有大厂先例背书**。
- **Google Docs**:2021 转 canvas 渲染,因 DOM 处理混合 LTR/RTL 不可靠;**维护
  隐藏离屏 DOM 供屏幕阅读器**。([The New Stack](https://thenewstack.io/google-docs-switches-to-canvas-rendering-sidelining-the-dom/))
- **VS Code / Monaco**:WebGPU 渲染原型,M2 上"滚到顶"帧时间降 30-40%,Windows
  游戏机降 50-70%;纹理图集 glyph,预分配 3000 行×200 列缓冲懒填。比例字体和长行
  仍有挑战。([vscode#221145](https://github.com/microsoft/vscode/issues/221145))
- **Flutter CanvasKit**:Skia → wasm(~1.5MB),像素级一致但包体大。

### 共同踩的坑(对我们是必答题清单)

- **文字选择/复制**:canvas 要手动重实现;Figma/Google Docs 靠隐藏 DOM
- **无障碍**:canvas 对屏幕阅读器默认不可见。解法:隐藏 DOM 镜像(维护成本高)、
  AccessKit、或新的 **HTML-in-Canvas API**(Chrome 148+ origin trial,原生可交互/
  可访问)([Chrome](https://developer.chrome.com/blog/html-in-canvas-origin-trial))
- **IME 输入**:canvas 文字编辑要全自定义
- **字体加载/回退**:canvas 要手动处理——**这正是我们 0001 选 pretext 规避的**
- **包体**:CanvasKit +1.5MB

### Rust/wasm 文字栈(可选依赖)

glyphon(cosmic-text + etagere + wgpu,新项目首选)、cosmic-text(纯 Rust 排版)、
vello(2D 含文字)、wgpu_glyph(旧)、msdfgen(MSDF,任意缩放锐利 + 描边/发光)。
GPU 文字技法:SDF / MSDF / glyph atlas。([redblobgames SDF](https://www.redblobgames.com/articles/sdf-fonts/)、
[CSS-Tricks WebGL 文字](https://css-tricks.com/techniques-for-rendering-text-with-webgl/))

### 关键发现:没有人在 canvas/wasm 上做流式对话

调研**没有找到任何**用 canvas/WebGL/WebGPU 渲染流式 LLM 对话的生产或研究案例。
Chrome 官方最佳实践明确推荐 **DOM + 流式 markdown 解析器**。Eric Ma 的 "Canvas Chat"
(2025)是用 DOM 元素在无限画布上做**非线性对话**,不是 GPU 文字渲染。
([Chrome](https://developer.chrome.com/docs/ai/render-llm-responses)、
[Canvas Chat](https://ericmjl.github.io/blog/2025/12/31/canvas-chat-a-visual-interface-for-thinking-with-llms/))

前人不做的原因(= 我们必须正面解决的):①DOM 原生的选择/复制对聊天关键;
②流式 markdown 注入 DOM 比注入 GPU 容易;③HTML-in-Canvas 成熟前,canvas 无优势
却有一堆无障碍成本。

---

## 七、我们的差异化定位

| 业界现状 | 我们的差异 |
|---|---|
| 流式平滑 = JS 层 50ms 批处理(妥协,粗粒度) | GPU 逐字符匀速 + spawn_time shader(无妥协,细粒度) |
| 逐字动效 = DOM/CSS,2000 字 = 2000 节点 | GPU instance,2000 字 = 小菜一碟 |
| 长对话卡顿 = 业界最大未解痛点(含 Claude Code 官方) | 视口裁剪,屏外是 buffer 数字,边际成本≈0 |
| 流式重渲染 = memo/批处理苦战 | 无 reconciliation,每帧只画可见 |
| 滚动锚定 = DOM 异步测量,需 hack | 高度同步精确,一次到位 |
| markdown 块级 memo(React 组件) | 块级冻结(GPU buffer)——同收敛,更彻底 |
| 字体 = canvas 方案的老大难 | pretext 借浏览器字体引擎,规避整类问题 |

**机会**:长对话性能 + 逐字符高动效,是 DOM 派系结构性做不好、而我们结构性做得好
的两块,且**无人在对话场景试过 GPU**——空白市场。

**风险(业界血泪,必须正面回应)**:
1. **选择/复制**:GPU 文字无原生选择,必须自建 hit-test + 选区 + clipboard(0002 待办)
2. **无障碍**:必须做隐藏 DOM 镜像(Figma/Google Docs 都这么做,验证可行但有维护成本)
3. **IME**:输入交给 React/DOM(我们本就如此,规避),但画布内编辑要小心
4. **生态从零**:Streamdown/shiki-stream/use-stick-to-bottom 这些现成轮子我们用不上,
   等价能力要自己造——这是自研的根本成本

## 八、可借鉴的具体技巧(拿来即用)

- **`remend` 式自动补全**:不等闭合先补全渲染,比我们的 hold 区更激进,可作为
  尾部块的备选策略(0004/0006)
- **`Intl.Segmenter` 做 CJK 分块**:平滑器的分词单位,中文不能按词(0002)
- **`smoothStream` 的 50ms / 5ms-per-char 参数**:我们平滑器初值的业界参照(0002 §4)
- **LibreChat 可恢复流 + Redis 多标签同步**:0003 容错的进阶参照
- **TanStack `anchorTo:'end'` + `scrollEndThreshold:80`**:滚动锚定阈值参照(0002 §6)
- **VS Code 纹理图集预分配(3000 行×200 列懒填)**:atlas 容量规划参照(0001/0004)
- **HTML-in-Canvas API**:无障碍镜像与交互卡片的未来路径,持续观望(0007 §4)

---

## 来源

流式渲染:[Chrome 最佳实践](https://developer.chrome.com/docs/ai/render-llm-responses) ·
[AI SDK smoothStream](https://ai-sdk.dev/docs/reference/ai-sdk-core/smooth-stream) ·
[Upstash 平滑流](https://upstash.com/blog/smooth-streaming) ·
[Akash Kumar 重渲染](https://akashbuilds.com/blog/chatgpt-stream-text-react) ·
[use-stick-to-bottom](https://github.com/stackblitz-labs/use-stick-to-bottom)

流式 Markdown:[Streamdown](https://streamdown.ai/docs/termination) ·
[Vercel 公告](https://vercel.com/changelog/introducing-streamdown) ·
[shiki-stream](https://github.com/antfu/shiki-stream) ·
[llm-ui](https://llm-ui.com/) ·
[marked #3657](https://github.com/markedjs/marked/issues/3657)

框架:[assistant-ui](https://github.com/assistant-ui/assistant-ui) ·
[Lobe Chat 架构](https://lobehub.com/docs/development/basic/architecture) ·
[Open WebUI 渲染](https://deepwiki.com/open-webui/open-webui/2.2.2-message-display-and-streaming) ·
[HF chat-ui](https://github.com/huggingface/chat-ui) ·
[LibreChat 可恢复流](https://www.librechat.ai/docs/features/resumable_streams)

性能:[ChatGPT 长对话慢](https://community.openai.com/t/chatgpt-gets-extremely-slow-in-long-browser-chats-any-fix-coming/1133247) ·
[Claude Code #24146](https://github.com/anthropics/claude-code/issues/24146) ·
[TanStack Chat 虚拟化](https://tanstack.com/blog/tanstack-virtual-chat) ·
[Vercel memo cookbook](https://ai-sdk.dev/cookbook/next/markdown-chatbot-with-memoization)

Canvas/GPU 文字:[Figma is a game engine](https://medium.com/@nike_thana/figma-is-a-game-engine-not-a-web-app-how-c-and-wasm-broke-the-react-ceiling-8ed991bea48f) ·
[Google Docs canvas](https://thenewstack.io/google-docs-switches-to-canvas-rendering-sidelining-the-dom/) ·
[VS Code WebGPU #221145](https://github.com/microsoft/vscode/issues/221145) ·
[HTML-in-Canvas](https://developer.chrome.com/blog/html-in-canvas-origin-trial) ·
[glyphon](https://github.com/grovesNL/glyphon) ·
[SDF 字体](https://www.redblobgames.com/articles/sdf-fonts/)
