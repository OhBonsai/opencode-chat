# 开发实践提取:把 sync 体系适配到本项目

- 日期:2026-06-13
- 来源:`sync/`(之前项目的 AI-native 开发流程体系:AGENTS + 9 skill + DEVMEM + 4 层文档 + ADR/diagnose + 铁律)
- 目的:针对本项目(opencode-chat wasm 对话渲染引擎)抽取该**复用 / 适配 / 新增**的
  开发技能、实践、文档规范

---

## 一、总览:复用 / 适配 / 新增

| sync 资产 | 本项目 | 说明 |
|---|---|---|
| AGENTS.md 入口 | **新增** | 我们缺 L0 入口,需要建 |
| DEVMEM 状态传递 | **复用** | 直接搬,gitignore 的本地 scratchpad |
| 4 层文档分级 | **适配** | 我们已有 `spec/`,需映射 + 补缺口 |
| ADR(append-only 编号) | **已有** | 我们的 `spec/decision/0001-0008` 就是 ADR,继续 |
| 域划分 | **适配** | 我们的"域" = architecture.md 的 **13 模块** |
| dev-start / dev-wrap | **复用** | 任务启动/收尾闭环,改卡口命令即可 |
| doc-write | **复用** | spec/plan/ADR/diagnose 四形态通用 |
| rust-write 铁律 | **适配** | 去 tokio 化(我们偏单线程 wasm),加 wasm/确定性铁律 |
| test-write 铁律 | **适配** | T1-T12 通用,加确定性重放 + 属性测试 |
| frontend-write | **缩小** | 我们前端只有薄 harness/bridge(TS),铁律精简 |
| dev-diagnose | **适配** | 我们的杀手锏是**录像重放复现**,写进流程 |
| 架构不变量 → 铁律 | **新增(最高价值)** | 我们架构有强不变量,落成可执行铁律 |
| render/shader skill | **新增** | wgpu/WGSL 是我们特有,需专属 skill |

---

## 二、文档规范:映射到 spec/

sync 的 4 层 + ADR/diagnose 与我们现状的对应,以及要补的缺口:

| sync 层 | sync 位置 | 本项目位置 | 状态 |
|---|---|---|---|
| L0 入口 | AGENTS.md / *_INDEX | `spec/README.md` + `AGENTS.md` | **要建** |
| L1 约定 | docs/conventions/ | `spec/dev-practices.md`(本文)+ 铁律 | 本文即起点 |
| L2 决策 ADR | docs/decisions/ | `spec/decision/0000-0008` | ✅ 已有 |
| L3 参考 | docs/reference/ | `spec/research/` | ✅ 已有 |
| L4 解释 | docs/explanation/ | `spec/architecture.md` | ✅ 已有 |
| plan | docs/plans/ | `spec/plan/` | ✅ 已有 |
| diagnose | docs/diagnose/ + ISSUE_INDEX | **要建** `spec/diagnose/` | **缺口** |

要补的三个缺口:

1. **`AGENTS.md`(仓库根)+ `spec/README.md`**:L0 入口,一页看全——项目一句话、
   13 模块速查表(域)、卡口命令(build/lint/test)、skill 列表、文档导航。
2. **`spec/diagnose/`**:非 trivial bug fix 后沉淀根因 + 踩坑要点;配 `ISSUE_INDEX`。
   我们有录像重放,诊断文档应附**复现录像路径**。
3. **域索引**:13 模块即域(下节)。

**ADR 编号沿用现状**:我们已用 `0000-0008`,继续 append-only,新决策 `0009+`。
(sync 用 `ADR-NNNN`,我们用 `NNNN-slug`,保持自己的即可,不强改。)

---

## 三、域划分 = architecture.md 的 13 模块

sync 的"域"在我们这里天然就是模块。域编号贯穿 commit scope / 分支名 / 日志 source /
文档目录:

| 域 | 模块 | OWNER |
|---|---|---|
| M1 | transport | TBD |
| M2 | protocol | TBD |
| M3 | store | TBD |
| M4 | fsm | TBD |
| M5 | smoother | TBD |
| M6 | content | TBD |
| M7 | layout | TBD |
| M8 | scene | TBD |
| M9 | effects | TBD |
| M10 | render | TBD |
| M11 | input | TBD |
| M12 | api | TBD |
| M13 | app | TBD |
| M0 | cross(跨模块) | TBD |

分支名:`<type>/M<n>-<module>/<name>_<MMDD>`;commit scope 用 `M<n>`;
日志 `tracing` 的 source 字段用模块名(R12 同款,便于 grep)。

---

## 四、项目专属铁律

### 4.1 架构不变量铁律 AR(最高价值,违反即破坏设计)

直接从 0002/0003/0005/0006 的不变量落成可执行规则,写代码 + review 必查:

| # | 铁律 | 违反代价 | 来源 |
|---|---|---|---|
| **AR1** | 事件改状态,渲染只读状态——render 路径禁写 store | 画面抽搐、状态错乱 | 0002 §2 |
| **AR2** | effects 只读核心状态、禁回写;model/presentation 字段分离 | 动画破坏逻辑、不可测 | 0002 §5.1 |
| **AR3** | 效果在 `age>=duration` 后必须恒等;hit-test/选区/滚动只读 settled 几何 | 选区错位、布局漂移 | 0002 §5.1 |
| **AR4** | delta 乐观追加必须配 part.updated 全量对账,禁只依赖 delta | 丢字不自愈 | 0003 §1 |
| **AR5** | FSM 用投影语义:任意态→任意态合法,禁拒绝"非法"迁移 | 漏事件即卡死 | 0003 §3.1 |
| **AR6** | catch-up 模式(快照/重连/分页)零动画零平滑 | late-join 重放整段动画 | 0003 §3.2 |
| **AR7** | 吐字/光栅化单位 = grapheme cluster,禁按 Unicode 码点切 | emoji/组合字碎成乱码 | 0002 §4.1 |
| **AR8** | 回合收尾禁只看 idle;多信号收敛 + 看门狗 + 快照对账 | 模型忘了 idle 即永久 loading | 0005 §4 |
| **AR9** | 流式前沿留 hold 区,禁提交有歧义的标签/语法字节 | 字面 `<thin`/语法字符闪现 | 0006 §3 |
| **AR10** | 每帧最多一次 wasm↔pretext 批调用,禁逐 part 循环跨界 | 跨界开销爆炸 | 0001 §3.4 |
| **AR11** | Turn 等聚合是投影,禁单独存储 | 乱序/重连后边界错乱 | 0005 §2 |
| **AR12** | 未知事件/part 类型 → `Ignored`,禁 panic | 服务端加类型即崩 | 0003 §3.6 |

### 4.2 分层 / crate 铁律 CR

| # | 铁律 | 违反代价 | 来源 |
|---|---|---|---|
| **CR1** | `core` crate 零 `wasm-bindgen`/`web-sys`/`wgpu` 依赖,保 native 可测 | 纯逻辑测不了、绑死平台 | plan1 §三.5 |
| **CR2** | 平台能力(网络/排版/时钟/渲染)走 seam trait,core 禁直接依赖实现 | 无法 mock、无法重放 | plan1 §三.5 |
| **CR3** | render 后端用 trait 选择,禁用 `cfg` 编译开关堆后端 | 条件编译地狱 | 0003 §5 |
| **CR4** | 跨界结果用平铺 TypedArray 零拷贝,禁传对象数组 | serde 逐字段转换最慢路径 | 0001 §3.4 |
| **CR5** | `wasm` crate 只暴露薄 `#[wasm_bindgen]` API,业务逻辑全在 core | 逻辑测不了、耦合 | plan1 §三.5 |

### 4.3 Rust 铁律(适配 wasm,调整 sync 的 R 系列)

保留通用项,去 tokio 化(wasm 主线程偏单线程),加确定性与 wasm 项:

| # | 铁律 | 违反代价 |
|---|---|---|
| **R1** | 生产路径禁 `unwrap()/expect()` → `?` + `thiserror`/`anyhow`(测试可放开) | panic 即 crash |
| **R2** | 禁 `println!/dbg!/eprintln!/todo!` → `tracing::{debug,info,warn,error}!` | 噪声 / 提交不掉 |
| **R3** | 每条 `#[allow(...)]` 带 `// reason:` 注释 | "为什么"漏失 |
| **R4** | 禁 `pub` 字段 → getter/builder 或 `pub(crate)` | 内部状态泄漏 |
| **R5** | 每 crate 有 README + 顶层 `//!` mod doc | AI 缺上下文写错 |
| **R6** | 错误必带上下文(`.context()` / thiserror enum) | 无法排查 |
| **R7** | 关键路径必有 `tracing` 日志,source = 模块名(M<n>) | 诊断无线索 |
| **R8** | **core 禁 `SystemTime::now()`/`Instant::now()` → 时间走 `Clock` seam** | 破坏确定性重放 |
| **R9** | **core 禁裸 `rand` → 随机走可注入种子** | 破坏确定性重放 |
| **R10** | **core/wasm 禁 `std::fs`/`std::net` → web-sys/seam 替代** | wasm 编译失败 |
| **R11** | 共享可变态优先 `Rc<RefCell>`(单线程);跨 worker 才考虑同步原语 | 过度同步 / 误用 |

> R8/R9 是为 testing §0 的确定性重放服务的——它们和架构 seam(Clock)是一回事。

### 4.4 测试铁律(适配 sync T 系列 + 我们的确定性)

| # | 铁律 | 违反代价 |
|---|---|---|
| **T1** | 测试禁 `sleep`/`setTimeout`;wait 必有显式条件 | flaky |
| **T2** | 每 case 自建 fixture,禁靠执行顺序 | flakiness 头号根因 |
| **T3** | 禁打真网络/真服务 → mock Connection(seam) | CI 不稳 |
| **T4** | 随机数据必 seed | Heisenbug |
| **T5** | 断言用户可见行为/最终状态,不断言私有调用 | 重构即崩 |
| **T6** | **纯逻辑(M2-M6,M13)native `cargo test`,禁拉浏览器** | 慢、难调 |
| **T7** | **容错/收尾/标签用 proptest 编码不变量(乱序/丢/重→快照态)** | 边界漏测 |
| **T8** | **视觉测试用 off-profile golden(确定性几何)+ 像素 diff** | 动画致 flaky |
| **T9** | **修 bug 先写复现(优先录像重放)再 fix,先红后绿** | 回归复发 |
| **T10** | flaky 不靠重跑掩盖,排根因 | 掩盖真问题 |

---

## 五、技能集

### 5.1 直接复用(改卡口命令即可)

- `dev-start` / `dev-wrap`:任务启动/收尾闭环。卡口改为
  `cargo fmt --check && cargo clippy --workspace -- -D warnings && cargo deny check && cargo test`。
- `doc-write`:spec/plan/ADR/diagnose 四形态;ADR 编号接我们 `0009+`。
- `dev-diagnose`:**适配**——复现优先用录像重放(testing §0),诊断文档附录像路径。

### 5.2 适配

- `rust-write`:换成 §4.3 的 R 铁律 + **每次写代码先过 §4.1 AR 不变量清单**。
- `test-write`:换成 §4.4 的 T 铁律,强调 native 优先 + proptest + off-profile 快照。
- `frontend-write` → **缩为 `bridge-write`**:只覆盖 `web/` 的薄 TS(pretext-bridge /
  glyph-raster / harness),铁律精简(类型安全、零拷贝视图 grow 后重建、不在 JS 侧
  解析事件)。

### 5.3 新增(本项目特有)

- **`render-write`**(wgpu/WGSL 专属铁律):
  - 着色器效果靠 `time - spawn_time`,CPU 零参与(0002 §5)
  - 关闭效果 = 参数置零非分支;profile full/reduced/off(0002 §5.1)
  - WGSL 经 naga 构建期校验;GPU 资源 grow 后重建视图
  - instance 按 y 有序,视口裁剪二分(0002 §6)
- **`replay-debug`**(确定性重放调试):录制 → 重放复现 → 单帧步进 + egui HUD 看
  FSM/smoother 水位 → 定位。配 panic hook + tracing-wasm(testing §1)。

### 5.4 DEVMEM 强制约束(契约保底)

`/dev-start` 写 DEVMEM § 2 时,按域注入强制约束。例:
- 改 M2-M6/M13(core)前必 `Skill(rust-write)` + 过 AR 清单
- 改 M8-M10(render)前必 `Skill(render-write)`
- 改 `web/` 前必 `Skill(bridge-write)`

---

## 六、落地建议(下一步,按优先级)

1. **建 L0 入口**:`AGENTS.md` + `spec/README.md`(13 模块速查 + 卡口 + skill + 文档导航)
2. **搬 DEVMEM**:`DEVMEM.template.md` → 根,加 gitignore
3. **固化铁律**:本文 §4 即 L1 约定;`rust-write`/`render-write`/`test-write` skill
   引用它
4. **建 `spec/diagnose/` + ISSUE_INDEX**:首个非 trivial bug 时启用,附录像路径
5. **适配 skill**:复制 sync `.claude/skills/`,按 §5 改卡口/铁律/新增两个 skill
6. **域 OWNER**:13 模块认领后填 §三 的 TBD

> 原则同 sync 的 FAQ:小项目不必全套。**最低配 = AGENTS.md + 本文铁律 + dev-start/dev-wrap**,
> 随项目长大再加 diagnose / OWNER / 更多 skill。
