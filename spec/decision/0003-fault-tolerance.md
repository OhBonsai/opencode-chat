# 决策记录 0003:容错、状态同步与降级渲染

- 日期:2026-06-13
- 状态:已采纳(原型验证前)
- 前置:0001(整体架构)、0002(事件驱动管线 + Part 状态机)
- 范围:Part FSM 容错、丢/重/乱序事件处理、重连修复、心跳看门狗、渲染后端降级

## 1. 核心不变量

**任何事件序列(丢失、重复、乱序)下,文档最终状态 = 快照状态;
动画只是路径,不是结果。**

这与 0002 §5.1 效果层的"恒等收敛"是同一哲学:表现层可降级/可跳变,
核心状态必须收敛到唯一正确值。SSE 是 at-least-once 且可能缺失,信封无序号
(`id` 是事件 id 不是序列号),所以纠错手段是全量对账(`part.updated`)+
重连重拉快照,而非可靠传输。

## 2. opencode 桌面端的容错模式(已确认,可借鉴)

来源:`packages/app/src/context/global-sync/event-reducer.ts`、`server-sync.tsx`

1. **全量事件 upsert,增量事件 drop**。`part.updated`/`message.updated` 遇到
   不存在的实体就地创建(找到 reconcile 覆盖,没找到二分插入);`part.delta`
   遇到 message/part 不存在直接 `break` **静默丢弃**——后续必有 `part.updated`
   带全量修复。
2. **一切幂等**:全是 upsert + reconcile,事件重放无害;ID 单调可排序,乱序到达
   靠二分插入归位,不依赖到达顺序。
3. **重连 = 整体重拉**:SSE 自动重连后再收到 `server.connected` → 所有目录进
   刷新队列,重新 bootstrap(快照覆盖本地)。启动后 1.5s 内的 `server.connected`
   跳过,避免双重加载。
4. **删除连带清理派生状态**:`message.removed` 把所属 part 的 delta 累积器一起删。
5. **错误是内容不是异常**:RetryPart、ToolStateError、message.error 正常渲染。

## 3. 我们的做法(在上述之上,为 FSM + 动画补三层)

### 3.1 FSM 用"投影"语义,不用严格迁移表

事件 at-least-once 且可能缺失,工具 part 可能从 pending 直接跳 completed
(没见过 running)。因此:

- 不做"当前状态 + 事件 → 拒绝非法迁移"
- 而做"事件载荷 → 目标状态,迁移 = 当前与目标的 diff"
- 任意状态到任意状态都合法;跳过的中间态动画快进合并或直接省略

### 3.2 每个 FSM 两种应用模式:live / catch-up

同一份 FSM 代码,模式作为参数传入:

- **live**:正常迁移 + spawn 动画 + 走平滑器
- **catch-up**:直接 settle 到目标状态,**零动画零平滑**

用于快照回放、重连修复、历史分页加载。游戏里 late-join 状态同步的标准做法——
新进玩家不重放之前的战斗。

### 3.3 孤儿 delta 缓冲(比 opencode 多半步)

`part.delta` 的 part 还没 Born 时,不直接丢:

- 按 partID 建有上限缓冲(如 64KB)
- `part.updated`(Born)到达时回放进平滑器
- 超上限才丢,等全量对账

理由:opencode 丢了无妨(updated 一到 Solid 重渲染);我们丢了会损失该段文本的
流式动画,缓冲半步保住体验。

### 3.4 重连做增量修复,不做整体重置

opencode 重拉后全量覆盖,DOM diff 兜底;我们整体重建会画面闪烁。做法:

- 重连后拉快照,与本地文档**逐 part 比对**
- 文本相同的 settled 块:GPU instance 完全不动
- 仅有差异的 part 走 catch-up 修复

### 3.5 心跳看门狗

协议每 10s 一个 `server.heartbeat`。超过 ~25s 无任何事件 → 主动断开重连。
EventSource 自带重连不能覆盖"连接僵死"(TCP 未断但无数据)的情况。

### 3.6 向前兼容

未知事件 / 未知 part 类型 → serde `#[serde(other)] Ignored`。服务端加新类型不崩。

## 4. 启动 / 重连时序

```
开 SSE(事件入缓冲,不应用)
  → fetch 快照(catch-up 模式建文档,无动画)
  → 回放缓冲(晚于快照的事件 live 应用)
  → 标记 live,正常运行
重连:
  → SSE 自动重连 → server.connected
  → 拉快照 → 与本地逐 part diff → 仅差异 part catch-up 修复(无整体重置)
```

## 5. 降级渲染(无 GPU / 弱机器)

浏览器无"完全无 GPU"——自带软件光栅化兜底。降级阶梯四层:

```
1. WebGPU(硬件)          完整体验
2. WebGL2(硬件)          完整体验,wgpu 自动改写 GLSL
3. WebGL2(软件光栅化)     自动发生,对代码透明(SwiftShader/llvmpipe/WARP)
4. Canvas2D 自绘后端       我们的最后一层
```

第 3 层大概率够用:负载极轻(带 alpha 的纹理 quad、可见区几千实例、无 3D 无
overdraw),SwiftShader 在普通 CPU 上可维持可用帧率。

### 5.1 后端 trait 边界

文档模型、FSM、平滑器、pretext 排版、块高度缓存全部与渲染器无关。需替换的只有
"instance 数组 → 像素"一段:

```rust
trait RenderBackend {
    fn begin_frame(&mut self);
    fn draw_glyphs(&mut self, instances: &[GlyphInstance], time: f32);
}
```

Canvas2D 后端可整个放 JS 侧,消费同一份平铺 instance 数组逐字 `fillText`。
效果降级:逐字符 shader → 简单 alpha 淡入;溶解/上浮等放弃。

**关键**:丝滑感大头来自平滑器和布局稳定性,不来自 shader。速率平滑、append-only
不跳动、精确滚动锚定在 Canvas2D 后端一分不少。降级丢的是"华丽",不是"舒适"。

### 5.2 检测与切换(与 0002 §5.1 的 profile 机制统一)

1. `requestAdapter()` 失败 → WebGL2;WebGL2 拿不到 → Canvas2D
2. `WEBGL_debug_renderer_info` 含 SwiftShader/llvmpipe/WARP → 软件渲染,
   不立即降级,跑性能探针:前 60 帧测帧时间,持续超预算才切 Canvas2D
3. 切换对上层无感(边界是 RenderBackend trait);能力检测、用户偏好、省电模式
   最终都汇到 0002 的 profile 选择器(选 full/reduced/off 效果档)

落地:先做 trait 边界(零成本),Canvas2D 后端等探针数据证明有需求再写。
