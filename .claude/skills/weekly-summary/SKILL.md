---
name: weekly-summary
description: 生成迭代周报。触发场景:用户说 "/weekly-summary"、"写周报"、"本周做了什么" 时执行。
---

# /weekly-summary · 迭代周报

> 从 git 历史 + decision/plan/diagnose 变更生成,不替用户编内容。

## 1 · 收集

```bash
git log --since="1 week ago" --oneline --no-merges
git diff --stat "@{1 week ago}"..HEAD
ls -t spec/decision/ spec/diagnose/ spec/plan/ | head
```

## 2 · 按域(模块)归类

把 commit 按标题里的 `M<n>-<module>` scope 归到 13 模块,统计各模块进展。

## 3 · 输出模板

```markdown
# 周报 · <YYYY-MM-DD 周>

## 本周进展(按模块)
- **M<n> <module>**: <一句话 + 关键 commit>

## 决策 / 文档
- 新增 decision: <NNNN 标题>
- 新增 diagnose: <NNN 标题>

## Plan 进度
- <planN>: <阶段 X/Y,本周完成的相位>

## 风险 / 阻塞
- <若有>

## 下周
- <计划>
```

## 反模式

- ❌ 把 commit message 原样堆上去(要按模块归纳)
- ❌ 编造没发生的进展
