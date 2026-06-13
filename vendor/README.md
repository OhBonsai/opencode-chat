# vendor/

第三方源码 vendored 进本仓库(plan2 H1 指定)。这些 crate **不进 workspace members**,
不套用我们的 `[workspace.lints]` / fmt 门禁(`Cargo.toml` 的 `exclude`),原样保留。

## jcode-render-core

- 来源:`~/w/agentscode/jcode/crates/jcode-render-core`(jcode 项目)
- 用途:后端中立的 markdown 文档模型 —— `parse_markdown(text) -> Document`
  (Block/Line/Span + 表格/列表/数学/换行),`core/src/content.rs` 适配成我们的
  `StyledSpan` + 渲染角色。
- 依赖:`pulldown-cmark` / `unicode-width` / `serde`(纯 Rust,wasm 安全,无 ratatui/GPU)。
- 更新:上游变更时重新 `cp -R` 覆盖本目录即可;许可证随 jcode 项目。
