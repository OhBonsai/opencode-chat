# 技术方案:整体链路与模块设计

- 日期:2026-06-13
- 定位:把 0001–0008 的决策落成一份可实现的模块分解方案
- 范围:LLM SSE 对话渲染 wasm 组件的内部架构

---

## 一、整体链路(端到端)

```
                          ┌─────────────── 宿主(React/Vue)───────────────┐
                          │  <canvas>        粗粒度回调        DOM overlay   │
                          └────┬───────────────▲──────────────────▲────────┘
                               │ 事件/配置      │ 摘要              │ embed 盒子
  opencode server              ▼               │                  │
   │ SSE /api/event   ┌────────────────────────┴──────────────────┴────────┐
   │ REST 快照        │                  api(接口层)                       │
   ▼                  ├──────────────────────────────────────────────────────┤
┌──────────┐          │  app(每帧编排循环)                                  │
│transport │──events─▶│                                                       │
│(SSE+重连) │          │  ┌─protocol─┐  ┌─store─┐  ┌─fsm─┐  ┌─smoother─┐      │
└──────────┘          │  │ 解码事件  │─▶│文档三表│─▶│状态机│─▶│ 节奏整流  │     │
   ▲ 快照/心跳         │  └──────────┘  └───────┘  └─────┘  └──────────┘      │
   │                  │        │脏尾块                          │上屏字符      │
   │                  │        ▼                                ▼             │
   │                  │  ┌─content─┐   ┌─layout──┐   ┌─scene────┐  ┌─effects─┐│
   │                  │  │md+标签+ │──▶│pretext  │──▶│glyph/atlas│─▶│tween+   ││
   │                  │  │高亮     │   │(JS边界) │   │embed/裁剪 │  │shader   ││
   │                  │  └─────────┘   └─────────┘   └──────────┘  └─────────┘│
   │                  │                                    │                  │
   │                  │  ┌─input──┐                        ▼                  │
   │                  │  │滚动/选区│◀─────────────▶ ┌─render(wgpu)─┐          │
   │                  │  │/hit-test│                │ backend+相机  │──pixels─▶ canvas
   │                  │  └────────┘                 └──────────────┘          │
   └──────────────────┴───────────────────────────────────────────────────────┘
```

一句话:**transport 收流 → protocol 解码 → store 落状态 → fsm 迁移 → smoother 整流
→ content 解析尾块 → layout 排版 → scene 组织 → effects 调制 → render 出像素**,
由 app 每帧驱动,api 对宿主收口。

---

## 二、模块清单

| 模块 | 职责 | 归属 | 主要决策 |
|---|---|---|---|
| `transport` | SSE 接入、重连、心跳看门狗、快照拉取 | wasm | 0003, 0008 |
| `protocol` | 事件信封 + Part 类型 serde 解码 | wasm | 0001 |
| `store` | 归一化文档三表 + 增量/全量对账 | wasm | 0002, 0003 |
| `fsm` | Part/Turn/Tag 状态机、收尾判定、投影 | wasm | 0002, 0005, 0006 |
| `smoother` | 流式节奏整流、逐 grapheme 吐字 | wasm | 0002 |
| `content` | 标签扫描 + markdown 解析 + 补全 + 高亮 | wasm | 0004, 0006 |
| `layout` | pretext 排版桥(wasm↔JS 句柄/零拷贝) | JS+wasm | 0001, 0004 |
| `scene` | glyph atlas、instance buffer、embed、视口裁剪、块缓存 | wasm+GPU | 0001, 0004, 0007 |
| `effects` | tween 池 + WGSL shader 效果 + profile | wasm+GPU | 0002 |
| `render` | wgpu 管线、RenderBackend trait、像素对齐相机 | wasm+GPU | 0001, 0003, 0007 |
| `input` | 指针/键盘、滚动状态、hit-test、选区、剪贴板 | wasm | 0002 |
| `api` | 对宿主的命令式 API、粗粒度回调、embed 盒子上报、无障碍 DOM 镜像 | wasm+JS | 0000, 0007 |
| `app` | 每帧编排循环,串起所有模块 | wasm | 0002 |

---

## 三、各模块设计

### 1. `transport`(传输层)

- **职责**:用 Rust 侧 EventSource(gloo-net)直收 `/api/event`;断线重连;
  心跳看门狗(>25s 无事件主动重连);REST 拉快照。
- **输入**:serverUrl、sessionId、(可选)鉴权回调。
- **输出**:`RawEvent`(未解码的 `{id, type, properties}` 文本/值)推进事件队列。
- **关键设计**:
  - 启动时序:先开 SSE 入缓冲 → 拉快照 catch-up → 回放缓冲 → 转 live(0003 §4)
  - 心跳区分两种沉默:有心跳无内容=模型停了(交 fsm 判收尾);无心跳=连接死(重连)
  - 连接策略抽象 `ConnectionStrategy`(direct / leader-elected),默认 direct,
    为 0008 多标签选主预留
- **不做**:解析业务语义(交 protocol)。

### 2. `protocol`(协议解码)

- **职责**:把 RawEvent 解码成强类型 Rust enum。
- **关键类型**:
  ```rust
  #[serde(tag="type", content="properties")]
  enum Event { PartDelta{..}, PartUpdated{part:Part}, MessageUpdated{..},
               SessionStatus{..}, Heartbeat, Connected, #[serde(other)] Ignored }
  #[serde(tag="type")]
  enum Part { Text(..), Reasoning(..), Tool(..), StepStart, StepFinish, ... }
  ```
- **关键设计**:未知类型 → `Ignored`(向前兼容,0003 §3.6);
  字符串只过界一次,在此完成 UTF-8 解码。
- **依赖**:opencode 协议(0001 §3.1)。

### 3. `store`(文档模型)

- **职责**:世界状态的唯一真相。归一化三表 + 对账。
- **关键类型**:
  ```rust
  struct Store {
      session: Vec<Session>,                  // 二分有序
      message: HashMap<SessionId, Vec<Message>>,
      part: HashMap<MessageId, Vec<Part>>,
      part_accum: HashMap<PartId, String>,    // delta 累积,供对账
  }
  ```
- **关键设计**:
  - delta 乐观追加 + part.updated 全量对账清零(0003 §1,opencode 同款)
  - 一切 upsert + 二分插入,幂等;乱序靠 ID 排序归位
  - 投影语义:Turn 等聚合不存储,从三表实时算(0005)
- **不做**:渲染、动画(纯数据)。

### 4. `fsm`(状态机)

- **职责**:Part FSM、Turn FSM(聚合+收尾)、Tag 区域 FSM。
- **关键设计**:
  - **投影迁移**:任意态→任意态合法,迁移=当前与目标 diff(0003 §3.1)
  - **live / catch-up 双模式**:catch-up 直接 settle 无动画(快照/重连/分页)
  - **Turn 收尾**:多信号收敛 + 看门狗(8s→Stalled,30s→settle 可复活)+ 快照对账(0005)
  - **Tag 区域**:缺失闭合在块边界/turn 收尾隐式闭合(0006 §5)
  - 迁移的 enter/exit 钩子负责 spawn 动画(交 effects),自身不管动画进度
- **输出**:迁移驱动 smoother(delta 入队)、effects(spawn tween)、向宿主报状态(api)。

### 5. `smoother`(平滑器)

- **职责**:把突发的 delta 整流成匀速上屏。
- **关键设计**:
  - 按 partID 各一 reveal 队列;积分器 `rate=base*(1+backlog*k)`(0002 §4)
  - **吐字单位 = grapheme cluster**(复用 pretext 分段,不切碎 emoji/组合字,0002 §4.1)
  - 基线:~200 字/秒;busy→idle 冲刺放完(0002 §4.2)
  - 上屏字符打 `spawn_time`,交 scene 生成 instance
- **输出**:本帧应上屏的字符区间。

### 6. `content`(内容管线)

- **职责**:原始文本 → 语义化样式 run。
- **流水**:
  ```
  part.text(尾块)→ segmenter(标签 hold 区,0006 §3)
    → [markdown 段] → remend 补全(0004 §5.1)→ jcode parse → StyledSpan
    → [标签区域] → 注册表 resolve → 语义区域/chip/hidden(0006 §4)
    → 高亮(syntect/fancy-regex,按 hash+lang 缓存,0004 §6)
  ```
- **关键设计**:只处理脏的尾部块,已完成块冻结(0004 §5);输出语义角色非像素。
- **输出**:StyledSpan 序列 + embed 占位标记。

### 7. `layout`(排版桥)

- **职责**:StyledSpan → 带位置的 run。唯一布局器。
- **关键设计**(wasm↔JS 边界,0001 §3.4):
  - StyledSpan → pretext rich-inline fragment(role+attrs→font)
  - `prepare()` 结果留 JS,wasm 持 u32 句柄;请求只传句柄+宽度
  - 结果走平铺 Float32Array 零拷贝回 wasm;每帧一次批调用
  - 块高度精确同步返回(喂 scene 的高度缓存)
- **输出**:run 的位置 + 块高度。

### 8. `scene`(场景管理)

- **职责**:把 run/embed 组织成可渲染的 GPU 资源,管理可见性与内存。
- **子模块**:
  - **glyph atlas**:预分配多页纹理 + LRU + CJK 分桶 + emoji RGBA 页(0004 §7.4)
  - **instance buffer**:run → GlyphInstance(pos/uv/spawn_time/style_id)
  - **embed**:图片/SVG/mermaid 纹理块、自绘卡片、DOM overlay 盒子(0007)
  - **视口裁剪 + 块缓存**:instance 按 y 有序,二分可见区间;屏外块释放 instance
    只留高度+文本;块高度缓存 keyed by (block,width)(0002 §6)
- **输出**:本帧可见 instance 区间 + embed quad。

### 9. `effects`(动画/效果)

- **职责**:动画系统,效果开关。
- **关键设计**:
  - **tween 池**:块级动画(卡片收起、退场),只写 presentation 字段(0002 §5)
  - **shader 效果**:逐字符靠 spawn_time 在 WGSL 算,零 CPU(0002 §5)
  - **profile**:full/reduced/off,关闭=参数置零非分支;恒等收敛不变量(0002 §5.1)
  - GPU 降级、用户设置、省电模式统一汇入 profile 选择器
- **输出**:写 instance 的效果参数 / uniform。

### 10. `render`(渲染后端)

- **职责**:instance + embed → 像素。
- **关键设计**:
  - `RenderBackend` trait:WebGPU / WebGL2 / Canvas2D 三档,探针驱动降级(0003 §5)
  - **像素对齐相机**:world unit = CSS pixel,统一正文/卡片/overlay/无障碍镜像坐标
    系,消除 DOM overlay 同步缝(0007 §3)
  - 两个 pass:chat 内容 pass + (将来)overlay,按 z-order 合成
- **输出**:画进宿主 canvas。

### 11. `input`(交互)

- **职责**:输入处理 + 选区 + 滚动。
- **关键设计**:
  - pointer 事件 JS 侧收集,每帧批量喂 wasm(0001)
  - 滚动 = `scroll_offset` 状态;滚动条是自绘 GPU 元素(两个矩形),非 DOM 非 egui;
    锚底阈值 ~48px + 仅底部跟随 + 手势区分(0002 §6)
  - **选区/复制**:GPU 无原生选择,自建 hit-test(基于 settled 几何)+ 选区渲染
    + clipboard 写入(0002 待办)
- **输出**:改 scroll_offset、selection(写 store),命令交 app。

### 12. `api`(接口层)

- **职责**:对宿主收口。
- **关键设计**:
  - 命令式入口:`new ChatCanvas(el, {serverUrl, sessionId})`、`scrollTo`、`setProfile`
  - 粗粒度回调:`connectionState`、`turnSettled`、`permissionRequest`、`questionRequest`、
    `selectionChanged`(低频小数据,0000 §2.2)
  - **embed overlay 盒子上报**:每帧把交互卡片的位置+尺寸交宿主渲染 DOM overlay(0007)
  - **无障碍 DOM 镜像**:隐藏 DOM 文本树,供屏幕阅读器/Cmd+F(0002 待办,Figma/
    Google Docs 同款)
- **不做**:业务 UI(那是宿主的事)。

### 13. `app`(编排循环)

- **职责**:每帧把所有模块按序驱动(0002 管线)。
  ```
  每帧:
    transport.drain() → protocol.decode() → store.apply() → fsm.step()
    smoother.update(dt) → content.parse(脏尾块) → layout.run()
    scene.rebuild(可见) → effects.update(t) → render.frame()
    input.flush() → api.emit(摘要)
  ```
- **关键设计**:严格分相(事件改状态 / 渲染只读状态);离散时钟驱动 fsm,
  连续时钟驱动 effects(0002 §2)。

---

## 四、模块依赖(单向,自上而下)

```
api ──▶ app ──▶ {transport, protocol, store, fsm, smoother, content, layout, scene, effects, render, input}
                   │         │        │     │       │         │        │       │      │        │       │
transport ▶ protocol ▶ store ▶ fsm ▶ smoother ▶ content ▶ layout ▶ scene ▶ effects ▶ render
                                 └────────────────────────────────────────▶ input ▶ store(回写选区/滚动)
```

规则:**数据流单向(事件→状态→渲染)**;effects 只读核心状态、不回写(0002 §5.1);
input 是唯一的反向写(选区/滚动→store),但只写、不触发渲染逻辑。

---

## 五、关键跨模块契约(接口冻结点)

这些接口一旦定下,模块就可并行开发:

1. **Event / Part**(protocol→store):opencode 协议的 Rust 类型
2. **Store 快照 + 变更**(store→fsm→其余):三表 + 对账语义
3. **StyledSpan**(content→layout):语义角色 + attrs(jcode 风格)
4. **LayoutResult**(layout→scene):run 位置 + 块高度(平铺 Float32Array)
5. **GlyphInstance**(scene→render):pos/uv/spawn_time/style_id
6. **RenderBackend trait**(render):begin_frame / draw_glyphs / draw_embed
7. **EffectProfile**(effects):style_id → 参数表;full/reduced/off
8. **HostApi**(api↔宿主):构造参数 + 回调集 + embed 盒子流

**排版模块(layout)刻意做成可替换**:接口是"StyledSpan in → 带位置 run out",
将来 pretext↔cosmic-text 切换,只动这一模块(0001 §2.2)。

---

## 六、一帧的真实走查(流式中)

1. `transport` drain 出 3 个 `message.part.delta` + 1 个 `session.status:busy`
2. `protocol` 解码;`store` 把 delta 追加到对应 part.text + part_accum
3. `fsm`:Text part 处 Streaming;Turn 处 Active;刷新看门狗活动时间
4. `smoother` update(dt):缓冲区水位偏高,本帧放 7 个 grapheme,各打 spawn_time
5. `content`:只对正在长的尾部块重解析(hold 区检查无半截标签 → jcode parse →
   StyledSpan → 增量高亮)
6. `layout`:新增 run 喂 pretext(句柄+宽度),拿回位置 + 块高度(零拷贝)
7. `scene`:生成 7 个新 GlyphInstance;视口裁剪算出可见区间;atlas 缺字形则光栅化入页
8. `effects`:tween 池推进;7 个新字 spawn_time 进 instance,profile=full
9. `render`:wgpu 一次 instanced draw 可见区间;相机 world=px
10. `input` flush:无;`api` emit:无状态变化(turn 仍 Active),不回调

稳态下每帧跨界数据量只与"新增字符数"成正比,与文档总长无关(0001 §3.4)。

---

## 七、实现顺序建议(降低风险)

1. **竖切最小链路**:transport(连真实 opencode serve)→ protocol → store →
   smoother → content(纯文本,先不接 markdown)→ layout(pretext)→ scene →
   render(WebGPU)→ 一个淡入 effect。**先打通,验证通信模式与帧率。**
2. 接 `fsm`(Part/Turn)+ 滚动/视口裁剪(input + scene 裁剪)
3. 接 `content` 全量(jcode markdown + remend + 高亮)
4. 接 `effects` profile + 富媒体 embed(图片→mermaid)
5. 补 `render` 降级(WebGL2 → Canvas2D)+ `api` 无障碍镜像 + 选区/复制
6. 后期:Worker 化(transport + layout + scene 移入 OffscreenCanvas Worker)

每步都保持"端到端可跑",避免先做完单模块再集成的大爆炸。

---

## 八、与既有决策的对应

| 模块 | 决策文档 |
|---|---|
| transport | [0003](decision/0003-fault-tolerance.md) [0008](decision/0008-multi-instance-sync.md) |
| protocol / store | [0001](decision/0001-canvas-architecture.md) [0002](decision/0002-event-driven-pipeline.md) [0003](decision/0003-fault-tolerance.md) |
| fsm | [0002](decision/0002-event-driven-pipeline.md) [0005](decision/0005-turn-aggregation-and-settlement.md) [0006](decision/0006-inline-tags-and-extensibility.md) |
| smoother | [0002](decision/0002-event-driven-pipeline.md) |
| content | [0004](decision/0004-markdown-and-embeds.md) [0006](decision/0006-inline-tags-and-extensibility.md) |
| layout | [0001](decision/0001-canvas-architecture.md) [0004](decision/0004-markdown-and-embeds.md) |
| scene | [0001](decision/0001-canvas-architecture.md) [0004](decision/0004-markdown-and-embeds.md) [0007](decision/0007-rich-media-embeds.md) |
| effects / render | [0002](decision/0002-event-driven-pipeline.md) [0003](decision/0003-fault-tolerance.md) [0007](decision/0007-rich-media-embeds.md) |
| input / api | [0002](decision/0002-event-driven-pipeline.md) [0007](decision/0007-rich-media-embeds.md) |
