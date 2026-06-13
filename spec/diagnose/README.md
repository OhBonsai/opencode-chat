# spec/diagnose · 诊断经验沉淀

非 trivial bug fix 的根因 + 踩坑要点,留给后人。由 `/doc-write` diagnose 分支生成,
`/dev-diagnose` 配合 `/replay-debug` 复现。

## 约定

- 文件名:`NNN-<slug>.md`,NNN 三位前导零全局递增
- 写完**必须**追加 [ISSUE_INDEX.md](./ISSUE_INDEX.md) 对应模块(M<n>)一行
- frontmatter `replay:` 引用 `replays/` 下的录像(本项目确定性重放,见 testing §0)
- 配回归测试(先红后绿)

## 目录

```
diagnose/
├── README.md              # 本文
├── ISSUE_INDEX.md         # 索引(grep 入口)
├── replays/               # 复现用录像(.jsonl,(t,event) 流)
└── NNN-<slug>.md          # 各诊断文档
```

## 模板(frontmatter + 正文)

```yaml
---
id: NNN
title: 一句话症状
domain: M<n>
status: resolved        # active | partial | resolved | superseded
severity: high          # low | medium | high | critical
date: YYYY-MM-DD
symptoms: [用户感知症状]
keywords: [grep 关键词]
related_code: [path/to/file.rs]
replay: spec/diagnose/replays/<slug>.jsonl
---
```

正文:症状 → 根因(分层带证据)→ 修复 → **踩坑要点 ★** → 验证(UI 动作 + 日志 grep + 重放命令)。
