# 测试、Benchmark 与可观测性方案

- 日期:2026-06-13
- 前置:architecture.md(13 模块)、0002/0003/0005/0006(不变量来源)
- 核心前提:**系统是事件驱动 + 固定 dt,给定事件流可确定性重放**——日志、测试、
  benchmark 全部建立在这条之上。

---

## 零、确定性重放(贯穿三者的基石)

- 整条管线的输入只有两类:**SSE 事件流** + **输入事件流(指针/键盘/resize)** +
  时间步 dt。
- 把这三者录下来(record),就能逐帧重放(replay)出完全相同的状态与渲染。
- 价值:
  - **debug**:用户报 bug → 拿到事件录像 → 本地一键复现,无需"碰运气"
  - **测试**:录像即 golden case;断言"重放后状态 = 快照状态"
  - **benchmark**:同一录像在不同设备/不同改动下跑,数字可比
- 实现:`Recorder`(挂在 transport + input 出口,记 `(timestamp, event)`)/
  `Player`(替换 transport,按录像驱动 app)。dt 在 replay 模式用录像里的值,不取
  墙钟。随机性(若有)走可注入种子。

---

## 一、可观测性 / wasm 日志与 debug

### 1.1 日志

- **`tracing` + `tracing-wasm`**(或 `log` + `console_log`):结构化日志输出到浏览器
  console,带 level + span(span 天然对应模块/帧)。
- **`console_error_panic_hook`**:把 Rust panic 的 message + backtrace 打到 console,
  否则 wasm panic 只显示无用的 `unreachable`。
- **分级 + feature gate**:`trace!/debug!` 仅 debug build 编入;release 默认只留
  `warn!/error!`,零开销。
- **关键路径埋点**:transport 连接状态、fsm 迁移、smoother 水位、对账触发、
  layout 跨界耗时、atlas 淘汰、降级切换。

### 1.2 浏览器 devtools

- 编 wasm 带 **DWARF debug info**(`debug = true`),Chrome 的 C/C++/Rust DevTools
  扩展可下断点、看变量、源码级单步。
- **Source map** 让 panic backtrace 指回 Rust 源码行。

### 1.3 GPU debug

- wgpu 开 **validation layer**(debug build),捕获非法用法。
- 帧捕获:RenderDoc(原生跑同一渲染代码时)/ WebGPU 的 timestamp-query 量化 GPU 耗时。
- atlas 可视化:把当前 atlas 纹理直接画到屏角(debug overlay)。

### 1.4 实时调试 HUD(egui,开发期)

feature-gated 的 egui overlay(0007 定位:仅调试),实时显示:

- 帧时间(p50/p95)、dropped frames、可见 instance 数
- 每个 Active Part/Turn 的 FSM 状态、smoother 各队列 backlog
- atlas 占用率/淘汰率、跨界调用次数与字节数、当前 render backend 档位
- 录像回放控制(暂停/单帧步进/拖时间轴)

### 1.5 不变量断言(debug build)

把 0003/0005 的不变量写成运行时 `debug_assert!`:
- store 有序性(二分前提)、part_accum 与 part.text 一致性
- effects 不回写核心状态(presentation/model 分离)
- 任意时刻可见 instance 的 y 单调

---

## 二、测试方案(分层,native 优先)

### 2.1 纯逻辑单元测试(native `cargo test`,占大头,最有价值)

这些模块是纯 Rust、无 GPU/无浏览器,在 native target 跑,毫秒级:
`protocol`、`store`、`fsm`、`smoother`、`content`(markdown+标签+补全)。

- **Golden 测试**:事件流输入 → 断言最终 store 状态 / StyledSpan 输出。
- **属性测试(proptest)**——直接编码不变量:
  - *容错*:对事件流做 **乱序 / 重复 / 丢失** 扰动 → 断言最终状态 == 快照状态(0003)
  - *FSM 投影*:任意状态序列 → 不 panic,落到合法终态(0005)
  - *smoother*:固定 dt 下吐字总数守恒、不切碎 grapheme(0002 §4.1)
  - *标签*:标签在任意位置被 delta 切断 → 不出现字面标签字符上屏(0006)
  - *markdown*:任意截断点 → remend 补全后可解析、已完成块不变(0004)
- **收尾判定矩阵**(0005 §5)逐行写成 case:idle 丢失、忘了 idle(有心跳)、
  连接死、快照对账、超时可复活。

### 2.2 wasm 边界测试(`wasm-bindgen-test`,headless 浏览器/node)

只测真正依赖浏览器的部分:
- `layout`:StyledSpan → pretext → 位置/高度,验证句柄管理 + 零拷贝视图(grow 后重建)
- `transport`:EventSource 收发、重连、Last-Event-ID
- `render`:backend 初始化与降级探针(WebGPU→WebGL2→Canvas2D)

### 2.3 视觉/快照测试(渲染到纹理,回读对比)

- **off-profile golden**:effects 关到 off,渲染结果是确定性几何 → 渲染到离屏纹理
  → readback 像素 → 与 golden PNG 对比(带容差)。验证 layout/scene/render 正确性。
- 覆盖:中英混排、emoji/组合字、代码块、表格、嵌入占位、长行换行、RTL(若支持)。
- 在 headless WebGPU(或原生 wgpu)跑,纳入 CI。

### 2.4 集成 / 端到端(录像重放)

- 收集一批**真实 opencode 会话录像**(各种工具调用、reasoning、报错、压缩、
  多 turn),全管线重放:
  - 断言:无 panic、最终状态正确、关键帧截图 diff 通过
  - 断言:turn 边界、收尾时机、折叠状态符合预期

### 2.5 故障注入(端到端)

在录像重放里注入:断流重连、丢/重/乱序事件、忘了 idle、标签跨 delta、超长代码块、
embed 加载失败、wasm memory grow、降级到 Canvas2D。断言恢复到正确状态且不崩。

### 2.6 高风险用子 agent 验证

对容错与收尾这类高风险逻辑,除单测外用独立 agent 做对抗式审查(边界用例、
竞态)——架构里 effects 不回写,结构上保证核心测试不被表现层破坏。

---

## 三、Benchmark 方案

### 3.1 要量化的指标(渲染侧;server 侧 TTFT 不归我们)

| 指标 | 说明 | 预算(初值,待实测) |
|---|---|---|
| 帧时间 p50/p95/p99 | 每帧 CPU+GPU 总耗时 | p95 < 8ms(留 120fps 余量) |
| 流式稳态 fps | 持续吐字时 | ≥ 60(高端 ≥120) |
| 滚动 fps | 快速甩动 | ≥ 60 |
| time-to-first-glyph | 首字上屏延迟 | < 一帧 |
| **长对话 fps 曲线** | fps vs 消息数 | **近似平坦(核心卖点)** |
| **内存 vs 消息数** | 常驻内存增长 | **有界(视口裁剪后)** |
| 跨界开销/帧 | wasm↔JS 调用次数+字节 | 与新增字符数成正比,与总长无关 |
| atlas 占用/淘汰率 | 字形图集压力 | 稳态淘汰率低 |
| GPU 时间/帧 | timestamp-query | < 4ms |

### 3.2 微基准(criterion,native)

纯逻辑热点,native 跑,回归门禁:
`markdown parse / 块`、`layout 调用`、`smoother update`、`store apply`、`标签扫描`、
`高亮 / 块`。criterion 出 p50 + 方差,CI 对比基线,超阈值 fail PR。

### 3.3 宏基准(in-browser,合成负载)

需要一个 **合成事件流发生器**(可控字符速率、消息数、工具调用比例、代码块比例):

- **流式稳态**:固定 N 条历史 + 持续吐字 X 字/秒,测帧时间分布
- **滚动压测**:长文档程序化甩动,测滚动 fps
- **长对话扩展**:消息数 100 → 1k → 10k → 100k,画 fps/内存曲线
- **冷启动**:首屏渲染延迟

### 3.4 头条 benchmark(验证项目立项假设)

调研结论:长对话卡顿是业界(含 Claude Code 官方 issue)最大未解痛点。**必须用数据
证明我们的差异化**:

- 同一合成负载下,**我们的 fps/内存 vs 消息数 = 平坦线**;
- 并行测一个 DOM 基线(react-markdown 或 Streamdown 的对话)同负载曲线,
  展示其随消息数劣化;
- 两条曲线的分叉点 = 项目价值的量化证据。

### 3.5 设备/后端矩阵

每个宏基准在三档跑,验证降级优雅(0003 §5):
- 高端(WebGPU 硬件)/ 中端(WebGL2 硬件)/ 低端(软件光栅化 或 Canvas2D 后端)
- 移动端(Tauri WKWebView / 移动浏览器)单列一档

### 3.6 回归门禁

- criterion 微基准 + 宏基准帧时间预算进 CI,基线存档,超预算阻断合并
- 录像重放的截图 diff 作为视觉回归门禁

---

## 四、CI 编排

```
PR:
  cargo test(native 单元 + 属性测试,秒级)        ← 必过
  cargo bench --no-run(编译保证)
  wasm-bindgen-test(headless,边界 + 降级)         ← 必过
  视觉快照 diff(off-profile golden)               ← 必过
  criterion 微基准 vs 基线                          ← 超阈值告警/阻断
nightly:
  宏基准全设备矩阵 + 长对话曲线 + DOM 基线对比      ← 出趋势报告
  录像重放回归(真实会话集)+ 故障注入套件
```

---

## 五、实现顺序(配合 architecture.md §7 竖切)

1. **先搭 record/replay + console 日志 + panic hook**——竖切最小链路时就要有,
   否则 debug 全靠瞎猜
2. 最小链路打通后:加 native 单元 + proptest(store/fsm/smoother)
3. 加 off-profile 视觉快照(layout/render 一正确就锁住)
4. 加合成负载发生器 + 长对话曲线 benchmark(尽早量化核心卖点,指导优化方向)
5. 接 content 全量后:markdown/标签/容错的属性测试 + 故障注入
6. 全设备矩阵 + DOM 基线对比(里程碑级,对外证明用)

---

## 六、关键工具清单

| 用途 | 工具 |
|---|---|
| 日志 | tracing + tracing-wasm / log + console_log |
| panic | console_error_panic_hook |
| wasm 单测 | wasm-bindgen-test(headless Chrome/FF/node) |
| native 单测 | cargo test |
| 属性测试 | proptest |
| 微基准 | criterion |
| GPU 校验/捕获 | wgpu validation layer、RenderDoc、WebGPU timestamp-query |
| 调试 HUD | egui(feature-gated,0007) |
| wasm 源码调试 | DWARF + Chrome Rust DevTools 扩展 |
| 视觉回归 | 离屏渲染 readback + PNG diff |
