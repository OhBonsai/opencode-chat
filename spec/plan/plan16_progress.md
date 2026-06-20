# Plan 16 进度(ShaderBox:矩形 shader 画板 + 五护栏 + 度量 + 50-icon 库 + agent logo)

- 状态(2026-06-21):**①②③③′③″⑤⑥ core/render/wasm/tsc 可验部分全落地 + 测试通过**;
  ④(输入纹理 channel)**有意延后**(无消费方、仅 GPU 人工验、风险 vs 收益,详见末节)。
  出图 / 动效 / raymarch / 缩放锐利须人工 GPU。
- 沙箱约束(同 Plan 12–15):cargo(native + wasm32)+ tsc + wgsl(naga)解析,无 GPU/浏览器 →
  视觉(icon 上屏、呼吸/旋转、glow-orb 头像、raymarch、morph)须人工实跑。

## 已落地(验证)

| 相位 | 落地(file:符号) | 验证 |
|---|---|---|
| **① 图元 + 管线** | core:`FrameShaderBox`(frame.rs:pos/size/shader_id/params[8]/bg/time/dynamic)+ `shaderbox.rs`(`ShaderId`/`IconId`/`ShaderboxClock`);render:`ShaderBoxInstance`(scene.rs,6 顶点属性)+ `shaderbox/common.wgsl`(SDF 工具箱)+ `fs.wgsl`;`backend.rs` 每 shader-id 一 pipeline（`make_shaderbox_pipeline`,复用 globals group0）+ draw pass(图后字前,逐实例选 pipeline)+ `ensure_shaderbox_buffer`;wasm:`FrameShaderBox`→`ShaderBoxInstance` 转换 + draw | cargo;wgsl naga(icons/glow_orb/raymarch 三 shader 解析过);**GPU 人工**:画板出图 + params 改色 |
| **② 护栏 1/2/4 + 度量** | `ShaderboxClock`(30fps 节流,护栏4,`advance` tick)；build_frame:护栏1 cull(box∉视口不发)、护栏2 静态即冻(静态 icon `time=0` 常量 → GPU 结果不变可冻;dynamic 才喂节流时钟)；`FrameStats.shaderbox_active/shaderbox_pixels`(Σ box∩viewport 面积)；`Rect::overlap_area`;wasm `stats()` + `?debug` perf 行 + web debug-panel 行 | cargo:`throttle_clock_steps_at_30fps`、`overlap_area_clips_to_intersection`、`shaderbox_metrics_count_onscreen_pixels`、`shaderbox_culled_when_offscreen`;**GPU 人工**:离屏零耗、静态冻、节流 30fps |
| **③ 首批内置 shader + morph** | `icons.wgsl shade()` morph 钩子:`p0.x`=icon_a、`p0.y`=icon_b、`p0.z`=t → `mix(cov_a,cov_b,t)`(copy→✓ 等,接 plan15 §2.7;t=0 向后兼容单图标)；loading spinner = `IconId::spinner()`(TheWorld) | wgsl naga;cargo:`aliases_map_into_deck`;**GPU 人工**:morph 动效、缩放锐利 |
| **③′ 内置 icon 库(§2.5)** | `icons.wgsl`(PixelSpiritDeck 整盘 50 支 `switch(icon)`,附录 A 逐字 WGSL 直译;SDF 工具箱并入 `common.wgsl`)；`IconId` 注册(50 项,值=源 case 号 + `is_dynamic`:46 动 / 4 静)；功能别名 `copy/check/spinner` 映射盘内 icon | wgsl naga(50 支全过);cargo:`icon_values_match_source_case_numbers`、`exactly_four_static_icons`;**GPU 人工**:contact-sheet 50 格、46 呼吸/旋、4 静态冻 |
| **③″ Agent logo(§2.6)** | `glow_orb.wgsl`(**自写**:hash21/vnoise 噪声调制发光环 + 角向高光 + 径向辉光 + 脉冲)；build_frame 在每 assistant 盒左侧钉 dynamic `GlowOrb` 头像(32px,`AVATAR_PX`)；流式(未 settled)`p0.w` 脉冲提速；离屏 cull + 计入像素度量 | wgsl naga;cargo:`agent_glow_orb_logo_on_assistant_box`;**GPU 人工**:logo 呼吸、streaming 加速、离屏停 |
| **⑤ 收编 raymarch(留位)** | `raymarch.wgsl`(**自写**:3D sphere/box + `rm_smin` smooth-union + 法线/漫反射；`RM_MAX_STEPS=48` 步数封顶 = 护栏5 平台 caps 钩子)；`ShaderId::Raymarch=2` 注册 pipeline | wgsl naga:`shaderbox_raymarch_shader_is_valid_wgsl`;**GPU 人工**:小区域 raymarch;移动端降 cap(运行时探测后续) |
| **⑥ 度量 + 面积封顶钩子 + 卡口** | 度量见②；护栏3 `shaderbox_exceeds_area_cap`/`SHADERBOX_MAX_EDGE_PX=512`(超阈 box 走 downscale 钩子,v1 仅判定,内置 box ≤32px 不触发)；进度文档 + 相位表翻牌 | cargo:`area_cap_triggers_only_for_oversized_boxes`;全卡口绿 |

## 卡口状态(本轮)

- `cargo clippy --workspace --all-targets` → **绿(0 警告)**。
- `cargo test`(native)→ **绿**:core 175、render 21 全过。
- `cargo build`(core+render+wasm)→ **绿**;`npm run build:wasm`(wasm-pack)→ **绿**。
- `cd web && tsc --noEmit` → **绿**;render wgsl(naga 解析,含 raymarch)→ **绿**。
- `wasm-pack test --headless` / GPU 上屏 → 人工卡口(沙箱无浏览器)。

## 待人工 GPU / 浏览器实跑(代码已就位)

- **icon 上屏**:copy 图标(TheEmperor)在每代码块右上角、缩放锐利;50 格 contact-sheet 全出。
- **呼吸 / 旋转**:46 个 animated icon 按 `S=1+sin(t)/4` 呼吸或 `time` 旋;4 个静态冻。
- **agent glow-orb**:assistant 盒左侧发光环呼吸;流式脉冲加速;滚出视口即停。
- **copy→✓ morph**:`mix(copy, check, t)`(交互触发后续,见下)。
- **raymarch**:`ShaderId::Raymarch` 小盒里 3D SDF 旋转;移动端降级不崩(平台探测后续)。

## 仍属范围 / 后续

- **④ 输入纹理 channel(本轮有意延后)**:`box 喂静态纹理作 channel0 + 溶解/扫光示例`。延后理由:
  ① 需给 shaderbox pipeline 加 group1(纹理+采样器)+ per-box `channel0_tex` + 上传路径,改动触及当前
  **稳定的 icons/glow_orb/raymarch 三 pipeline**(它们只绑 group0);② **当前无消费方**(没有发射带纹理的
  ShaderBox 的内容路径);③ 验证仅 `GPU 人工`(沙箱不可验,除 wgsl 解析 + tsc);④ plan §5 本就把
  「复杂 channel 链」列后续。→ 作独立增量,有真实消费场景时再落(复用 plan14 image 纹理 + bind group)。
- **copy→✓ morph 交互触发**:shader morph 能力已就位(`p0.z`=t);缺「点 copy 图标 → 写剪贴板 → t 动画
  0→1→0」的命中测试 + 剪贴板 + 逐块 morph 动画态(core 暂无 copy-click infra)。薄后续。
- **agent logo 落点精修**:当前每 assistant **part-view** 各钉一个头像(多 part 消息会叠多个);理想是
  plan13 AsstBox **turn 级**头像位(Taffy 固定小 leaf,§2.6 落点)→ 一回合一头像。布局改动后续。
- **平台 caps 运行时探测(护栏5)**:raymarch 步数 cap 现为编译期常量;WebGL2/移动端运行时探测降 cap +
  降精度(换 shader/const)后续。
- **面积封顶 downscale 实现**:护栏3 现仅判定钩子;离屏纹理渲染 + 放大的实现待基准超标才启(§6)。
- **性能基准(§7)**:活跃 box × 像素 vs fps 基准入册待 GPU 实测(沙箱无法测)。

## 许可(§2.5/§6,记结论 + 风险)

PixelSpiritDeck 整盘 50 卡 icon = **vendored-with-risk,个人/非商用 test 自用**(作者拍板接受风险)。
卡造型 LICENSE 明禁商用**和**非商用产品使用 → **触发重审条件**(任一即停):转商用、公开仓库/分发二进制、
对外发布。届时 SDF helper(LYGIA 数学)可留,50 卡造型须整库自画或取授权。agent logo(glow_orb)/ raymarch
均**自写**(借技法 + LYGIA 噪声范式),不 verbatim 抄。
