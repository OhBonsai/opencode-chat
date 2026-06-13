---
name: replay-debug
description: 确定性录像重放调试 — 录制 / 重放 / 单帧步进 / HUD 观测。触发场景:用户说 "/replay-debug"、"重放复现"、"录一段"、"单帧调试"、"看 HUD" 时执行。
---

# /replay-debug · 确定性重放调试

> 本项目调试基石(testing §0):管线是事件驱动 + 固定 dt,录下 SSE+输入流即可逐帧复现。
> 触发先 Read DEVMEM § 2。

## 何时用

- 用户报的现象难复现 → 拿录像本地重放,100% 复现
- 流式/动画/容错的 bug → 单帧步进定位
- 配合 `/dev-diagnose`:诊断的"复现步骤"= 一条录像 + 重放命令

## 1 · 录制

```bash
# 开 harness 时打开录制,产出 (t, event) 流
OPENCODE_CHAT_RECORD=spec/diagnose/replays/<slug>.jsonl <run-harness>
```
录像记 transport(SSE)+ input 两路;dt 用墙钟值一并记下。

## 2 · 重放

```bash
# native 重放纯逻辑(无浏览器),断言/打印最终 store
cargo test --test replay -- <slug>

# 浏览器内重放(含渲染),用 Player 替换 transport,dt 取录像值
<run-harness> --replay spec/diagnose/replays/<slug>.jsonl
```

## 3 · 单帧步进 + HUD(egui,debug build)

debug HUD(feature `dev-hud`)叠在画布上,实时显示:
- 帧时间 p50/p95、dropped frames、可见 instance 数
- 各 Active Part/Turn 的 FSM 状态、smoother 各队列 backlog
- atlas 占用/淘汰率、跨界调用次数+字节、当前 render backend 档位
- 录像回放控制:暂停 / 单帧步进 / 拖时间轴

```bash
<run-harness> --replay <file> --features dev-hud --pause
# 空格单帧步进,观察某帧 FSM 迁移 / instance 变化
```

## 4 · 日志与 panic

- `tracing-wasm` → console;级别按 AGENTS §6.5
- `console_error_panic_hook` 保证 wasm panic 有可读 backtrace
- 重放结果不确定 → 必有非确定性源:查 core 是否用了 `now()`/裸 rand(违 R8/R9),
  应走 Clock / seed seam

## 5 · 沉淀

定位后:把录像留在 `spec/diagnose/replays/`,在 diagnose 文档 frontmatter 的 `replay:`
字段引用它(`/doc-write` diagnose 分支)。

## 反模式

- ❌ 靠"多试几次"复现(用录像确定性复现)
- ❌ 重放不一致却不查非确定性源
- ❌ HUD/录制代码进 release(feature-gate `dev-hud`)
