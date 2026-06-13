---
last_reviewed: 2026-06-13
layer: L0
domain: cross
---

# AGENTS.md · opencode-chat · AI 代理入口

> 仓库唯一的 AI 代理入口。新 session / 新人 / 新任务,从这里开始。

## 1. 项目一句话

opencode-chat = Rust + WebAssembly + wgpu 的 **LLM SSE 对话渲染引擎**:消费 opencode
server 的事件流,在 GPU 画布上做流式、高动效的对话渲染;交付为框架无关的 wasm 组件
(React/Vue 可用)。本质是"实体为文字的游戏客户端"。

## 2. 第一次进项目必读

1. **本文 AGENTS.md**(项目地图)
2. **[spec/README.md](./spec/README.md)**(文档总导航 + 阅读顺序)
3. **[spec/0000-overview / decision/0000-overview.md](./spec/decision/0000-overview.md)**(为什么做、做什么)
4. **[spec/architecture.md](./spec/architecture.md)**(13 模块 = 域;动态视图)
5. **[spec/dev-practices.md](./spec/dev-practices.md)**(铁律 AR/CR/R/T + 技能 + 文档规范)

## 2.5 AI Rules(10 条)

适用于本项目每个任务。非平凡工作上偏保守;平凡任务用判断力。

1. **Think Before Coding** — 不确定就问,不猜;歧义时列多种解读。
2. **Simplicity First** — 最少代码解决问题,不投机抽象。
3. **Surgical Changes** — 只改必须改的,不顺手"改进"邻近代码。
4. **Goal-Driven Execution** — 定义成功标准,循环验证。
5. **Surface conflicts** — 模式冲突时选一个(更新/更可测)并说明,不平均。
6. **Read before write** — 写之前读 exports / 调用方 / 共享工具。
7. **Tests verify intent** — 测试编码 WHY,不只是 WHAT。
8. **Checkpoint** — 每步总结已做/已验证/待办。
9. **Match conventions** — 匹配代码库风格,异议先 surface 不静默 fork。
10. **Fail loud** — 跳过任何步骤就不算"完成";不静默隐藏不确定性。

## 3. 文档分层

| 层 | 是什么 | 在哪 |
|---|---|---|
| **L0** 入口 | AGENTS / spec/README | 仓库根 + `spec/` |
| **L1** 约定 | 铁律 / 流程规范 | [spec/dev-practices.md](./spec/dev-practices.md) |
| **L2** 决策 ADR | append-only 编号决策 | [spec/decision/](./spec/decision/) `0000-` |
| **L3** 参考 | 业界调研 | [spec/research/](./spec/research/) |
| **L4** 解释 | 模块动态视图 | [spec/architecture.md](./spec/architecture.md) |
| plan | 多阶段开发方案 | [spec/plan/](./spec/plan/) |
| diagnose | 诊断经验沉淀 | [spec/diagnose/](./spec/diagnose/) |

## 4. 域速查(= architecture.md 的 13 模块)

| 域 | 模块 | 职责 | OWNER |
|---|---|---|---|
| **M1** | transport | SSE 接入/重连/看门狗/快照 | TBD |
| **M2** | protocol | 事件 + Part serde 解码 | TBD |
| **M3** | store | 归一化文档三表 + 对账 | TBD |
| **M4** | fsm | Part/Turn/Tag 状态机 + 收尾 | TBD |
| **M5** | smoother | 流式节奏整流 | TBD |
| **M6** | content | 标签扫描 + markdown + 高亮 | TBD |
| **M7** | layout | pretext 排版桥 | TBD |
| **M8** | scene | atlas/instance/embed/裁剪 | TBD |
| **M9** | effects | tween + shader + profile | TBD |
| **M10** | render | wgpu 后端 + 相机 | TBD |
| **M11** | input | 滚动/选区/hit-test | TBD |
| **M12** | api | 宿主 API + 回调 + 无障碍镜像 | TBD |
| **M13** | app | 每帧编排循环 | TBD |
| **M0** | cross | 跨模块 | TBD |

**Rust workspace**:`crates/{core,render,wasm}`(见 [plan1 §三.5](./spec/plan/plan1-minimal-prototype.md))
**前端 harness**:`web/`
跨域决策 → `spec/decision/`(全局编号,不分子目录)。

## 5. Skill 间共享上下文 · `DEVMEM.md`

仓库根 `DEVMEM.md`(**已 gitignored**)是 skill 间本地 scratchpad:
- **§1 长期凭证** · **§2 当前任务**(dev-start 写,dev-wrap 清) · **§3 历史选择** · **§4 备注**

| 时机 | 动作 |
|---|---|
| 任何 skill 触发 | 先 Read DEVMEM §2 |
| `/dev-start` | 写 §2 task_id/branch/domain/intent/chain/强制约束 |
| 写代码/测试期间 | 关键决策追加 §2 |
| `/dev-wrap <task_id>` | 读 §2 拼 commit → 清空 §2 |

**不写**:能从代码查到的事实 / throwaway grep 输出 / 跨项目通用经验。

## 6. Skill 列表(`.claude/skills/`)

| Skill | 何时用 |
|---|---|
| `/dev-start` | 新任务:同步远端·建分支·评估 S/M/L·推荐链·写 DEVMEM §2 |
| `/dev-wrap <task_id>` | 收尾:卡口+rebase+squash+commit·清 §2 |
| `/doc-write` | 写 spec / plan / decision(ADR)/ diagnose |
| `/rust-write` | 写 Rust(自带 AR/CR/R 铁律) |
| `/render-write` | 写 wgpu/WGSL 渲染(自带渲染铁律) |
| `/bridge-write` | 写 `web/` 薄 TS 桥(pretext/glyph/harness) |
| `/test-write` | 写测试(自带 T 铁律 + native 优先 + proptest) |
| `/test-run` | 跑测试 / 录像重放场景 |
| `/dev-diagnose` | 诊断:录像重放复现 → hypothesis |
| `/replay-debug` | 录制/重放/单帧步进 + HUD 调试 |
| `/weekly-summary` | 迭代周报 |

## 6.5 日志规则(给 /dev-diagnose 留素材)

关键路径必留 INFO(里程碑)+ ERROR(失败):transport 连接生命周期、fsm 迁移、
对账触发、降级切换、layout 跨界异常、embed 加载失败。
级别:**ERROR** 用户感知失败 / **WARN** 重试降级 / **INFO** 里程碑 / **DEBUG** 默认关。
`tracing` 的 source 字段填**模块名(M<n>)** 便于 grep。

## 7. 卡口命令

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo deny check
cargo test                          # native 纯逻辑
wasm-pack test --headless --chrome  # wasm 边界
```

任一 fail = 阻塞,不硬合。

## 8. 不要做的事

- ❌ 改老 decision(ADR)内容(append-only,新决策写 superseding)
- ❌ 违反 [dev-practices §4](./spec/dev-practices.md) 的 AR/CR 不变量铁律
- ❌ 在 `core` crate 引 wasm-bindgen/web-sys/wgpu(破坏 native 可测,CR1)
- ❌ core 用 `now()`/裸 rand(破坏确定性重放,R8/R9)
- ❌ 测试 flaky 重跑直到绿(找根因)
- ❌ 跳过任一卡口先合
- ❌ commit DEVMEM.md(已 gitignored)
