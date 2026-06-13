---
name: test-write
description: 写测试 — 自动加载反 flaky + 确定性铁律。触发场景:用户说 "/test-write"、"写测试"、"加 unit test"、"补回归测试" 时执行。
---

# /test-write · 写测试

> 触发先 Read DEVMEM § 2 拿 domain + 关键决策。完整见 [dev-practices §4.4](../../spec/dev-practices.md)。

## 铁律 T(写每个 case 前必看)

| # | 铁律 | 违反代价 |
|---|---|---|
| **T1** | 测试禁 `sleep`/`setTimeout`;wait 必有显式条件 | flaky |
| **T2** | 每 case 自建 fixture,禁靠执行顺序 | flakiness 头号根因 |
| **T3** | 禁打真网络/真服务 → mock Connection(seam) | CI 不稳 |
| **T4** | 随机数据必 seed(`StdRng::seed_from_u64`) | Heisenbug |
| **T5** | 断言用户可见行为/最终状态,不断言私有调用 | 重构即崩 |
| **T6** | **纯逻辑(M2-M6,M13)native `cargo test`,禁拉浏览器** | 慢、难调 |
| **T7** | **容错/收尾/标签用 proptest 编码不变量(乱序/丢/重→快照态)** | 边界漏测 |
| **T8** | **视觉测试用 off-profile golden(确定性几何)+ 像素 diff** | 动画致 flaky |
| **T9** | **修 bug 先写复现(优先录像重放)再 fix,先红后绿** | 回归复发 |
| **T10** | flaky 不靠重跑掩盖,排根因 | 掩盖真问题 |

## 选层级 + 框架

| 用例特性 | 层级 | 工具 |
|---|---|---|
| 纯逻辑单函数(M2-M6,M13) | 单测 | `cargo test`(native) |
| 容错/收尾/标签不变量 | 属性 | `proptest` |
| wasm 边界(M1/M7/M10) | wasm 单测 | `wasm-bindgen-test` headless |
| 渲染正确性 | 视觉快照 | off-profile readback + PNG diff |
| 整管线 | 集成 | 录像重放 |
| 修 bug | 回归 | 先红后绿 + 录像 |

## 不变量怎么写成 proptest(本项目重点)

```
任意事件流扰动(乱序/重复/丢失) → 重放后 store 最终状态 == 快照状态   # AR4/AR5
任意状态序列 → FSM 不 panic,落合法终态                              # AR5
任意截断点 → 标签不漏字面字符上屏 / markdown 补全后可解析            # AR9
固定 dt → 吐字总数守恒,grapheme 不被切碎                            # AR7
```

## 修 bug 的回归流程(先红后绿)

1. 先写复现测试(优先用录像重放),跑一次确保 fail
2. 提交"fail 的测试" commit
3. 写 fix
4. 再跑确保 pass
5. PR 贴:"Step 2 = fail, Step 4 = pass";配 diagnose 文档

## 跑测试

见 `/test-run`。fail = 阻塞合;flaky 重跑 3 次不一致 = 排根因(wasm flaky 先疑时间/随机未走 seam)。

## 反模式

- ❌ `sleep`/`setTimeout` 等异步 / 测试调真服务 / 测试间共享 state
- ❌ 纯逻辑硬拉浏览器测(违 T6)
- ❌ 修 bug 不写回归 / flaky 重跑直到绿
