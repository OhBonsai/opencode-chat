---
name: doc-write
description: 写 spec / plan / decision(ADR)/ diagnose。触发场景:用户说 "/doc-write"、"写个 spec"、"列测试用例"、"做个 plan"、"写个决策/ADR"、"记录决策" 时执行。
---

# /doc-write · 写 spec / decision / plan / diagnose

> 触发先 Read DEVMEM § 2 拿 domain + 关键决策。新 decision 编号 / spec 路径完成后追加到 §2。

## 第 0 步 · 四选一

| 形态 | 选这个 | 判据 | 位置 |
|---|---|---|---|
| 单 PR 方案 | **spec** | 一文件讲完 | `spec/plan/`(或随 PR) |
| 多阶段/多 PR | **plan** | 分 N 阶段、每阶段交付物 | `spec/plan/planN-<slug>.md` |
| 已拍板留证据 | **decision(ADR)** | append-only,合并即冻结 | `spec/decision/NNNN-<slug>.md` |
| bug 根因沉淀 | **diagnose** | 配 bug fix,给后人警示 | `spec/diagnose/NNN-<slug>.md` |

domain 来源:DEVMEM §2。本项目 domain = 13 模块 M1-M13(见 AGENTS §4)。

## 第 0.5 步 · 事前 / 事后

```bash
git fetch origin main -q
git log --oneline origin/main..HEAD | head -1
git diff --shortstat origin/main..HEAD
```
干净分支 → **事前**(走问询);已有 commit → **事后**(从 diff + DEVMEM 反推草稿)。

## 分支 A · spec / plan

事前 7 问:What / Why / How(2-3 步)/ Test(3-5 case)/ Risk / Scope(不做什么)/ Done。
事后:diff 反推草稿 → 展示 → A/B/C 修订;空的 Scope/Risk/Done 单独问。

模板:
```markdown
# Spec · <标题>
## What / Why
## How
## 测试用例提纲
- [ ] 正常: …  - [ ] 边界: …  - [ ] 错误: …
## Scope · 不做什么
## Risk / Open Questions
## Done
## 关联
- decision: <link 或 无>  · Code 入口: <file:line>
```

## 分支 B · decision(ADR)

- 编号:读 `spec/decision/` 取最大 `NNNN`,**+1**(全局连续;现状到 0008,新建从 0009)
- 文件名:`NNNN-<kebab-slug>.md`,**直接放 `spec/decision/`**(不分子目录)
- 模板:
```markdown
# 决策记录 NNNN:<标题>
- 日期: YYYY-MM-DD
- 状态: 已采纳 | Proposed | Superseded by NNNN
- 前置: <相关 decision>
- 范围: <一句话>

## 1. 背景 / 问题
## 2. 决策
## 3. 备选与否决理由
## 4. 影响(正/负)
## 5. 实现入口(file:line 或 无)
```
- **append-only**:写完不改;新决策覆盖旧的时,旧文件头加 `Superseded by: NNNN`。

## 分支 D · diagnose

- 编号:`spec/diagnose/` 取最大 NNN+1,3 位前导零
- frontmatter:id / title / domain(M<n>)/ status / severity / date / symptoms / keywords / related_code / **replay**(录像路径,本项目特有)
- 模板:症状 → 根因(分层带证据)→ 修复 → **踩坑要点 ★** → 验证方法(UI 动作 + 日志 grep + 录像重放命令)
- **强制**:写完追加 `spec/diagnose/ISSUE_INDEX.md` 对应 domain 一行

## 反模式

- ❌ decision 写"待定"(那是 spec) / 跳号 / 改老 decision
- ❌ spec 不写"不做什么"
- ❌ 事后模式无脑 7 问(直接 diff 反推)
- ❌ diagnose 写完不更新 ISSUE_INDEX
