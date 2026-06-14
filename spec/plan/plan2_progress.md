# Plan 2 实施总结(plan2-usable-chat 落地记录)

- 日期:2026-06-14
- 范围:[plan2-usable-chat.md](./plan2-usable-chat.md) 全部 5 相位(F–J)
- 状态:**F–J 全部实现、过卡口、各自提交;markdown 已切到 jcode**;主体闭环可跑,体验项待真机
- 提交:`30eb758`(F)→ `82406cf`(G)→ `11b5240`(H)→ `c310ae7`(I)→ `e4c5b23`(J)
  → `06b7d63`(jcode 接入)→ `36fca09`/`5cfbbca`(ADR 0009/0010)
- 配套:[plan1 phase1_progress](./phase1_progress.md)、[../architecture.md](../architecture.md)、[../knowledge/opencode.md](../knowledge/opencode.md)

---

## 1. 一句话

把 Plan 1 的"流式文字可见"推进到"**正确、可用、可滚动的 markdown 对话**":刷新不丢历史、
长对话不卡、markdown 有结构有色、回合不卡 loading、弱网能自愈。新增 **46 个 native 测试**
(Plan 1 是 22),markdown 解析换成 vendored **jcode-render-core**。

---

## 2. 交付物(相对 Plan 1 的增量)

```
crates/core/src/
├── fsm.rs            # 占位 → 实现:TurnTracker 收尾看门狗(Phase I)
├── content.rs        # 纯文本直通 → jcode markdown 适配 + 角色(Phase H)
├── store.rs          # + 快照灌入 / session 归属 / 对账强化(F/J)
├── protocol.rs       # + parse_snapshot / Part.sessionID / session.status(F/I)
├── app.rs            # + 快照预热 / 块冻结缓存 / 视口裁剪 / 滚动 / 收尾 / resync
└── frame.rs          # FrameGlyph + style(角色上色,H)
crates/render/src/
├── scene.rs          # GpuInstance + style + glyph_key((style,cluster) 分桶)
└── shaders/glyph.wgsl# + 按 StyleRole 上色
crates/wasm/src/
├── transport.rs      # + fetch_snapshot(catch-up)
├── lib.rs            # + 快照预热 / ?session= / 滚轮 / viewport / 周期 resync
└── glyph_bridge.rs   # rasterize(cluster, style)
web/src/
├── pretext-bridge.ts # + fontForRole(按角色选字体)
└── glyph-raster.ts   # rasterize(cluster, style)
vendor/jcode-render-core/   # vendored markdown 文档模型(plan2 H1)
spec/decision/         # 0009 渲染引擎 / 0010 markdown 解析策略(两条新 ADR)
```

---

## 3. 各相位落地 + 验证

| 相位 | 内容 | 关键实现 | 验证 |
|---|---|---|---|
| **F** 快照/过滤 | 刷新不丢、晚开能看、`?session=` 生效 | `parse_snapshot`;`apply_snapshot`(catch-up 零淡入 AR6);partID→messageID→sessionID 归属;连 SSE 前先拉快照 | 快照灌入/归属/过滤/instant 单测 |
| **G** 滚动/裁剪/冻结 | 长对话不卡(核心卖点) | per-block 排版缓存(settled 块不重排);视口裁剪;scroll_offset+锚底;滚轮监听 | block-freeze(counting-layout)+ cull 测 |
| **H** markdown | 真实 md 回复有结构有色 | jcode `Document` → 角色 span(隐藏语法、表格/列表/数学);remend 防闪;FrameGlyph.style→着色器上色 | 7 content 测(含表格)+ naga shader |
| **I** 收尾 | "忘了 idle"不再永久 loading | `TurnTracker` 投影 + soft8s/hard30s 看门狗;session.status 解码;`turn_status()` | 收尾矩阵 6 测 + 集成 |
| **J** 容错 | 弱网/重连不丢不错 | `resync_from_snapshot`(只补缺不动 live);周期 resync + EventSource 自动重连;AR4 收敛/幂等测试 | 故障收敛 + 幂等 + resync 测 |

---

## 4. 卡口结果(全绿)

```
cargo fmt --all --check                                    ✓
cargo clippy --workspace --all-targets -- -D warnings      ✓ (native)
cargo clippy -p ...-wasm --target wasm32 -- -D warnings     ✓
cargo test --workspace                                      ✓ 46 测(proptest + 确定性重放 + naga)
cargo build -p ...-wasm --target wasm32-unknown-unknown     ✓
wasm-pack build + vite build                                ✓(pkg 已重生成)
```
> vendored `jcode-render-core` 经 `exclude` 排除出 members,不套我们严格 lint/fmt(第三方原样)。

---

## 5. 关键决策(本期产生)

- **markdown 改用 jcode-render-core**(plan2 H1):先用 pulldown-cmark 自接,后按计划 vendor jcode
  的后端中立文档模型(`Document`),`content.rs` 适配成我们的渲染角色;正确支持表格/列表/数学。
- **ADR 0009 文字渲染引擎**:排查到 jcode 实际渲染在 `jcode-desktop`(glyphon/cosmic-text)。决策
  **保留浏览器系统字体 JS 桥**(守 BR5 零字体打包),glyphon 作未来升级路径备案(需打包字体)。
- **ADR 0010 markdown 解析策略**:对比 warp(手写 nom + 行级流式 diff)。决策**沿用 pulldown-cmark**
  (省自维护;我们的块冻结 + remend 已解决流式)。可借鉴:可点超链接、行级 diff(留 Plan 3)。

---

## 6. 有据偏差 / 推迟项(均在代码/ADR 标注)

- **真 pretext per-role 精确度量**(H4):measureText 桥按 body 字体度量;粗/斜/code 光栅化已按角色
  换字体(视觉对,宽度近似)。精确度量推迟。
- **syntect 语法高亮**(H5):重依赖,推迟(代码块目前等宽+统一色)。jcode-render-core 也只给结构。
- **Turn 完整分组投影(AR11)+ 折叠 tool/reasoning(I5)**:本期做收尾判定(最痛项),分组/折叠推迟。
- **显式心跳 backoff 重连(J2)**:用"周期 resync + EventSource 自动重连"覆盖等价效果,显式 backoff 推迟。
- **可视滚动条 + 块内 glyph 级裁剪(G)**:块级裁剪已平坦,细化推迟。
- **glyphon 渲染引擎**:ADR 0009 决策暂不采用(BR5),备案。

---

## 7. 待真机验证(本环境无 GPU/浏览器)

像素/markdown 观感(含表格 `│` 分隔、角色上色)、≥60fps、10k 行 fps/内存曲线、滚动手感、
重连观感、收尾后 loading 解禁。运行:`node scripts/serve.mjs` → `cd web && npm run dev` → 先开
页面(`?server=...&session=ses_xxx` 现在能看历史+过滤)→ `node scripts/chat.mjs` 多轮。

---

## 8. 下一步 / Plan 3 入口

- **embed**(图片→mermaid→卡片,0007 三层)、**内嵌标签**(`<thinking>`,0006)。
- **input / 选区 / hit-test**:含 ADR 0010 记的**可点超链接**(借鉴 warp `hyperlink+Action`)。
- **SDF 字形 + 富 shader 效果**(发光/描边/溶解,0007);或按 ADR 0009 触发条件评估 glyphon。
- 渲染降级 WebGL2/Canvas2D(0003 §5)、无障碍 DOM 镜像。

---

## 9. 审核记录(2026-06-14)

对本文声明做代码层独立核实,**结论:属实,无注水**。可核实项全部对得上。

**核实通过:**

| 声明 | 结果 |
|---|---|
| 提交历史(F→G→H→I→J→jcode→ADR) | ✅ 8 个哈希全存在、描述/顺序吻合(`git log` 比对) |
| 46 个 native 测试 | ✅ 精确 46 个 `#[test]`;分相位吻合(content 7 / fsm 6 / replay 3 / store 收敛+幂等 / app 冻结·裁剪·快照·resync) |
| 各相位关键实现 | ✅ `parse_snapshot`/`prime_from_snapshot`/`resync_from_snapshot`/`TurnTracker`/`remend`/`fontForRole` 均在 |
| 测试名兑现卖点 | ✅ `block_freeze_skips_settled_relayout`、`viewport_culls_offscreen_blocks`、`final_state_converges_to_snapshot_under_faults`、`snapshot_apply_is_idempotent`、`replay_is_deterministic` |
| 推迟项确属推迟非破损 | ✅ `syntect` 0 引用、`glyphon` 0 引用,与 §6 / ADR 0009 一致 |
| 与 ADR 一致 | ✅ 0009(不上 glyphon)、0010(沿用 pulldown);jcode 实际开了 TABLES/MATH/STRIKETHROUGH/TASKLISTS/FOOTNOTES/GFM |
| 构建新鲜度 | ✅ target 编译于 06-13 17:07,晚于最新源文件 17:06 → 产物对应当前代码 |

**本环境未能独立复核(已诚实标注):**

- ⚠️ **卡口"全绿"未亲自重跑**:审核沙箱无 Rust 工具链(无 `cargo`),§4 的 fmt/clippy/test/wasm-build 仅由提交记录 + 构建产物佐证,未在审核环境 `cargo test` 复跑。
- ⚠️ **§7 真机项**(≥60fps、10k 行内存曲线、滚动/重连手感、像素观感)确未验,需 GPU+浏览器。

**小瑕疵(无伤大雅):**

- §4/§1 "46 测(proptest…)":46 是 `#[test]` 计数,`store.rs` 另有 1 个 `proptest!` 属性测试未计入,实际为 46+1。
- §3 H 仅列 表格/列表/数学,但 jcode 实际已开**脚注/任务列表/删除线**——比文档更全(印证 0006/0010 补记的"`[^1]` 直接用 `ENABLE_FOOTNOTES`")。

**审核置信度**:结构/提交/测试/代码四项高置信;唯一悬空为"运行时绿 + 真机体验",恰为本文 §7 自圈待验项。
