---
name: rust-write
description: 写 Rust 代码 — 自动加载架构不变量 + 分层 + Rust 铁律。触发场景:用户说 "/rust-write"、"写 Rust"、"改 Rust"、改 crates/core 或 crates/render 的 .rs 时执行。
---

# /rust-write · 写 Rust 代码

> 触发先 Read DEVMEM § 2 拿 domain + 关键决策。做非显然设计选择时 Edit §2 决策列表追加一行。
> 完整铁律见 [spec/dev-practices.md §4](../../spec/dev-practices.md)。下面是写每行必看的速查。

## A. 架构不变量铁律 AR(违反即破坏设计,最优先)

| # | 铁律 | 来源 |
|---|---|---|
| **AR1** | 事件改状态,渲染只读状态 — render 路径禁写 store | 0002 |
| **AR2** | effects 只读核心、禁回写;model/presentation 字段分离 | 0002 |
| **AR3** | 效果 `age>=duration` 后恒等;hit-test/选区/滚动只读 settled 几何 | 0002 |
| **AR4** | delta 乐观追加必须配 part.updated 全量对账,禁只靠 delta | 0003 |
| **AR5** | FSM 投影语义:任意态→任意态合法,禁拒绝迁移 | 0003 |
| **AR6** | catch-up(快照/重连/分页)零动画零平滑 | 0003 |
| **AR7** | 吐字/光栅化单位 = grapheme cluster,禁按码点切 | 0002 |
| **AR8** | 收尾禁只看 idle;多信号 + 看门狗 + 快照对账 | 0005 |
| **AR9** | 流式前沿留 hold 区,禁提交歧义标签/语法字节 | 0006 |
| **AR10** | 每帧最多一次 wasm↔pretext 批调用,禁逐 part 跨界 | 0001 |
| **AR11** | Turn 等聚合是投影,禁单独存储 | 0005 |
| **AR12** | 未知事件/part 类型 → Ignored,禁 panic | 0003 |

## B. 分层 / crate 铁律 CR

| # | 铁律 |
|---|---|
| **CR1** | `core` crate 零 wasm-bindgen/web-sys/wgpu,保 native 可测 |
| **CR2** | 平台能力(网络/排版/时钟/渲染)走 seam trait,core 禁直接依赖实现 |
| **CR3** | render 后端用 trait 选择,禁 cfg 堆后端 |
| **CR4** | 跨界结果用平铺 TypedArray 零拷贝,禁传对象数组 |
| **CR5** | `wasm` crate 只暴露薄 `#[wasm_bindgen]` API,业务逻辑全在 core |

## C. Rust 铁律 R

| # | 铁律 | 违反代价 |
|---|---|---|
| **R1** | 生产路径禁 `unwrap()/expect()` → `?` + thiserror/anyhow | panic 即 crash |
| **R2** | 禁 `println!/dbg!/eprintln!/todo!` → `tracing::*` | 噪声/提交不掉 |
| **R3** | 每条 `#[allow(...)]` 带 `// reason:` | "为什么"漏失 |
| **R4** | 禁 `pub` 字段 → getter/builder 或 `pub(crate)` | 状态泄漏 |
| **R5** | 每 crate README + 顶层 `//!` mod doc | AI 缺上下文 |
| **R6** | 错误必带上下文(`.context()` / thiserror) | 无法排查 |
| **R7** | 关键路径必有 `tracing` 日志,source = 模块名 M<n> | 诊断无线索 |
| **R8** | **core 禁 `SystemTime::now()`/`Instant::now()` → 走 Clock seam** | 破坏确定性重放 |
| **R9** | **core 禁裸 `rand` → 可注入种子** | 破坏确定性重放 |
| **R10** | core/wasm 禁 `std::fs`/`std::net` → web-sys/seam | wasm 编译失败 |
| **R11** | 共享可变态优先 `Rc<RefCell>`(单线程);跨 worker 才用同步原语 | 过度同步/误用 |

## 4 步流程

1. **读 context**:相关 decision + spec(无 spec 先要一句话需求)
2. **选 crate/模块**:确认改 M<n>(core/render/wasm 哪个)
3. **写代码 + 自检**:每行对照——
   - 改 store/逻辑?→ 过一遍 AR1-AR12 相关项
   - 在 core?→ 无 wasm-bindgen/now()/rand/fs(CR1,R8-R10)
   - 每个 unwrap → `?`(R1);每个 println → tracing(R2);每个 pub 字段 → pub(crate)(R4)
   - 关键路径 → 有日志且 source=M<n>(R7)
4. **跑 lint**:`cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --check`

## 反模式

- ❌ 写 Rust 不看 AR/CR 铁律
- ❌ 在 core 引平台依赖 / 用 now()/rand
- ❌ `.unwrap()` + `// TODO: handle error`
- ❌ render 路径回写 store(违 AR1)
