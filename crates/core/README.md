# opencode-chat-core

平台无关的对话渲染内核(M2 protocol / M3 store / M5 smoother / M6 content / M13 app)。

## 铁律红线(见 [spec/dev-practices.md §4](../../spec/dev-practices.md))

- **CR1**:零 `wasm-bindgen`/`web-sys`/`wgpu` 依赖 → native `cargo test` 可跑。
- **CR2**:网络/排版/时钟/渲染走 [`seam`](src/seam.rs) trait 注入。
- **R8/R9**:不碰 `Instant::now`/裸 `rand`;时间以 `dt_ms` 注入逐帧累加 → 确定性重放。
- **AR4**:`delta` 乐观追加必配 `message.part.updated` 全量对账([`store`](src/store.rs))。
- **AR7**:吐字单位 = grapheme cluster([`smoother`](src/smoother.rs))。
- **AR12**:未知事件/Part → `Ignored`/`Other`,不 panic([`protocol`](src/protocol.rs))。

## 模块

| 文件 | 职责 |
|---|---|
| `seam.rs` | 平台缝 trait(Connection/LayoutEngine/Clock/RenderSink)+ DTO |
| `protocol.rs` | opencode 事件解码(Plan1 text 子集) |
| `store.rs` | 归一化三表 + 增量/全量对账 |
| `smoother.rs` | 逐 grapheme 匀速整流 + spawn_time |
| `content.rs` | 纯文本直通 StyledSpan(Plan2 接 markdown) |
| `app.rs` | `Engine<C,L,R>` 每帧编排 |
| `frame.rs` | FrameGlyph / FrameData(交 RenderSink) |
| `record.rs` | Recorder / Player(确定性录制重放) |
| `support.rs` | native stub:MonospaceLayout / NullSink / CollectSink(测试用) |
| `fsm.rs` | 占位,Plan2 |

## 测试

```bash
cargo test -p opencode-chat-core            # 单测
cargo test -p opencode-chat-core --test replay   # 确定性重放
```
