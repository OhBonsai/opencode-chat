# 决策记录 0006:文本内嵌标签与插件扩展

- 日期:2026-06-13
- 状态:已采纳(原型验证前)
- 前置:0002(管线 + 效果开关)、0004(markdown 语义层)、0005(收尾容错)
- 范围:模型 raw 输出 / 工具消息 / 插件注入的非 markdown 标签(`<thinking>` 等)的处理

## 1. 背景

流式文本里会嵌入非 markdown 的语义标签,来源三类:

- 模型 raw 输出里的 `<thinking>` 之类
- 工具消息正文里的标记
- opencode 插件注入的自定义 tag

需要在 markdown 解析**之前**插一个标签提取层。核心难点:流式边界、可扩展性、安全。

## 2. pre-markdown 标签扫描器(segmenter)

不能直接交给 jcode/pulldown-cmark——它会把 `<thinking>` 当 inline HTML 吞掉
(StyleRole::Html),语义丢失。故在 0004 管线最前面加一道扫描:

```
原始文本 → segmenter → [普通 markdown 段] + [标签区域] 交替序列
  普通段 → jcode parse → pretext layout(0004)
  标签区域 → 注册表 → 语义实体
```

## 3. 流式边界:hold 区(最关键)

标签会被 delta 切断:`<thi` | `nking>`。扫描器必须可恢复:

- 尾部留 **hold 区**:遇未闭合的 `<` 暂停提交,只送出之前无歧义的文本进布局,
  `<...` 悬着等后续 delta
- 与"未闭合代码围栏"(0004 §5 checkpoint)同类:**流式前沿永远有一小段不提交,
  直到歧义消解**
- 上限保护:超过阈值(如 1KB)仍无 `>` → 判定非标签,当普通文本放出,防止孤立
  `<` 卡死整条流

## 4. 标签注册表:已知 vs 未知

```rust
enum TagBehavior {
    Region { role: StyleRole, collapsible: bool, default_collapsed: bool }, // <thinking>
    Chip,        // <citation id=..> → 内联徽章
    Hidden,      // 纯控制标记,不渲染
    Literal,     // 原样当文本显示
}
fn resolve(tag_name: &str) -> TagBehavior  // 查不到走默认策略
```

- 已知控制标签(thinking/citation/工具标注)→ 语义区域或 chip,与 StyledSpan 同管线
- **未知标签默认保守 = `Literal`**(原样显示,绝不静默吞掉)。白名单模式下才
  `Hidden`。不默认 strip——否则插件出 bug 时内容凭空消失无法排查

## 5. 标签区域是带 FSM 的实体(套用 0005 收尾容错)

```
Open    开标签到 → 渲染为进行中(如推理区)
Settled 闭标签到 → 收尾
```

模型常忘记吐闭标签(与 0005"忘记吐 idle"同构)。同款兜底——闭标签缺失时,在
以下任一时机隐式闭合:

- 块边界(markdown block 结束)
- turn 收尾(0005)
- 下一个同级标签出现

不能让未闭合的 `<thinking>` 把后续正文全吃进推理区。

## 6. 安全:标签是数据,永不解释为真 HTML

来源(模型输出、插件)均不可信。画布渲染无 DOM 故无 XSS,但仍当纯数据:
不执行、不当真 HTML 解析、属性值只读不 eval。注册表是唯一行为来源。

## 7. 插件扩展点 = 注册表(与 0002 §5.1 同构)

插件/配置只能注册 `tag 名 → TagBehavior`,核心闭合、行为开放。与"效果是数据
不是分支"同哲学:新增插件标签不动管线代码,只加一条注册。注册表可热加载。

## 8. 协议层事实:两条 reasoning 通道

opencode 已把 reasoning 拆成独立 part type(`reasoning`),思考主通道走 part 而非
文本内标签。文本内 `<thinking>` 主要来自**插件注入**或**某些模型把思考混在 text
part 里**。

设计要求:两条通道(part FSM 0002 / 文本内标签区域 FSM 本文 §5)产出**同一种
语义区域**,渲染层不区分来源,视觉表现统一。

## 9. 管线位置总结

```
SSE text.delta → part.text 累积(原始文本)
每帧(尾部脏块):
  segmenter(hold 区)→ [markdown 段] + [标签区域]
    markdown 段 → jcode parse → pretext layout → glyph instance(0004)
    标签区域 → 注册表 resolve → 语义区域/chip/hidden/literal
                → Region 进 FSM(§5),与 reasoning part 统一表现
  render
```
