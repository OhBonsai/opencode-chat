# Plan 26:/chat 迭代结构完备性 —— 分析 + 两个结构补件(ThemeTokens / a11y 骨架)

- 日期:2026-07-02
- 状态:**已落地(2026-07-02)**。缺件①②③ 全部补齐,统一门全绿(349 测)。档位:主题=**运行时 ThemeTokens**;a11y=**读屏级**。
  - **缺件① ThemeTokens**:`theme.rs` 全部常量收编为 `pub struct Theme`(serde `#[serde(default)]` 局部覆盖;`Default` 逐值 = 旧观感,native 迁移护栏 + 视觉黄金帧双守);Engine 持 `theme` + `set_theme`(颜色不进排版缓存 → 不重排下一帧生效,native 测);wasm `set_theme(json)`;web:`?theme=<name>`(`themes/ocean.json` 样例)+ style-panel「Theme」节(11 token colorField)+ 剧本 `meta.theme`。e2e `theme.spec`(换主题像素生效/非法 JSON 忽略/空覆盖复原)。
  - **缺件② a11y 读屏骨架**:text-layer ARIA 镜像(容器 `role=log`,块级 `article` 包裹 `display:contents` + `aria-roledescription/posinset/setsize`,虚拟化只镜像可见块);`announcer.ts` live region(状态迁移粒度播报,**不逐 delta**;阻塞态 assertive;节流去重纯函数 vitest);Dock `alertdialog` + 焦点入首按钮/应答后还原;canvas `role=main` landmark。e2e `a11y.spec` 3 测。
  - **缺件③ /chat 底盘** = Plan 25(已落地,见彼)。
  - 迭代形态达成:调色=面板/URL/剧本秒级;a11y=措辞/粒度调优;案例=编辑剧本 JSON——均零结构改动。
- 前置:[Plan 25](./plan25-chat-scripted-showcase.md)(/chat 底盘)· [0030](../decision/0030-canvas-text-copy-select-a11y.md)(文本层三合一,§步骤3 即本篇 a11y 补件)· [0031](../decision/0031-event-fsm-resilience-and-js-rust-boundary.md)/[0033](../decision/0033-part-render-contract.md)(稳定轴结构)· research:reveal-rhythm / perception-readability-stability / streaming-chat-ux-standards / agent-ui-industry-survey
- 一句话:**/chat 案例要沿「稳定 / a11y / 美」三轴长期迭代;本篇审计哪些轴结构已闭合(迭代=纯丰富),哪些需一次性补结构。结论:稳定✅、节奏✅、视觉 token 与 a11y 各缺一件,加上 Plan 25 底盘共三件;补完后迭代不再动结构。**

---

## 1. 审计结论(按迭代轴)

| 轴 | 结构现状 | 判定 | 迭代形态(补完后) |
|---|---|---|---|
| **稳定** | FSM+F1–F12(Rust)、SSE 韧性(TS)、R8 确定钟、`push_event` 缝、test/ 四层门 | 🔵 闭合 | 加剧本场景 + resilience spec,零结构 |
| **节奏/reveal(美·动)** | 0025 动画系统、`set_reveal_cps/slow`、table reveal style、`seek_reveal` 定帧;research 三篇支撑 | 🔵 闭合 | 调参 + 剧本 dt 编排 |
| **视觉 token(美·静)** | `theme.rs` 全 `pub(crate) const` 硬编码;仅表格有 `set_table_style` 先例 | 🟡 **缺件①** | 补 ThemeTokens 后:面板/剧本/URL 秒级调色 |
| **a11y** | 0030 底盘对且已建一半(text-layer 承载复制/选区/查找);**§步骤3 镜像未做**:零 ARIA、无 live region、无焦点管理 | 🟡 **缺件②** | 补骨架后:措辞/粒度/verbosity 纯调优 |
| **/chat 底盘** | Plan 25 已定稿未实现(剧本 schema/player/boot 抽取/scripted 输入) | 🔴 **缺件③(=Plan 25)** | 建成后:迭代=编辑剧本 JSON |

> 非缺件备注:审美人工确认出图管线(Plan 24 §3.2)属测试资产丰富,不算结构;`style-config.ts`+面板机制就是缺件①的现成落点。

---

## 2. 缺件①:ThemeTokens(运行时主题结构)

**目标**:把 `theme.rs` 的散常量收成一个可运行时替换的 token 结构,调色循环从「改 Rust+重编译」变「面板拖动/剧本指定,秒级」。

### 2.1 设计

- **core**:`theme.rs` 改造——`pub struct Theme { code_bg: [f32;4], code_border: …, quote_bar: …, selection: …, card_bg: …, diff_add_bg: …, table_header_bg: …, alert_*, … }`(**逐一收编现有全部常量**,一个不漏);`impl Default for Theme` = 现值(**默认主题逐字节等于今日观感**,零回归)。`Engine` 持 `theme: Theme` 字段,原 const 读点全改走 `self.theme.*`。
- **关键性质(为什么便宜)**:颜色不进排版——缓存里存的是 `StyleRole`/几何,颜色在 `build_frame` emit 时才解析 → `set_theme` **不触发重排,下一帧即生效**,与 0029 虚拟化正交。若发现任何把颜色烤进缓存的点,视为 bug 顺手修。
- **wasm**:`set_theme(json: &str)`(serde 进 `Theme`,缺字段用 Default 补 → 局部覆盖友好),仿 `set_table_style` 先例。
- **web**:`style-config.ts` 加 `theme` 节(RGBA 语义同现有);`style-panel` 分节渲染;`?theme=<name>` 载 `web/public/themes/<name>.json`;**剧本 meta 可指定 theme**(Plan 25 格式加可选 `meta.theme`)。
- **CR1/R8**:Theme 纯数据、无平台依赖;native 可测。

### 2.2 测试

- native:`Theme::default()` 与旧常量逐值相等(迁移护栏,迁移完成后此测试退役为快照);set_theme 后 emit 色确定性。
- e2e:换主题截图 ≠ 默认截图(生效证明);默认主题黄金帧不变(零回归证明)。

---

## 3. 缺件②:a11y 读屏骨架(0030 §步骤3 兑现)

**目标**:读屏用户可用:消息流可导航朗读、流式有播报、Dock 阻塞可感知可操作。骨架一次建成,之后 a11y 迭代只调语义细节。

### 3.1 设计(xterm.js / PDF.js 模式,底盘=现有 text-layer)

- **ARIA 镜像(text-layer.ts 增强)**:容器 `role="log"` + `aria-label="对话"`;每消息块 `role="article"` + `aria-roledescription`(用户消息/助手消息/工具卡…从 `visible_turns()` 的 role 映射)+ **`aria-posinset`/`aria-setsize`**(总消息数为 setsize——虚拟化只镜像可见块,posinset 保序,xterm.js 式虚拟列表)。
- **live region 播报器(新 `web/src/announcer.ts`)**:视觉隐藏的 `aria-live="polite"` 区。**不逐 delta 播报**(会淹没读屏)——按「part settle / 消息完成」粒度播;`session_status()` 变化播状态(思考中/调用工具/完成);节流+去重逻辑独立纯函数(vitest)。permission/question 弹出 → `aria-live="assertive"` + **焦点移入 Dock**。
- **焦点管理**:Dock 打开时焦点入首按钮、Esc/应答后**焦点还原**到之前位置;dock 按钮本就是真 `<button>`(键盘可达),补 `role="alertdialog"` + `aria-modal`;find-bar/chat-input 已可聚焦,补 landmark(`role="main"` 画布区、`role="form"` 输入区)。
- **数据源零新增**:全部由既有 `visible_turns()`/`visible_text_runs()`/`session_status()` 驱动,与 text-layer 同一泵;**core/wasm 不改**。

### 3.2 测试

- vitest:播报器节流/去重/粒度(喂状态序列断言播报串)。
- e2e(进默认门):ARIA 属性存在且 posinset 随滚动正确;合成事件驱动下 live region 文本按预期更新;Dock 打开焦点在按钮、关闭焦点还原。(axe-core 扫描可作丰富项后加。)

---

## 4. 落地顺序与并行关系

```
Plan 25 PR-A/B(/chat 底盘)──┐
缺件① ThemeTokens ───────────┼─→ 三者互不依赖,可并行/任意序
缺件② a11y 骨架 ─────────────┘
之后:/chat 案例三轴迭代 = 剧本 JSON + 调参 + 措辞,零结构改动 ✅
```

- 建议顺序:**Plan 25 PR-A/B 先**(案例先能跑)→ ①(美轴迭代最频繁,早解锁)→ ②。
- 全部走 test/ 默认门;①② 各自带上面的测试,防"补结构引入回归"。

## 5. 非目标

- 多主题产品化(亮色主题=丰富项,结构①已留门)。
- AOM/虚拟 a11y 树(0030 已否)。
- 剧本可视化编辑器。
