# Plan 25:`/chat` 剧本回放页 —— 预设事件流还原完整 agent 对话

- 日期:2026-07-02
- 状态:**已落地(2026-07-02)**。PR-A/B/C/D/E 全部完成,统一门 `node test/run.mjs` 全绿(349 测)。
  - **PR-A**:剧本 schema/校验/时间轴(`web/src/chat/script.ts`)+ 调度器(`scripted-player.ts`,tick 驱动可测;**打字门**:user 指令返回 Promise 时冻结时间轴,任何倍速/负载下顺序确定)+ `boot.ts` 共享装配抽取(main 与 /chat 复用)。vitest 25 测。
  - **PR-B**:`web/chat/index.html` + vite 多页;chat-input **scripted 模式**(`mountScriptedInput`:同一份真组件真样式,禁网络,`typeText`/`flashSend`);Dock 自动点击(真 DOM click);播放器 chrome(播/停/倍速/scrubber;后向 seek = `?at=` 重载快放)。**注意**:/chat 用**空 replay** 启动(静默 Player)——否则落合成演示流污染剧本。`chats/mini.json` 端到端。
  - **PR-C**:`chats/showcase-full.json` —— §3 全 11 场(修复登录超时 bug 叙事),`meta.theme: "ocean"` 带 Plan 26① 主题。e2e 里程碑序列断言。
  - **PR-D**:`scripts/record-chat.mjs`(SSE 录制 + 纯转换:session 过滤/噪音丢弃/user 升格/dt 差分;vitest 于 `convert.test.ts`,草稿过剧本 schema round-trip)。
  - **PR-E**:demo 主页「💬 完整对话」入口;`PAGES_BASE` 多页构建验证(`dist/chat/index.html` 带子路径);`chat-route.spec` 进 test/ 默认门。
  - 人工余项:真 server 录一份会话跑一遍转换器(PR-D 工具已备);Pages 部署后线上点一遍;1× 速度全片人审节奏(§4 🟡)。
- 前置:[Plan 22](./plan22-opencode-events-and-fsm.md)(`push_event` 全事件消费,一切内容经它进引擎)· [Plan 23](./plan23-part-render-implementation.md)(全 part 渲染)· [Plan 17](./plan17-intro-film.md)(film 导演时钟,本页的近亲)· [Plan 24 §4](./plan24-integration-functional-effect-test.md)(录像资产思路)· `web/src/replay.ts`(现有 case 格式,本剧本为其超集)· 调研 [agent-ui-industry-survey](../research/agent-ui-industry-survey.md) / [streaming chat UX](../research/)(参考 Claude Cowork 等对话形态)
- 需求陈情已确认(2026-07-02):**① 用户输入=模拟打字+发送;② 纯自动播片(可加播放器控制);③ 剧本手工编排,但须支持「opencode 真实对话 → 剧本」转换,剧本=配置文件;④ 上 GitHub Pages。**

---

## 0. 目标 / 非目标

**目标**:新增 `/chat/` 路径 = 一场**预设剧本的完整 agent 对话回放**——用户逐字打字发问 → assistant 流式回答 → reasoning → tool 卡三态 → diff → 权限 Dock(自动应答)→ 反问 → 报错恢复 → compaction → 多轮收尾。**当前支持的所有 event 类型全部出场**,节奏还原真实对话。不连 opencode,数据全来自剧本配置文件。

**非目标**:
- 不做观看者交互(Dock 由剧本自动应答;播放器仅 暂停/倍速/进度)。
- 不做剧本可视化编辑器(配置文件手编 + 转换器打底)。
- 不改 core/wasm 的事件语义(引擎眼里这就是一场真会话——**零引擎特判**)。

**核心原则**:`/chat` 是引擎的**消费者**,只经公开接口(`push_event` + DOM)驱动。所有画布内容走真实事件路径 → 这份剧本天然是 Plan 24 §4 想要的「全事件回归资产」,一鱼两吃。

---

## 1. 剧本格式(`web/public/chats/<name>.json`)

现有 `cases/`(纯 text delta)表达力不够:要用户轮、要任意事件、要 Dock 应答、要节奏标记。新格式 = **指令时间轴**,三种指令:

```jsonc
{
  "meta": { "title": "修复登录 bug", "version": 1 },
  "track": [
    // ① 用户轮:输入框逐字打出 → 停顿 → 发送。发送本身不产生画布内容——
    //    紧随其后的 message.updated(role=user) 事件才是画布上的用户气泡(真实路径)。
    { "dt": 0,    "user": { "text": "帮我修一下登录超时的 bug", "cps": 14, "holdMs": 500 } },

    // ② 事件:opencode 事件**原样 JSON 对象**(非字符串,手编/diff 友好);player 序列化后 push_event。
    { "dt": 300,  "event": { "type": "message.updated", "properties": { /* user 气泡 */ } } },
    { "dt": 400,  "event": { "type": "message.part.delta", "properties": { /* 流式… */ } } },

    // ③ Dock 应答:剧本替观看者"点"真按钮(DOM click .dock-allow,走真代码路径)。
    { "dt": 1200, "dock": "allow" }
  ]
}
```

- **`dt` = 距上一条的毫秒**(手工插改一条不用重排全轴;转换器从录制时间戳差分得出)。
- 兼容性:`cases/`(旧 replay)不动;`chats/` 是新目录新格式。`?speed=` 全局缩放 dt(同 replay 惯例)。
- schema 用 TS 类型 + 运行时校验(载入时报错指向条目下标),vitest 单测。

## 2. 组件与改动清单

| 件 | 位置 | 做什么 |
|---|---|---|
| **页面** | `web/chat/index.html` + vite 多页 input | 产出 `dist/chat/index.html` → Pages 下 `/infinite-chat/chat/`。薄壳:共享装配 + player |
| **共享装配** | `web/src/boot.ts`(从 main.ts 抽取) | canvas/wasm init、layout/rasterize、overlay/文本层泵——main 与 chat 复用,**不复制粘贴** |
| **剧本 player** | `web/src/chat/scripted-player.ts` | 指令时间轴调度(rAF+墙钟,暂停/倍速/seek);`user`→打字机;`event`→`push_event`;`dock`→DOM click |
| **打字机** | chat-input 加 **scripted 模式** | 禁网络(不 POST);暴露 `typeText(text, cps)`(逐字填输入框+光标)与 `flashSend()`(按发送态样式闪一下再清空)。真组件真样式,拒绝仿品 |
| **播放器 chrome** | 复用 film/player.ts 的进度条形态 | 播/停/倍速/scrubber;时长=Σdt。**seek 向后 = 从头快放到目标点**(事件流是累积状态,不可逆放;R8 确定性保证结果一致) |
| **转换器** | `scripts/record-chat.mjs` | 连真 opencode `/event` 录 `{t,raw}` + 过滤目标 session → 差分成 `dt` → user 消息事件自动升为 `user` 指令(text 从事件抽出)→ 产**剧本草稿**,人再精修节奏/删噪音 |
| **全事件谱剧本** | `web/public/chats/showcase-full.json` | 手工编排的叙事(见 §3) |
| **Pages 接入** | pages.yml 无需改(crates/web 路径已触发);demo 主页链接栏加「完整对话」入口 | `VITE_DEMO` 语义不变;`/chat` 页自身无输入依赖 |

## 3. 全事件谱剧本(验收清单,缺一不算完)

一个连贯任务叙事(如「修 bug」),按真实节奏出场:

| # | 场 | 覆盖的 event/效果 |
|---|---|---|
| 1 | 用户开场提问 | 打字机 + `message.updated`(user) |
| 2 | assistant 思考 | `reasoning` part(思考区弱化样式) |
| 3 | 流式回答 | `message.part.delta` 逐字 + markdown 结构块(代码块/列表/表格) |
| 4 | 读文件 | `tool`(read)pending→running→completed 三态徽章 |
| 5 | 权限请求 | `permission.asked` → Dock 弹出 → **剧本自动点 Allow** → 解阻 |
| 6 | 改文件 | `tool`(edit)+ `metadata.filediff` → diff 块(增删行) |
| 7 | file 附件 | `file` part chip |
| 8 | 反问 | `question.asked` → Dock → 自动应答 |
| 9 | 出错与恢复 | `tool` state=error 红卡 → 重试 completed;`session.error` 错误卡恒一张 |
| 10 | compaction | 分隔线 + 标签 |
| 11 | 二轮追问收尾 | 再一轮 user 打字 → 简答 → `session.status idle` |

节奏基准:参考 [md-reveal-cadence 北极星](../research/)——阅读体验优先,登场节奏由剧本 dt 手工调到"像真的"。

## 4. 测试(进 test/ 默认门)

- **vitest**:剧本 schema 校验(合法/非法样例);`dt`→绝对时间轴换算;转换器纯函数(录制记录→草稿)round-trip。
- **e2e `chat-route.spec.ts`**:`/chat/?speed=50` 快放到底,断言里程碑序列:用户气泡文本 → tool 卡 → Dock 出现且自动消失 → diff 块 → 错误卡恰一张 → 终态 idle;全程无 crash/console error。**剧本本身即 Plan 24 §4 的回归资产**。
- **人工确认(🟡)**:1× 速度全片人看一遍,签节奏;之后靠 e2e 里程碑守回归。

## 5. 落地顺序(PR 切分,每步可跑)

1. **PR-A 格式+player 核**:剧本 schema/loader/调度器(纯 TS,vitest)+ `boot.ts` 抽取。
2. **PR-B 页面+打字机**:`chat/index.html` + chat-input scripted 模式 + Dock 自动点击 + 播放器 chrome。用 3 条指令的迷你剧本端到端跑通。
3. **PR-C 全事件谱剧本**:手工编排 `showcase-full.json` 达成 §3 清单,调节奏。
4. **PR-D 转换器**:`record-chat.mjs` 录真会话→草稿;用一次真录制验证格式覆盖(反哺 §3 补漏)。
5. **PR-E 上线+回归**:demo 主页入口 + Pages 验证子路径;`chat-route.spec.ts` 进默认门。

## 6. 风险

- **seek 语义**:事件流不可逆放 → 后退=重放到目标(R8 确定性使其可行);长剧本重放要快(`speed=∞` 静默灌+末段正常速),PR-A 就定此机制。
- **chat-input 侵入**:scripted 模式必须零影响真发送路径(条件分支仅在构造参数,e2e F02 将来补测真路径时不受扰)。
- **Pages 子路径**:`BASE_URL` 已有惯例(replay/msdf 都处理过),chat 页 fetch `chats/` 沿用即可;`chat/index.html` 相对路径注意 `../` 资源引用(vite 会处理,验收时 Pages 上点一遍)。
