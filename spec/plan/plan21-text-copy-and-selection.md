# Plan 21:文本可复制 / 选区落地(ADR 0030 步骤 1+2+4)

- 日期:2026-06-24
- 状态:**已落地(2026-06-25)**;实现 [0030](../decision/0030-text-copy-selection-and-a11y-mirror.md) 的**步骤 1 + 2 + 4**;**步骤 3(ARIA a11y 镜像)挂 TODO**(见 §7)。
  - **已完成**:P1 复制按钮(`visible_turns`/`copy-button.ts`)、P2 选区+复制(`visible_text_runs`/`set_selection`/`text-layer.ts` + GPU 逐行高亮)、P3 自建 Cmd+F 跨历史(`find`/`scroll_to`/`find-bar.ts`)+ 富文本复制(`ClipboardItem` text/plain+text/html)+ 选区墨团(**逐行圆角连体**)。
  - 测试全绿:native N1–N7 + `verify` 五卡口(215 native);Playwright E1–E8 + 视觉 V1–V3(11/11)。
  - **唯一保留**:墨团的 **0025 §4 跨行 `smin` 连体**(需 render 侧 WGSL pass)未做——当前是逐行圆角连体 `FrameRect`(已是实用观感)。跨行 SDF 连体作后续视觉细化,见 §1.3。
- 前置:[0030](../decision/0030-text-copy-selection-and-a11y-mirror.md)(决策:DOM 管语义 / GPU 管视觉)、[0029](../decision/0029-session-virtualization-and-glyph-working-set.md)/[Plan 19](./plan19-session-virtualization.md)(Hot tier 虚拟化,文本层必须复用)、[0022](0022-dom-overlay-layer.md)/`embed-overlay.ts`(相机同步 overlay 机制,照搬)、[0025](../decision/0025-sdf-node-animation-system.md)(SDF 选区墨团)、[0011 §3.3④](../decision/0011-gpu-text-as-sdf-primitive.md)(CPU 盒备用)、`theme.rs`(选区色令牌)。
- 目标:让画布里的对话**可复制、可选中**:① 每条消息一键复制;② 拖选 + Cmd+C 复制,选区高亮画在文字下(任意多色不被遮);④ 选区做成 SDF 墨团 + 富文本复制 + 自建页内查找。**a11y 镜像(③)留 TODO,但本期结构为它预留。**

---

## 0. 范围与一句话架构(扣 0030)

> **DOM 透明文本层(仅可见块)管语义**(原生选区/复制/Cmd+F)**+ GPU 用引擎自己的几何画选区高亮于文字下**(视觉)**+ core 不碰平台**(CR1)。

本期 = 0030 §9 的 **1(复制按钮)+ 2(选区+复制 MVP)+ 4(增强)**;**3(ARIA 读屏)挂 TODO**。**关键约束:DOM 文本层 DOM 节点数 ∝ 可见块(Hot tier),不随历史增长**——否则毁掉 Plan 19 刚拿到的无限会话内存目标。

> **一处冲突已 surface(AI Rule 5)**:0030 §9 把 `ariaNotify` 归在步骤 4。但 `ariaNotify` 是**读屏播报**,属 a11y 范畴,脱离步骤 3 的 DOM 镜像单独做没意义 → **本计划把 `ariaNotify` 从步骤 4 移到步骤 3 的 a11y TODO 一起做**,步骤 4 只保留与 a11y 无关的增强(SDF 墨团 / 富文本复制 / 查找)。

---

## 1. 三相策略(对应 0030 步骤 1/2/4)

| 相 | = 0030 步 | 做什么 | 攻克 |
|---|---|---|---|
| **P1 复制按钮** | 步骤 1 | 每个可见回合一个"复制",取该回合渲染后纯文本 → `clipboard.writeText` | 最高频需求"复制这段",零选区 |
| **P2 选区 + 复制 MVP** | 步骤 2 | 虚拟化透明 DOM 文本层(仅 Hot 块)+ 原生选区 + Cmd+C + `getClientRects`→**GPU 高亮于文字下** | 任意选区 + 复制,高亮不遮文字 |
| **P3 增强** | 步骤 4(去 ariaNotify) | SDF 墨团选区([0025])+ 富文本复制(text/plain+text/html)+ 自建页内查找(Store 索引) | 效果上限 + 跨历史查找 |

**a11y(0030 步骤 3 + ariaNotify)= TODO**,见 §7,另起 plan。

### 1.1 P1 — 每消息复制按钮

- **core/wasm**:`ChatCanvas.visible_turns()`(仿 `frame_embeds()`)→ JSON `[{turn_id, role, x, y, w, h, text}]`,`text` = 该回合**渲染后纯文本**(join 各块 `cache.clusters`,去 markdown 噪声;无 cache 的 Warm 块用 `revealed`/`Store` 源补)。坐标 = 屏幕设备像素(world→screen)。
- **web**:`copy-button.ts`——每帧(rAF,同 embed-overlay)按 `visible_turns()` 在每回合右上角摆一个浮层"复制"按钮(`pointer-events:auto`);点击 → `navigator.clipboard.writeText(text)`(用户手势,满足 Clipboard 约束)→ 短暂"已复制 ✓"。回收滚出视口的按钮(同 embed-overlay 习惯)。
- **验收**:点任一可见回合的复制 → 剪贴板得该回合渲染后纯文本;按钮随相机 pan/zoom 跟手;按钮数 ∝ 可见回合。

### 1.2 P2 — 选区 + 复制(MVP)

- **core/wasm**:
  - `ChatCanvas.visible_text_runs()` → JSON `[{block, line, x, y, w, h, text}]`(仅 **Hot/可见块**,line 级即可):每可见块每行一个透明 span 的位置 + 文本 + `block`/起始字符偏移,供 host 建文本层。坐标屏幕设备像素。
  - `ChatCanvas.set_selection(json)`:host 把 DOM 选区映射成**字符区间** `[{block, start_char, end_char}]` 灌入;core 存为选区态(presentation,**不进 reveal/不破确定性**,与相机同级)。
  - `app.rs::build_frame`:据选区字符区间 → 查该块 `cache.placed` 对应字形盒 → 发 **`FrameRect`(选区高亮)**,**在 glyph 之前**绘制(rect pass 顺序天然在前)→ 文字永全不透明在上。颜色取 `theme.rs` 新令牌 `SELECTION`。
- **web**:`text-layer.ts`——
  - 每帧按 `visible_text_runs()` 建/复用透明 `<span>`(`color:transparent; white-space:pre; user-select:text`),带 `data-block`/`data-char0` 属性;层 `color-scheme: only light`(防强制反色,PDF.js 式)。
  - **`::selection { background: transparent }`**(DOM 不画高亮,交 GPU)→ 杜绝"双高亮"与遮挡。
  - 监听 `selectionchange` → 用 `getSelection().getRangeAt(0)` + span 的 `data-*` 映射成 `[{block,start_char,end_char}]` → `chat.set_selection(json)`(节流:选区变化时,**非每帧**,避免 `getClientRects`/layout 抖)。
  - Cmd+C / 复制事件 → 取选中文本(`getSelection().toString()` 或按字符区间从 `visible_text_runs` 重组)→ `clipboard.writeText`。
- **验收**:拖选可见文字 → **GPU 高亮压在文字下、文字全可读(代码/链接/标题多色都不被洗淡)**;Cmd+C 得选中渲染纯文本;**文本层 DOM 节点数 ∝ 可见(用 debug 计数验,不随 10k 历史涨)**;选区限可见窗内(v1)。

### 1.3 P3 — 增强(0030 步骤 4,去 ariaNotify)

- **SDF 墨团选区([0025])**:把 P2 的逐行 `FrameRect` 高亮升级为跨行并集 `smin` 圆角连续墨团(走 panel/widget SDF);macOS 选区观感。纯视觉,接 0025 §4 相位 4。
  - **已落地**:`app.rs::push_selection_rects` 把同行被选字形**合并成圆角连续条**(`FrameRect.radius`),逐行一条 → 实用墨团观感,零 render 改动。N7 守"墨团 ⊇ 逐字形并集";V3 黄金帧。
  - **保留**:跨**行**的 `smin` 连体(0025 §4 相位 4)需 render 侧新 SDF pass,作后续视觉细化。
- **富文本复制**:`clipboard.write([new ClipboardItem({'text/plain':…, 'text/html':…})])`——同时给纯文本 + HTML(或 markdown 源),提保真;Safari 友好写法(同步传 Promise)。
- **自建页内查找(Cmd+F)**:用 `Store` 全文建轻量索引(已有 `parts_in_order`/`char_count`)→ 输入框查 → `ChatCanvas.find(query)` 返回命中位置 → `scroll_to` + `set_selection` 跳转并选中。**解决"虚拟化导致原生 Cmd+F 只覆盖可见"**(0030 §7.7)。
- **验收**:选区呈 SDF 墨团;复制含 text/html;Cmd+F 跨**全历史**命中并跳转选中。

---

## 2. 改动清单(精确到 file:符号;CR1/R8 守则见 §5)

**core(`crates/core`)**
- `app.rs`:`visible_turns()` 数据(回合屏幕盒 + 渲染纯文本)、`visible_text_runs()` 数据(可见块逐行盒+文本+字符偏移);`set_selection(ranges)` 存选区态;`build_frame` 据选区查 `placed` 发选区 `FrameRect`(glyph 前)。
- `theme.rs`:加 `SELECTION`(+ 可选 `SELECTION_INK` for P3 墨团)色令牌。
- (P3)`app.rs`:`find(query)`(Store 索引)→ 命中位置;`scroll_to(block/char)`。
- (P3)`shaderbox.rs`/render:选区墨团 SDF(`smin` 并集,接 0025)。

**wasm(`crates/wasm`)**
- `lib.rs::ChatCanvas`:`visible_turns()`/`visible_text_runs()`(JSON,仿 `frame_embeds`)、`set_selection(json)`、(P3)`find`/`scroll_to`。

**web(`web/src`)**
- 新 `copy-button.ts`(P1)、`text-layer.ts`(P2:透明虚拟文本层 + selectionchange→set_selection + 复制);`main.ts` 挂载(rAF pump,同 embed-overlay);CSS(透明文本、`::selection` 透明、`color-scheme:only light`)。
- (P3)`find-bar.ts`(Cmd+F UI)、富文本复制(ClipboardItem)。

**测试**:完整方案见 **§3**(自动化清单 + 人工 TODO),每个 PR 必须带上对应自动化测试且全绿才算完成(FAIL_TO_PASS ∧ PASS_TO_PASS)。

---

## 3. 测试方案(自动化清单 + 人工 TODO)

> 给 Claude Code 直接执行:**§3.1–§3.3 是可自动化测试**(每条 = 一个可写的测试,带文件/名/断言);**§3.4 是人工测试**(需人眼/真机/真读屏,不自动化,挂 TODO)。验收 = 对应自动化测试全绿 + 卡口过。

### 3.1 Native 单元/属性测试(`crates/core`,`cargo test`,最快最稳,无浏览器/GPU)

| # | 测试名(`app.rs` `#[cfg(test)]`) | 断言 | 相 |
|---|---|---|---|
| N1 | `sel_highlight_rects_cover_selected_glyphs`(proptest) | 任意 `placed` + 任意字符区间 `[a,b)` → `build_frame` 的选区 `FrameRect` 集合**覆盖区间内每个非零墨字形盒**、**不覆盖区间外字形** | P2 |
| N2 | `sel_empty_range_no_highlight` | 空/越界区间 → 0 个选区 `FrameRect` | P2 |
| N3 | `visible_turns_text_deterministic` | 同 store 状态两次调 `visible_turns()` → `text` 逐字节相同;且 = 各可见块 `cache.clusters` join(去 markdown 噪声) | P1 |
| N4 | `visible_text_runs_excludes_offscreen` | 构造含屏外 Warm 块的引擎 → `visible_text_runs()` 不含 Warm/屏外块的 run(**虚拟化**) | P2 |
| N5 | `selection_does_not_affect_reveal`(R8) | `set_selection` 前后,reveal 调度/各 view `spawn` 不变;录像重放末状态与"无选区"逐字节相同(选区是 presentation) | P2 |
| N6 | `find_hits_match_naive`(proptest,P3) | 同 query 同 store → 命中位置序列确定 = 朴素子串扫描结果 | P3 |
| N7 | `selection_ink_blob_contains_line_rects`(P3) | 墨团 `smin` 并集几何**包含**逐行矩形并集(升级不漏选) | P3 |

### 3.2 Playwright E2E(`web/tests/*.spec.ts`,headless Chromium WebGPU,复用现有 `playwright.config.ts`)

> 通用前置:`await context.grantPermissions(['clipboard-read','clipboard-write'])`;`page.goto('/?replay=showcase&noinput')`;`await page.waitForFunction(() => window.__chat?.stats)`。定帧用 `page.evaluate(()=>{__chat.set_paused(true); __chat.seek_reveal(3000)})`。CI 用 `Control+C`(非 `Meta`)。

| # | 文件 › 用例 | 步骤 + 断言 | 相 |
|---|---|---|---|
| E1 | `copy.spec.ts` › 复制按钮写剪贴板 | 点 `.copy-btn`(首个可见回合)→ `navigator.clipboard.readText()` 含该回合预期文字 | P1 |
| E2 | `copy.spec.ts` › 复制按钮随相机跟手 | `__chat.pan_by(0,-300)` → 按钮屏幕 y 相应变化(读 `getBoundingClientRect`) | P1 |
| E3 | `selection.spec.ts` › 拖选产生 GPU 高亮 | 定帧 → `page.mouse` 拖选一段 → `__chat.stats()` 的选区高亮矩形数(新增计数字段)`> 0` | P2 |
| E4 | `selection.spec.ts` › Cmd+C 复制选中文本 | 拖选 → `Control+C` → `clipboard.readText()` = `window.getSelection().toString()` 对应渲染文本 | P2 |
| E5 | `text-layer-virtualized.spec.ts` › DOM ∝ 可见 | `goto('/?bench&spread=0')`(载 10k)→ `page.locator('.text-layer span').count()` < 500(**远小于总行数**);且 ≈ 可见块行数 | P2 |
| E6 | `selection.spec.ts` › 选区限可见窗(v1) | 选区一端拖到屏外 → 不报错、选区 clamp 到可见(断言不抛 + 高亮矩形数有界) | P2 |
| E7 | `find.spec.ts` › 自建 Cmd+F 跨历史(P3) | 载 10k → `__chat.find('某屏外词')` 返回命中 → 触发跳转 → 该词进入可见(`visible_text_runs` 含之)+ 被选中 | P3 |
| E8 | `copy.spec.ts` › 富文本复制含 html(P3) | 复制 → `navigator.clipboard.read()` 的 `ClipboardItem` 同时有 `text/plain` 与 `text/html` | P3 |

### 3.3 视觉回归(Playwright `toHaveScreenshot`,定帧确定性,守渲染不回退)

> 钉死现有 headless Chromium + WebGPU flags(已在 config);容差 `maxDiffPixelRatio: 0.01`(容 GPU 抗锯齿亚像素噪声)。首次 `npx playwright test --update-snapshots` 生成基线入库。

| # | 文件 › 用例 | 做法 | 相 |
|---|---|---|---|
| V1 | `visual.spec.ts` › 选区高亮黄金帧 | 定帧(`seek_reveal(3000)`)→ 程序化设选区(`__chat.set_selection([...])`)→ `expect(page.locator('#chat')).toHaveScreenshot('sel-3s.png')` | P2 |
| V2 | `visual.spec.ts` › 高亮不遮文字(像素证) | 同帧:无选区截图 vs 有选区截图,断言**选中区文字色像素仍存在**(选区区域内仍能采到非背景的文字色 → 证文字在高亮之上) | P2 |
| V3 | `visual.spec.ts` › SDF 墨团黄金帧(P3) | 多行选区定帧 → `toHaveScreenshot('ink-blob.png')` | P3 |

### 3.4 人工测试(TODO — 需人眼/真机/真读屏,不自动化)

> Claude Code **不实现**这些,留人工/后续;`verify` 里以"待人工确认"标注,不阻断自动化绿灯。

- [ ] **选区/墨团观感**是否好看(主观审美;后续可接 LLM pairwise 判图,首期人看)。
- [ ] **暗主题选区色对比**是否舒服(人眼)。
- [ ] **粘贴到外部应用**(Word/Notion/微信)格式保真(text/html)——CI 剪贴板隔离,粘进真实第三方 app 需人工。
- [ ] **触摸板/触屏拖选手感**、长按选词、双击选词、三击选行(真设备交互体感)。
- [ ] **真机多 OS**(mac/Win/Linux)复制快捷键 + 剪贴板行为差异。
- [ ] (步骤 3 a11y,本期不做)**真实屏幕阅读器**(VoiceOver/NVDA)朗读顺序与可懂性——自动化只能验 ARIA 属性存在,实际朗读体验需人工 + 读屏用户。

### 3.5 机器可读输出(接 [Plan 20](./plan20-minimal-test-pipeline.md))

- `playwright.config.ts` reporter 加 `['junit', { outputFile: 'web/test-results/junit.xml' }]`;native 走 `cargo test`(或 nextest JUnit)。
- `verify` 聚合 native + Playwright + 视觉结果 → `report/TESTREPORT.md`(失败名+断言+file:line)交 dev agent。

### 3.6 验收门(= 上述自动化全绿 + 卡口)

| 相 | 必过自动化 | 卡口 |
|---|---|---|
| P1 | N3 · E1 · E2 | clippy / native test / wasm build / tsc |
| P2 | N1 N2 N4 N5 · E3 E4 E5 E6 · V1 V2 | 同上 |
| P3 | N6 N7 · E7 E8 · V3 | 同上 |

---

## 4. 关键不变量与护栏(扣 0030 §7)

1. **虚拟化(硬约束)**:DOM 文本层 + 复制按钮 = **仅 Hot 可见块/回合**,随相机重建(复用 Plan 19 tier / `frame_embeds` 同步)。**DOM 节点 ∝ 可见,绝不随历史**。
2. **高亮用引擎几何**:选区→字符区间→`placed` 盒 → 自动对齐;DOM 层只需**字符序正确**,不追像素对齐(绕开 PDF.js scaleX 痛点)。
3. **选区不遮文字**:GPU 画文字下(rect pass 在 glyph 前);DOM `::selection { background: transparent }` 防双高亮。
4. **暗主题**:文本层 `color-scheme: only light`;选区色走 `theme.rs`,不继承浏览器默认蓝。
5. **CR1**:core 只新增**纯数据**(选区态 + 几何计算 + 文本提取),DOM 全在 web → core 仍 native 可测。
6. **R8 确定性**:选区/复制是 **presentation 输入**(同相机 pan),**不进 reveal、不入录像内容** → 不破重放。
7. **a11y 预留**:文本层 span 已带 `data-block`/`data-char` → 步骤 3 加 ARIA(role/aria-live/posinset)是**纯增量**,本期结构不挡路。

---

## 5. 落地顺序(PR 切分,每个端到端可跑 + 过卡口)

> 每个 PR **必须随附其测试**(§3 对应 id)且全绿才算完成;括号内为必带测试。

1. **PR-A(P1)**:`visible_turns()` + `copy-button.ts`。最小、立刻可用。**测试:N3 · E1 · E2**。
2. **PR-B(P2 core)**:`visible_text_runs()` + `set_selection` + `build_frame` 选区高亮 + `theme.SELECTION`。**测试:N1 N2 N4 N5**(native 先行,无浏览器即可验)。
3. **PR-C(P2 web)**:`text-layer.ts`(透明虚拟层 + selectionchange→set_selection + Cmd+C)+ CSS。端到端选区+复制达标。**测试:E3 E4 E5 E6 · V1 V2**;`playwright.config.ts` 加 JUnit reporter(§3.5)。
4. **PR-D(P3)**:SDF 墨团选区 + 富文本复制 + 自建 Cmd+F。**测试:N6 N7 · E7 E8 · V3**。
5. **(TODO,另起 plan)** 步骤 3:ARIA a11y 镜像 + ariaNotify(见 §7)。

---

## 6. 非目标(本期)

- 选区跨虚拟化边界(一端在 Warm/屏外块):v1 限可见窗内选区。
- on-canvas 文本编辑(画布只读,输入走 `chat-input.ts`,0030 §1)。
- 跨浏览器像素一致 / BiDi(opinionated 单实现,同 TODO V)。

## 7. TODO:步骤 3 — ARIA a11y 镜像(本期不做,但已预留)

> 屏幕阅读器读不了 canvas(黑像素)→ 可嵌入组件**否决项级硬需求**(README 原则 9 / TODO R)。**复用 P2 的同一层透明文本**,加语义即可,故本期把钩子留好、不做实现:

- 文本层根 `role="log"` + `aria-live="polite"`(流式播报,注意节流防念爆);每回合 `role="article"` + `aria-label`(user/assistant);虚拟列表 `aria-posinset`/`aria-setsize`(xterm.js 式,因虚拟化只有可见块在 DOM)。
- `ariaNotify`(Chromium 141+,从 0030 步骤 4 移来)做事件播报(如"已选中 N 字"),fallback `aria-live`。
- **虚拟化 × 读屏冲突**:屏外块无 DOM → 读屏只能读可见 + 焦点哨兵翻页(xterm.js 式)或配合自建 find。设计见 0030 §7.7。
- 另起 **Plan 22(a11y 镜像)** 落地。

---

参考:[0030](../decision/0030-text-copy-selection-and-a11y-mirror.md)(决策 + 业界实证)· [Plan 19](./plan19-session-virtualization.md)/[0029](../decision/0029-session-virtualization-and-glyph-working-set.md)(Hot tier 虚拟化)· `embed-overlay.ts`/[0022](0022-dom-overlay-layer.md)(相机同步)· [0025](../decision/0025-sdf-node-animation-system.md)(SDF 墨团)。
