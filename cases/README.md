# Replay cases(Plan 5D)

带时间戳的 text-delta 序列,模拟 opencode SSE 流(**不连服务端**),逐条覆盖 [Plan 5C](../../../spec/plan/plan5-streaming-markdown.md) 规格表,验"streaming 全程无跳变"。

- 格式:`{ steps: [{ t, delta }], sessionID?, messageID?, partID? }`;`delta` 拼接 = 完整 markdown。
- 跑:`?replay=<name>`(如 `?replay=c06-table`);叠标尺:`?replay=c06-table&verify`;看帧统计:加 `&debug`(面板可暂停/单步 = 逐帧检视过渡)。
- harness:[`web/src/replay.ts`](../../src/replay.ts) 把每步包成 `message.part.delta` 信封 → `ChatCanvas{replay}` → core `Player`。

| case | 构造(5C) | 看点 |
|---|---|---|
| c01-plaintext | 纯文本 | 逐字 enter + 折行 |
| c02-bold-close | `**粗**`/`*斜*` | 闭合瞬间字面→样式,字宽 update 不压扁 |
| c03-inline-code | `` `行内码` `` | 闭合→等宽 + chip 底 |
| c04-list | 列表逐项 | item enter,行距 |
| c05-fence | 代码围栏 | open 当段落 → close 归类代码块 + 底 rect |
| **c06-all** | 表格全场景一条流 | 对齐/CJK/内联/残缺/宽表 5 段连看(一次看到所有缺口) |
| c06-table | 表格逐行(**核心**) | 新行撑宽列 → 旧行右侧单调右移补间;含对齐标记 + CJK 行 |
| c06b-table-align | 表格列对齐 `:--`/`:-:`/`--:` | 左/中/右对齐(当前实现:仅左对齐 → 暴露缺口) |
| c06c-table-cjk | 表格 CJK / 全角 | 中日全角列宽(display_width 计 2)对齐 |
| c06d-table-inline | 单元格内联格式 | `**粗**`/`` `码` ``/`[链接]`/`~~删~~`(当前:纯文本 → 暴露缺口) |
| c06e-table-ragged | 残缺/多余单元格 + 空格 | 缺格补空、超列丢弃、空行不 panic |
| c06f-table-wide | 多列 / 长内容 | 宽表超出消息列宽的溢出行为(暴露缺口) |
| c07-setext | setext 标题 | 下一行 `===` 回溯升级,字号 scale |
| c08-quote-alert | 引用 + GFM Alert | 左条/类型色底 enter |
| c09-mixed-long | 综合多块 | 上方块长高 → 下方块整体下移补间 |
| c10-cjk | 中英混排 | CJK 禁则/折行 under streaming |

> DoD:每 case 重放全程无跳变(肉眼 + 截图回归)、无掉帧、atlas 不 thrash、NodeId 稳定。截图快照回归(5D4)留后续。
