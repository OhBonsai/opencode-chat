---
name: test-run
description: 跑测试 + 录像重放场景。触发场景:用户说 "/test-run"、"跑测试"、"重放这条会话"、"复现场景" 时执行。
---

# /test-run · 跑测试 / 录像重放

> 触发先 Read DEVMEM § 2 拿当前 domain。

## 命令速查

```bash
# native 纯逻辑(M2-M6,M13)— 快
cargo test
cargo test -p opencode-chat-core           # 单 crate

# 属性测试(容错/收尾/标签不变量)
cargo test --features proptest

# wasm 边界(M1 transport / M7 layout / M10 render)
wasm-pack test --headless --chrome
wasm-pack test --headless --firefox

# 视觉快照(off-profile golden 像素 diff)
cargo test --features visual-snapshot

# 微基准
cargo bench
```

## 录像重放(本项目特有)

确定性重放是复现/调试的基石(testing §0)。

```bash
# 用已录会话重放整条管线,断言最终 store 状态
cargo test --test replay -- <replay-name>

# 列可用录像
ls spec/diagnose/replays/ tests/replays/
```

重放结果不确定 → 八成违反了 R8/R9(core 用了 now() 或裸 rand)→ 查 Clock/seed seam。

## 故障注入场景

```bash
# 注入:丢/重/乱序事件、断流重连、忘了 idle、标签跨 delta、embed 失败、降级
cargo test --test fault_injection -- <scenario>
```

## 结果处理

- fail = 阻塞合
- **flaky 重跑 3 次不一致 = 排根因,不靠重跑掩盖**;wasm flaky 优先怀疑时间/随机未走 seam
- 截图 diff fail → 看 diff 图,确认是回归还是 golden 该更新(更新需 review)

## 反模式

- ❌ 靠重跑掩盖 flaky
- ❌ 无脑更新 golden(必须确认非回归)
