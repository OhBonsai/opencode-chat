# 决策记录 0005:Turn 聚合层与回合收尾判定

- 日期:2026-06-13
- 状态:已采纳(原型验证前)
- 前置:0002(事件管线 + 状态机)、0003(容错 + 状态同步)、0004(markdown + 嵌入)
- 范围:assistant 多消息的视图聚合、Turn FSM、回合收尾的多信号判定与超时兜底

## 1. 背景

用户感知的"一来一回",数据层实为一个 message group:一条 user message 之下挂
若干条 assistant message,每条含多个 step(step-start/step-finish)和多个 part
(reasoning/tool/text)。模型因工具往返、重试、继续生成而产生多条 assistant
message。需要一个视图层的"turn(回合)"聚合,以及可靠的收尾判定。

## 2. Turn 聚合(纯投影)

Turn 不存在于数据模型,渲染时从扁平 message/part 表算出。与 0003 投影语义一致:
不单独存储,事件乱序/重连重拉后重新投影即可,turn 边界自然恢复。

```
Turn = {
  user: UserMessage,
  assistant: AssistantMessage[],   // 锚点到下一个 user 锚点之间的全部
}
```

- 分组键 = "下一条 user message 的边界"(opencode 桌面端 constructMessageRows 同款)
- **扁平化 part 流,跨 message 拼接**:回合内所有 assistant message 的 part 按
  时间铺平(reasoning → tool → text → tool ...),message 边界对用户不可见
- **噪音边界折叠**:step-start/step-finish/snapshot/patch 不渲染(0002 噪音过滤),
  仅用于驱动分组与结算
- **中断/压缩插分隔线**:回合内 `MessageAbortedError`/compaction 处插 TurnDivider,
  不断成两个回合,保持"一来一回"视觉完整
- **结算聚合**:多条 message 的 cost/tokens 累加,显示在回合末尾一处

## 3. Turn FSM

Turn 是高于 Part 的聚合实体,带 FSM:

```rust
enum TurnState {
    Active,    // session busy,part 持续流入
    Stalled,   // 软超时:疑似卡住,表观降级(光标停闪 + 提示),但不冻结、仍收数据
    Settling,  // 收尾确认,enter: 收尾动画(光标隐、结算栏淡入、工具卡折叠)
    Settled,   // 整个回合冻结进 GPU buffer,作为视口裁剪/驱逐单元(0002 §6)
}
```

Settled 区分来源:
- `settled_by: idle / completed`(terminal,不可复活)
- `settled_by: timeout`(软终态,可复活,见 §5)

## 4. 收尾判定:多信号收敛 + 超时兜底

**核心原则:idle 是收尾最快的证据,不是唯一证据。** `session.status: idle` 是
网络事件,会丢、会因模型/服务端异常永不发出。不能让 idle 成为唯一依据,否则
丢失即卡死(光标常闪、输入框常禁用)。

### 4.1 多信号收敛(任一充分条件即 settle)

- `session.status: idle` —— 最快最权威,正常路径
- 末条 assistant message 带 `step-finish`(正常终止)且 `time.completed` 已置
  —— 内容级证据,idle 丢了也成立
- `message.updated` 将末条 assistant 标记终态(completed/error/aborted)
- `message.error` —— 错误终止(注:ToolStateError 不算,模型可能继续)

正常情况几条几乎同时到,谁先到谁触发;FSM 幂等(0003 投影语义),重复无害。

### 4.2 静默超时看门狗(兜底)

每个 Active turn 维护"最后活动时间",任何属于该 turn 的事件刷新它。两级:

```rust
struct TurnWatchdog {
    last_activity: Instant,
    soft_timeout: Duration,  // ~8s 无任何 part 活动
    hard_timeout: Duration,  // ~30s
}
```

- **soft(~8s 无 part 活动)** → `Stalled`:光标停闪 + 轻量"似乎已结束"提示,
  **不冻结**,数据仍可接收。诚实反馈,避免无限 loading。
- **hard(~30s 无 part 活动)** → 强制 `Settled`(按当前内容),
  标记 `settled_by: timeout`。

### 4.3 用 heartbeat 区分两种沉默(关键)

`server.heartbeat`(每 10s)用来区分:

- **有心跳但无 part = 模型真停了(忘了 idle)** → 触发 settle
- **连心跳都没了 = 连接僵死** → 走 0003 §3.5 重连,**不 settle**

两者处理完全不同,必须分开。

### 4.4 重连快照对账裁决

连接问题导致 idle 丢失时:重连拉快照(0003 §3.4),快照里 session 真实状态
为权威。快照显示 idle/completed 而本地仍 Active → 立即 catch-up settle。
覆盖"idle 在断线窗口发出但未收到"的情况。

### 4.5 新活动可复活(防误判)

超时 settle 可逆:已被 `settled_by: timeout` 收尾的 turn 又收到属于它的
delta/part(模型只是卡很久又继续)→ 允许 Settled 回到 Active,catch-up 补齐
超时期间状态(不重放动画)。误判超时的代价仅一次状态回滚,而非内容丢失。
`settled_by: idle` 不可复活。

### 4.6 用户显式打断

用户点"停止" → 本地立即 settle(乐观)+ 发 interrupt 请求;服务端 abort 事件
回来再对账。不依赖服务端先 idle。

## 5. 判定矩阵

| 信号 | 含义 | 动作 |
|---|---|---|
| `session.status: idle` | 权威结束 | settle(terminal) |
| 末条 message completed / step-finish | 内容级结束 | settle(terminal) |
| `message.error` | 错误终止 | settle(terminal) |
| 有 heartbeat,8s 无 part | 模型疑似卡住 | → Stalled(表观,不冻结) |
| 有 heartbeat,30s 无 part | 模型忘了 idle | settle(timeout,可复活) |
| heartbeat 也断 | 连接僵死 | 重连(非 settle) |
| 重连快照显示已结束 | idle 丢在断线窗口 | catch-up settle |
| 用户点停止 | 显式打断 | 乐观 settle + interrupt |

## 6. 不变量

收尾是"证据收敛 + 超时兜底 + 可复活"的状态判定,**不是等一个事件**。
与 0003 总不变量一致:最终状态由对账(快照)保证,idle 只是"快"而非"准"的来源。
超时阈值(8s/30s/心跳 25s)为初值,需依实测调整。
