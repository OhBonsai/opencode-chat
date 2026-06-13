---
name: dev-wrap
description: 任务收尾 + 一站式提交。task_id 必传 → 卡口 → rebase → 静态分析 → PR 描述 → squash + commit → 输出 push 命令。触发场景:用户说 "/dev-wrap <task_id>"、"收尾"、"提 PR"、"准备合并" 时执行。
---

# /dev-wrap · 任务收尾 + 一站式提交

> 流程:`/dev-start` → 写代码/测试/文档 → **`/dev-wrap <task_id>`(本)** → `git push`(用户手动)

```
/dev-wrap <task_id>
```

## Hard Rules(违反即终止)

1. 必须传合法 task_id,否则阻断
2. rebase 冲突 → 停止,要求用户解决后重跑
3. 卡口 fail → 阻断,不进 commit/push
4. commit message 必须含 task_id 引用
5. 不 push、不创建 MR(只输出 URL)
6. commit message 必须用户确认

## 阶段零 · 参数校验

解析第一个参数为 task_id;不合法 → 提示错误,终止。

## 阶段一 · DEVMEM § 2 一致性

Read DEVMEM § 2,核对传入 task_id 与 §2 字段。不一致 → 询问。

## 阶段二 · 工作区准备

`git status --porcelain`。不干净 → AskUserQuestion 给 A)临时commit B)stash C)discard(二次确认) D)取消。

## 阶段三 · Rebase

```bash
git fetch origin main
git rebase origin/main
```
冲突 → 停止,告知用户解决后重跑。

## 阶段四 · 卡口(阻断)

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo deny check
cargo test                          # native 纯逻辑
wasm-pack test --headless --chrome  # wasm 边界(若本次涉及边界/渲染)
```
fail → 阻断,不进 commit。哪些本地跑、哪些留 CI 视改动范围定。

## 阶段五 · 静态分析(不阻断)

```bash
git diff --name-only $MERGE_BASE..HEAD
git diff --stat $MERGE_BASE..HEAD
```

- **敏感文件**:`.env`/`*.pem`/`*.key`/`credentials.*` → 强调安全风险
- **量级 + 文档命中**:

| commit type · size | 期望命中 | 缺失动作 |
|---|---|---|
| feat/refactor · M | spec ≥ 1 | 提醒补 spec |
| feat/refactor · L | decision ≥ 1 + spec ≥ 1 | 提醒补 decision + spec |
| fix · non-trivial | diagnose ≥ 1 + 更新 ISSUE_INDEX | 提醒补诊断 |
| docs/chore/test | — | 不检查 |

- **构建配置**:`Cargo.toml`/`deny.toml`/`package.json` 命中 → 提示确认
- 命中项一次性用 AskUserQuestion 列出,不阻断

## 阶段六 · PR 描述

用 DEVMEM §2 + git log + diff 生成:
```markdown
## Summary
- <1-3 bullet>
## Test plan
- [ ] <验证 1>
## Risk
<潜在影响;无则"无">
```

## 阶段七 · Squash + commit

- 1 commit → `git commit --amend`;多 commit → `git reset --soft $MERGE_BASE && git commit`
- 标题:`type(M<n>-<module>): summary`
- 正文 bullet;末尾 `to #<task_id>` + `Co-Authored-By: Claude <model> <noreply@anthropic.com>`
- 展示 → A 确认 / B 调整 / C 取消,循环到 A

## 阶段八 · DEVMEM § 2 清空

经验摘到 §3;§2 全字段重置空态;§1 凭证 / §3 不动。

## 阶段九 · 输出

```
## ✅ /dev-wrap 完成
**Rebase**: pass  **卡口**: <摘要>  **Size**: <S/M/L>
**Commit**: <hash> <title>  **Branch**: <branch>

## PR 描述
<阶段六>

## 下一步
  git push origin <branch>
```

## 反模式

- ❌ 跳过卡口 / test fail 标"已知 flaky"先合
- ❌ commit message 不展示直接提交 / 自动 push
