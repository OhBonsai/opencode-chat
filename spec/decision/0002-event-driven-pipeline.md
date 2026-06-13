# 决策记录 0002:事件驱动管线、状态机与可见区域渲染

- 日期:2026-06-13
- 状态:已采纳(原型验证前)
- 前置:0001(整体架构)
- 范围:渲染循环分相、Part 状态机、双时钟模型、平滑器、动画系统、视口裁剪

## 1. 核心思想

采用游戏客户端的标准架构:**事件不直接画画面,事件改状态,渲染只读状态**。
SSE 事件是离散、突发的;画面是连续、按帧的。中间隔一层世界状态(文档模型),
这是解决 token 突发导致画面抽搐的本质手段。

对应关系:

| 游戏 | chat 画布 |
|---|---|
| 网络包 | SSE 事件 |
| 世界状态 | 文档模型(session/message/part 三表) |
| 实体 + FSM | Part + Part 状态机 |
| 网络插值缓冲 | 流式平滑器 |
| 动画系统(tween 池 + shader) | 同款 |
| 渲染器 | wgpu instanced draw |

本质上是一个"回合制 MMO 客户端",实体是文字。

## 2. 每帧管线(严格分相)

```
SSE 事件(EventSource, Rust 侧)
   ↓ 入队(不立即应用)
─── 每帧(rAF)──────────────────────────
1. drain 事件队列 → 文档状态机迁移        ← 离散时钟
2. 平滑器 update(dt) → 决定本帧上屏字符   ← 连续时钟
3. 排版(只对脏的尾部块,一次跨界批调 pretext)
4. 动画系统 update(t) → 写 instance 属性
5. render:instanced draw                  ← 纯读,不改任何状态
```

规则:

- **双时钟**:离散时钟(事件)驱动状态机**迁移**;连续时钟(time)驱动动画**插值**。
  迁移瞬间完成,表现连续展开。
- **动画挂在迁移上**:FSM 的 enter/exit 钩子负责 spawn 动画(tween/timeline),
  动画系统按时间推进,渲染读插值结果。状态机不关心动画进行到哪。
- update/render 频率解耦:无物理,update 跟 rAF 变步长即可;平滑器内部用积分器模式。

## 3. Part 状态机

每个 Part 是一个实体,带自己的 FSM。SSE 协议已经天然携带状态机结构。

### 3.1 TextPart / ReasoningPart

```
Born       part.updated(空文本) 到达    enter: 创建渲染对象
Streaming  part.delta 流入             字符进平滑器
Settling   part.updated(全量+time.end)  对账;enter: 收尾动画(光标淡出等)
Settled    布局/instance 冻结进 GPU buffer
```

### 3.2 ToolPart(协议原生状态机)

`state.status: pending → running → completed | error`

迁移即表现:pending→running 转菊花;running→completed 收起 + 打勾动画;
running→error 红色展开。

### 3.3 Session

`idle ⇄ busy ⇄ retry`(`session.status` 事件)——驱动"思考中"指示器、输入框禁用、
平滑器的"完结冲刺"(busy→idle 时把缓冲区剩余字符加速放完)。

### 3.4 事件 → 状态机路由表

| SSE 事件 | 目标 | 动作 |
|---|---|---|
| `message.updated` | Message 表 | 建/更新消息壳(role、model、cost、error) |
| `message.part.updated` | Part FSM | 出生(Born)/ 对账(→Settling)/ 工具状态迁移 |
| `message.part.delta` | Part FSM(Streaming) | 文本进平滑器(唯一走平滑器的事件) |
| `message.part.removed` / `message.removed` | 表 | 删实体 + 退场动画 |
| `session.status` | Session FSM | idle/busy/retry 迁移 |
| `session.updated` | Session 表 | 标题、revert 等元数据 |
| `permission.asked` / `question.asked` | 交互层 | 弹 UI(转发 React) |
| `step-start`/`step-finish`/`patch`/`snapshot` part | — | 不渲染(噪音过滤,同 opencode 桌面端) |
| `server.heartbeat` | 连接监控 | 活性检测 |

注:只有字符串字段走 delta(text/reasoning 正文、工具 raw 参数),其余全是全量
part 反复 updated。平滑器只服务 delta 通道。

## 4. 平滑器(发射器/积分器)

网络游戏平滑远端玩家位置的同一招:追赶式插值。

```rust
fn update(&mut self, dt: f32) {
    // 缓冲区水位决定速率:水位高加速,低减速
    let target_rate = base_rate * (1.0 + self.backlog() as f32 * k);
    self.accumulator += target_rate * dt;
    while self.accumulator >= 1.0 {
        self.accumulator -= 1.0;
        self.reveal_next_char(now);   // 打 spawn_time,生成 glyph instance
    }
}
```

- 按 partID 各一个 reveal 队列
- session busy→idle 时冲刺放完剩余缓冲
- 上屏的字符打 `spawn_time` 进 instance 属性

### 4.1 吐字单位 = grapheme cluster(正确性,非可选)

`reveal_next_char` 的"一个 char"必须是 **grapheme cluster,不是 Unicode 码点**。
按码点切会把 emoji、组合字符、ZWJ 序列(如 👨‍👩‍👧、带肤色修饰的 emoji、声调
组合字)切碎成乱码或半个字符闪现。复用 pretext 已做的 grapheme 分段(它本就按
`Intl.Segmenter` 切),平滑器从分段结果里逐 cluster 取,绝不在 cluster 内部断开。
CJK 按字自然成立,但这条规则是为了 emoji/组合序列的正确性,不是可选优化。

### 4.2 参数基线(业界参照,初值)

公式里的常量需实测调,但给一组有出处的起点(见 [industry 调研](../research/industry-llm-chat-rendering.md)):

- `base_rate`:目标稳态吐字速率 ≈ **200 字/秒(5ms/字)**——Upstash/业界常用值,
  readable 又不拖沓
- 批处理/帧合并:业界 DOM 方案用 ~50ms 批(为少渲染);我们每帧渲染,无需此妥协,
  但平滑器内部仍按 dt 积分,效果等价于"每帧匀速"
- `k`(水位加速系数)、soft/hard 超时(0005:8s/30s)、心跳超时(0003:~25s):
  同为初值,依实测调

## 5. 动画系统

两层:

1. **tween 池**:`{target, property, curve, t0, duration}`,状态机迁移时 spawn,
   每帧统一推进,完成自动回收。用于块级动画(工具卡片收起、消息退场)。
2. **shader 动画**(零 CPU 成本):逐字符效果不进 tween 池——`spawn_time` 直接进
   instance 属性,WGSL 里 `uniform time - spawn_time` 算淡入/上浮/溶解。
   游戏里"把动画下放到 vertex shader"的标准优化。

### 5.1 效果开关的代码设计

原则:**效果是数据,不是分支;效果是单向消费者,不是参与者**。

1. **模型/表现分离,依赖单向**。仿真层(文档、FSM、平滑器、排版)不知道效果的
   存在,只产出事实(`spawn_time`、FSM transition 记录)。效果层订阅事实做表现。
   依赖箭头永远是 `效果层 → 核心层`,反向零条:效果完成回调不触发任何逻辑,
   逻辑只由 FSM 和时间驱动。

   ```rust
   // 核心层输出,与效果无关,永远生成
   struct GlyphInstance { pos, uv, spawn_time, style_id }
   // 效果层:style_id → 参数表(uniform buffer)
   struct EffectParams { fade_dur, rise_px, dissolve, ... }
   ```

2. **关闭 = 参数置零,不是代码路径切换**。shader 写成"基础值 + 调制":
   `pos = base_pos + effect_offset(params, age)`。效果档位(profile)分
   full / reduced / off 三档,off 即全零参数表——同一条代码路径,行为退化为
   恒等,开关组合不会产生 2^n 条路径。GPU 能力检测(0001 降级阶梯)、用户设置、
   省电模式都只是 profile 选择器的输入。性能需要时再用 specialization constant
   编译掉零分支(优化,非架构)。

3. **恒等收敛不变量(最关键)**:任何效果在 `age >= duration` 后必须严格等于
   恒等变换。效果只调制瞬态外观(alpha/offset/扭曲),不影响最终停留位置。
   推论:hit-test、选区、滚动、布局永远基于 settled 几何,不读动画中的位置;
   开关任意效果,最终视觉几何逐像素一致;中途切 profile 最坏跳变一帧,逻辑零影响。

4. **tween 池只写表现字段**。实体字段分 model(FSM 状态、settled 高度)与
   presentation(当前显示高度、alpha)两组;tween 只写 presentation,布局和逻辑
   只读 model。"收起动画影响后续块位置"的情形:目标高度属于 model(迁移瞬间
   定死),当前高度属于 presentation,滚动锚定按目标高度算。

测试推论:核心层用 off profile 跑 golden test(事件流输入 → 断言最终几何);
效果层不回写,结构上不可能破坏核心测试。

## 6. 可见区域渲染(视口裁剪)

opencode 桌面端用 virtua(DOM 虚拟列表)做可见区域渲染。**DOM windowing 这个技术
本身不适用**(我们没有 DOM 节点),但它解决的三个问题依然存在,用 GPU 渲染的
对应手段解决:

| 问题 | DOM 虚拟列表的解法 | 我们的解法 |
|---|---|---|
| 渲染成本无界 | 只挂载可见行的 DOM 节点 | **视口裁剪**:instance 按 y 排序(布局 append-only 天然有序),二分查出可见区间,只 draw 该范围;长会话百万 glyph 也只画几千 |
| 内存无界 | 卸载不可见节点 | **GPU buffer 驱逐**:远离视口的块只保留"块高度 + 文本",glyph instance 释放;滚回来时重排版重建(pretext layout() 是纯算术,重建便宜)。atlas 的 LRU 同理,屏外字形自然淘汰 |
| 滚动条稳定/跳转 | 行高缓存 + 估算 | **块高度缓存** keyed by (block, width):不物化内容也能算总高度、滚动条位置、跳转目标。宽度变化时全量失效,但 layout() 重算很快 |

**我们的先天优势**:DOM 虚拟列表的高度是**异步测量**的(挂载后才知道),
所以 opencode 需要 90 帧连续锚底 hack(virtua #301)对付测量抖动。我们的布局是
**同步且精确**的——渲染前就知道每块的确切高度,滚动锚定无抖动,不需要任何
补偿 hack。这是自绘画布相对 DOM 的结构性优势。

仍要照搬的:

- **块级粒度**:裁剪、驱逐、高度缓存都以 markdown 块为单位(与 0001 的缓存边界一致)
- **overscan**:可见区间上下各多保留一屏,滚动时无空白
- **历史分页**:滚到顶部拉 200 条(REST cursor),预排版后插入,滚动位置按新增高度
  整体偏移(我们高度精确,一次到位)
- **锚底策略**:仅当用户在底部才跟随;wheel/touch 手势区分用户滚动与自动滚动。
  "多接近底部算在底部"需定一个阈值——业界跨度很大:opencode 桌面端用 **4px**(严格,
  几乎贴底才跟随),TanStack Virtual 用 **80px**(宽松,留一点余量更跟手)。
  初值取 **~48px**(约一行高),后续按手感调。配合 `followOnAppend` 语义:已在
  阈值内→新内容跟随;用户上滑出阈值→新内容落在下方不抢焦点。
  ([参照 TanStack](../research/industry-llm-chat-rendering.md))

## 7. 状态所有权小结

- 文档模型、Part FSM、平滑器、tween 池、滚动状态:全在 wasm
- prepare() 缓存(排版句柄):JS 侧,wasm 持 u32 句柄(0001 §3.4)
- glyph instance / atlas:GPU,wasm 持元数据(区间索引、高度缓存)
- React:只读摘要(session 列表、状态指示、permission/question 弹窗),经粗粒度回调
