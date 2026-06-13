---
name: dev-start
description: 任务启动 — 同步远端 / 检查工作树 / 创建分支 / 加载 AGENTS / 评估 S/M/L 规模 / 推荐 skill 链 → 输出"可以开发了"。触发场景:用户说 "/dev-start"、"开始开发"、"新任务"、"新功能"、"修个 bug" 时执行。
---

# /dev-start · 任务启动

> 流程:**`/dev-start`(本)** → 写代码/测试/文档(rust-write / render-write / bridge-write / test-write / doc-write) → `/dev-wrap <task_id>` → `git push`

准备**完整开发上下文**,最后输出"可以开发了"就停,不替用户写代码。

## 0 · 读 DEVMEM § 2(残留检查)

Read `DEVMEM.md` § 2。若残留上个任务未清空 → 问:"上轮 `<intent>` 似乎没走 dev-wrap,继续还是开新?"
继续 → 复述 §2 直接到 step E;开新 → 走完整 A-F。

## A · 同步远端 + 检查工作树

```bash
git fetch origin main
git status --porcelain
```

工作树不干净 → 用 AskUserQuestion 给:A)临时 commit B)stash C)discard(二次确认) D)取消。
**绝不 silent 改 git 状态**。

## B · 听需求 + 建分支

- B.1 必问 1 句(用户已说清则跳过):"你这次要干什么?一句话。"
- B.2 **域定位**:读 AGENTS.md § 4 域速查(13 模块 M1-M13),确定落在哪个模块。
  只读所涉 1-2 模块对应的 decision/architecture 章节,不整读。
- B.2.5 **历史诊断扫描**(0 成本不阻断):`grep -i -B1 -A2 "<关键词>" spec/diagnose/ISSUE_INDEX.md`,命中则提示先看对应诊断的「踩坑要点」。
- B.3 **分支名**:`<type>/M<n>-<module>/<name>_<MMDD>`
  type ∈ feat/fix/refactor/chore/docs/ci;module 用模块名;name kebab-case <30 字符。
- B.4 `git checkout -b <new-branch> origin/main` — 真的把分支建出来。

## C · 加载 AGENTS + 评估规模

- C.1 Read AGENTS.md 拿 L0 全局视图。
- C.2 规模:**S** 单文件 <50 行/纯文档配置 · **M** 多文件单模块 <500 行 · **L** 跨模块 OR >500 行 OR 含 decision/架构变更。
- C.3 推荐 skill 链:

| 意图 / 规模 | 推荐链 |
|---|---|
| 改 core/逻辑(M2-M6,M13)· S | `/rust-write` → `/dev-wrap` |
| 改 core/逻辑 · M-L | `/doc-write`(spec) → `/rust-write` → `/test-write` → `/dev-wrap` |
| 改渲染(M8-M10,wgpu/WGSL)| `/render-write` → `/test-write` → `/dev-wrap` |
| 改 `web/` 桥 | `/bridge-write` → `/dev-wrap` |
| 加/改测试 | `/test-write` → `/dev-wrap` |
| 跑测试/录像场景 | `/test-run` |
| 报错/日志看不懂 | `/dev-diagnose` → `/replay-debug` |
| 写 spec/decision/plan | `/doc-write` → `/dev-wrap` |
| L 跨模块大改 | **先 `/doc-write` 写 decision + spec → 再拆 PR** |

## D · 澄清问题(最多 3 问)

只问对决策有影响的,上下文清楚就跳过。每问给 2-4 个选项(AskUserQuestion)。

## E · 收尾输出(严格格式)

```
✅ 上下文已就位:
1. **分支**:`<branch>`(已建好)
2. **域**:M<n> <module> · <一句话定位>
3. **预估规模**:<S/M/L> · <理由>
4. **建议流程**:<skill-1> [→ …] → /dev-wrap <task_id>

可以开发了 — 你直接说怎么改,我开干。
```

### E.1 主动加载下一个 skill 的铁律

chain 起手是 `/rust-write` → `Skill(skill="rust-write")`;`/render-write` → 同理;
`/bridge-write` / `/test-write` / `/doc-write` → 同理。让铁律先入上下文。

dev-start 到此**结束**,不追问"开始吗?"。

## F · 写 DEVMEM § 2

Edit DEVMEM § 2:task_id/branch/domain/intent/size/chain/强制约束/空决策列表。
强制约束按域选(core→rust-write+AR 清单;render→render-write;web→bridge-write)。

## 反模式

- ❌ 不替用户写代码 — dev-start 是上下文准备器
- ❌ silent 改 git 状态
- ❌ 澄清 >3 问 / 输出 E 后追问"确认开始吗?"
