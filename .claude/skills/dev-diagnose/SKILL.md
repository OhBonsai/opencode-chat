---
name: dev-diagnose
description: 开发问题诊断 — 录像重放复现 → 读日志 → 给 hypothesis → 沉淀 diagnose。触发场景:用户说 "/dev-diagnose"、"启动报错"、"日志看不懂"、"这个现象怎么回事" 时执行。
---

# /dev-diagnose · 问题诊断

> 触发先 Read DEVMEM § 2 拿 domain。本项目杀手锏:**录像重放确定性复现**(见 `/replay-debug`)。

## 1 · 先查历史诊断(0 成本)

```bash
grep -i -B1 -A2 "<现象关键词>" spec/diagnose/ISSUE_INDEX.md
```
命中 → 先看对应 `spec/diagnose/NNN-*.md` 的「踩坑要点 ★」,可能直接命中。

## 2 · 复现(优先录像重放)

- 有录像 → `cargo test --test replay -- <slug>` / harness `--replay`(确定性复现)
- 无录像 → 引导用户开 `OPENCODE_CHAT_RECORD=...` 录一段触发现象的会话,再重放
- 复现后用 `--pause` 单帧步进 + dev-hud 观察(见 `/replay-debug`)

## 3 · 读日志 / 状态

- console 日志按 source(M<n>)grep,定位出问题的模块
- HUD 看:FSM 卡在哪个状态、smoother backlog 是否异常、对账是否触发、backend 档位
- 按模块归因:
  - 画面抽搐 → M5 smoother / M13 帧编排
  - 丢字/错字 → M3 store 对账(AR4)/ M1 transport 丢事件
  - 永久 loading → M4 收尾判定(AR8)/ M1 心跳看门狗
  - 闪烁/重排 → M6 content 块边界(AR9)
  - 卡顿 → M8 裁剪(RD5)/ M10 跨界(AR10)
  - 字形乱码 → M5/M6 grapheme 单位(AR7)

## 4 · 给 hypothesis(分层带证据)

不直接改代码,先输出:症状 → 可能根因(按概率排序,每条附日志/HUD 证据)→ 验证方法。
不确定 → 列 2-3 个 hypothesis 让用户选,或建议加临时日志再重放。

## 5 · 沉淀(非 trivial 必做)

修复后走 `/doc-write` diagnose 分支:
- `spec/diagnose/NNN-<slug>.md`:症状 / 根因 / 修复 / **踩坑要点 ★** / 验证(含录像路径)
- frontmatter `replay:` 引用录像;**强制**追加 `ISSUE_INDEX.md` 对应 domain 一行
- 配回归测试(`/test-write` 先红后绿)

## 反模式

- ❌ 不查历史诊断就从头查
- ❌ 靠多试复现(用录像)
- ❌ 直接改代码不先给 hypothesis
- ❌ 修完不沉淀 diagnose / 不更新 ISSUE_INDEX
