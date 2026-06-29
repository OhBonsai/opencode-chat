//! partspecific — Plan 23 的 **specific 漂亮渲染器**(消费 0033 契约,覆盖兜底)。
//!
//! 设计定调(plan23 §0):
//! - **数据驱动两级分派**:`PartKind` 走 [`RenderRegistry`];tool 再按 *工具名* 在 [`tool_render`]
//!   内二级分派(bash/read/edit…),加工具 = 改这一处 match,不动 `build_frame` 骨架(0006)。
//! - **CR1 / R8**:每个渲染器是纯 [`RenderFn`](`fn(PartKind,&RenderPart,&RenderCtx)->Vec<StyledSpan>`),
//!   同输入同输出、零平台依赖、native 可测;且**每个都过** [`crate::partrender::assert_renderfn_conforms`]
//!   契约一致性闸(不 panic + 确定 + 非空输入非空输出)—— 保护与 Plan 22 并行不互相破坏。
//! - **SKIP 对齐 opencode**:patch/step part 不渲染;diff 来自 **tool 的 `metadata.filediff`**,不是 patch part。
//!
//! 视觉(卡底面板 / 左条 / diff 行底色 / 徽章色)由 `app.rs` 按 [`StyleRole`] 画装饰;本模块只决定
//! "是什么文字 + 什么角色",输出对齐既有 content→layout 契约(0001)。

#![allow(clippy::trivially_copy_pass_by_ref)] // RenderFn 统一签名按引用收 ctx(同 partrender.rs)

use serde_json::Value;

use crate::content::{parse_markdown, StyleRole, StyledSpan};
use crate::partrender::{PartKind, RenderCtx, RenderPart, RenderRegistry};

/// Plan 23 起步注册表:在全兜底基础上 `register` 已实现的 specific 渲染器。
/// **加渲染器 = 这里多一行 + 一条 N3 契约测试**(plan23 §0/§4),其余 kind 继续走兜底 → UI 始终完整。
#[must_use]
pub fn default_registry() -> RenderRegistry {
    let mut reg = RenderRegistry::new();
    reg.register(PartKind::Reasoning, reasoning_render); // R1
    reg.register(PartKind::Compaction, compaction_render); // R1
    reg.register(PartKind::Tool, tool_render); // R2 + R3(diff 二级分派)
    reg
}

// ───────────────────────── R1:reasoning + compaction ─────────────────────────

/// 推理 / 思考区(0006):合成 `💭 Thinking` 标题行 + 弱化正文(markdown 结构保留,普通文走
/// [`StyleRole::Reasoning`] 弱化色)。折叠态由 ctx 控制:`folded` → 只出标题(降噪)。
fn reasoning_render(_kind: PartKind, part: &RenderPart, ctx: &RenderCtx) -> Vec<StyledSpan> {
    let mut out = vec![StyledSpan::new("💭 Thinking\n", StyleRole::ToolTitle)];
    if ctx.folded || part.text.is_empty() {
        return out;
    }
    for span in parse_markdown(&part.text) {
        out.push(dim_reasoning(span));
    }
    out
}

/// 把普通正文角色降级到 [`StyleRole::Reasoning`](弱化);代码/标题等结构角色保留以不丢语义。
fn dim_reasoning(span: StyledSpan) -> StyledSpan {
    match span.role() {
        StyleRole::Normal | StyleRole::Bold | StyleRole::Italic | StyleRole::BoldItalic => {
            StyledSpan::styled(
                span.text().to_owned(),
                StyleRole::Reasoning,
                span.is_struck(),
            )
        }
        _ => span,
    }
}

/// 上下文压缩通知(0026):标签行 + 整宽分隔线锚点(render 据 [`StyleRole::Rule`] 画细线)。
fn compaction_render(_kind: PartKind, _part: &RenderPart, _ctx: &RenderCtx) -> Vec<StyledSpan> {
    vec![
        StyledSpan::new("上下文已压缩 · Context compacted\n", StyleRole::Reasoning),
        StyledSpan::new(" ", StyleRole::Rule),
    ]
}

// ───────────────────────── R2:通用 tool 卡(+ R3 diff 二级分派)─────────────────────────

/// 工具调用状态(plan23 §1.2;0031 已解码 `ToolState.status`)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ToolStatus {
    Pending,
    Running,
    Completed,
    Error,
    /// 未知 / 缺失:**不隐藏内容**(content-first),按 completed 展示。
    Unknown,
}

impl ToolStatus {
    fn parse(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "pending" => Self::Pending,
            "running" => Self::Running,
            "completed" | "done" | "success" => Self::Completed,
            "error" | "failed" => Self::Error,
            _ => Self::Unknown,
        }
    }

    /// pending/running:隐藏 args/output(plan23 §1.2,降噪 + 防半截内容闪)。
    fn hides_body(self) -> bool {
        matches!(self, Self::Pending | Self::Running)
    }

    /// 状态徽章文本(render 侧据状态上色;此处只给文字)。
    fn badge(self) -> &'static str {
        match self {
            Self::Pending => "[pending]",
            Self::Running => "[running]",
            Self::Completed => "[done]",
            Self::Error => "[error]",
            Self::Unknown => "[·]",
        }
    }
}

/// 通用 tool 卡:`▸ <name>  [status]` 标题行 + 状态分派的 args/output;edit/write/apply_patch 走 diff。
/// 工具名二级分派在此(plan23 §0 数据驱动);未识别工具走"通用 args/output"分支(不丢内容)。
fn tool_render(_kind: PartKind, part: &RenderPart, _ctx: &RenderCtx) -> Vec<StyledSpan> {
    let (name, status) = parse_tool_tag(&part.kind_tag);
    let mut out = vec![
        StyledSpan::new(format!("▸ {name}"), StyleRole::ToolTitle),
        StyledSpan::new(format!("  {}\n", status.badge()), StyleRole::ToolBadge),
    ];

    let obj = json_object(part.payload_json.as_deref());

    // R3:diff 工具 —— 优先用 metadata.filediff 出增删块(替代普通 output)。
    if is_diff_tool(name) {
        if let Some(diff) = obj.as_ref().and_then(filediff_of) {
            out.extend(render_diff(&diff));
            return out;
        }
    }

    if status.hides_body() {
        return out; // 运行中:只留标题 + 徽章
    }

    // 通用:args(input,弱化)+ output / error(中性);JSON 不可解析则原样 dump(不丢内容)。
    match &obj {
        Some(map) => {
            if let Some(input) = map.get("input").filter(|v| !v.is_null()) {
                out.push(StyledSpan::new(
                    format!("{}\n", compact_json(input)),
                    StyleRole::ToolArg,
                ));
            }
            let body = map
                .get("error")
                .or_else(|| map.get("output"))
                .and_then(Value::as_str)
                .unwrap_or("");
            if !body.is_empty() {
                out.push(StyledSpan::new(body.to_owned(), StyleRole::ToolOutput));
            }
        }
        None => {
            if let Some(raw) = &part.payload_json {
                if !raw.is_empty() {
                    out.push(StyledSpan::new(raw.clone(), StyleRole::ToolArg));
                }
            }
        }
    }

    // 正文(若 part 自带 text,如 webfetch 摘要):兜在最后,markdown 渲染。
    if !part.text.is_empty() {
        out.extend(parse_markdown(&part.text));
    }
    out
}

/// 解析身份标签 `tool:<name> · <status>` → (工具名, 状态)。无 `tool:` 前缀/无 `·` 均稳健兜底。
fn parse_tool_tag(tag: &str) -> (&str, ToolStatus) {
    let body = tag.strip_prefix("tool:").unwrap_or(tag);
    let mut it = body.splitn(2, '·');
    let name = it.next().unwrap_or("").trim();
    let name = if name.is_empty() { "tool" } else { name };
    let status = ToolStatus::parse(it.next().unwrap_or(""));
    (name, status)
}

/// edit/write/apply_patch:diff 挂 tool(plan23 §1 / SKIP patch part)。
fn is_diff_tool(name: &str) -> bool {
    matches!(name, "edit" | "write" | "apply_patch" | "patch")
}

/// `metadata.filediff` 或顶层 `filediff` → diff 文本。
fn filediff_of(map: &serde_json::Map<String, Value>) -> Option<String> {
    let v = map
        .get("metadata")
        .and_then(Value::as_object)
        .and_then(|m| m.get("filediff"))
        .or_else(|| map.get("filediff"))?;
    v.as_str().map(str::to_owned)
}

/// JSON 字符串 → 顶层对象(非对象 / 不可解析 → None,调用方原样兜底)。
fn json_object(payload: Option<&str>) -> Option<serde_json::Map<String, Value>> {
    match serde_json::from_str::<Value>(payload?) {
        Ok(Value::Object(m)) => Some(m),
        _ => None,
    }
}

/// 紧凑 JSON(determinism:serde_json 对象按 key 有序;序列化失败回退空串,不 panic)。
fn compact_json(v: &Value) -> String {
    serde_json::to_string(v).unwrap_or_default()
}

// ───────────────────────── R3:diff 解析 + 渲染 ─────────────────────────

/// 一行 diff 的分类(朴素 unified-diff:按首字符)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffKind {
    /// 新增行(`+`,排除 `+++` 文件头)。
    Added,
    /// 删除行(`-`,排除 `---` 文件头)。
    Removed,
    /// hunk 头(`@@`)。
    Hunk,
    /// 上下文 / 文件头 / 其它。
    Context,
}

/// 一行 diff:分类 + 文本(去掉首个 +/- 标记符,文件头/hunk 保留原样)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffLine {
    pub kind: DiffKind,
    pub text: String,
}

/// `filediff`(unified diff)→ 逐行分类(纯函数,N2)。朴素规则,确定:
/// `@@`→Hunk;`+++`/`---`→Context(文件头);`+`→Added;`-`→Removed;其余→Context。
#[must_use]
pub fn diff_parse_lines(diff: &str) -> Vec<DiffLine> {
    diff.lines()
        .map(|line| {
            let (kind, text) = if line.starts_with("@@") {
                (DiffKind::Hunk, line)
            } else if line.starts_with("+++") || line.starts_with("---") {
                (DiffKind::Context, line)
            } else if let Some(rest) = line.strip_prefix('+') {
                (DiffKind::Added, rest)
            } else if let Some(rest) = line.strip_prefix('-') {
                (DiffKind::Removed, rest)
            } else {
                (DiffKind::Context, line)
            };
            DiffLine {
                kind,
                text: text.to_owned(),
            }
        })
        .collect()
}

/// diff 块渲染:`+a -d` 变更摘要徽章 + 每行(增=绿/删=红/上下文=弱化 output)。
fn render_diff(diff: &str) -> Vec<StyledSpan> {
    let lines = diff_parse_lines(diff);
    let added = lines.iter().filter(|l| l.kind == DiffKind::Added).count();
    let removed = lines.iter().filter(|l| l.kind == DiffKind::Removed).count();

    let mut out = vec![StyledSpan::new(
        format!("+{added} -{removed}\n"),
        StyleRole::ToolBadge,
    )];
    for l in &lines {
        let role = match l.kind {
            DiffKind::Added => StyleRole::DiffAdded,
            DiffKind::Removed => StyleRole::DiffRemoved,
            DiffKind::Hunk => StyleRole::ToolArg,
            DiffKind::Context => StyleRole::ToolOutput,
        };
        let mark = match l.kind {
            DiffKind::Added => "+",
            DiffKind::Removed => "-",
            _ => " ",
        };
        out.push(StyledSpan::new(format!("{mark}{}\n", l.text), role));
    }
    out
}

// ───────────────────────── R4:回合分组(三桶 + context 折叠)─────────────────────────

/// 分组用的最小 part 描述(core 纯函数输入;Plan 22 据 store 填)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartRef {
    pub kind: PartKind,
    /// 工具名(`kind == Tool` 时有意义,如 `read`/`bash`;其余空)。
    pub tool: String,
}

impl PartRef {
    #[must_use]
    pub fn new(kind: PartKind, tool: impl Into<String>) -> Self {
        Self {
            kind,
            tool: tool.into(),
        }
    }
}

/// 回合内 part 的分组结果(plan23 §1.1)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PartGroup {
    /// 辅助时间线单块(reasoning / tool 卡 / 中间文本)。
    Aux(usize),
    /// 连续 read/glob/grep/list → 一个 "Gathered context" 折叠组(降噪)。
    Context(Vec<usize>),
    /// 最终回复(尾部 text / file 主体)。
    Final(usize),
}

/// 检索类工具(连续段折叠成 context 组;plan23 §1.1,采纳 opencode 降噪)。
fn is_context_tool(name: &str) -> bool {
    matches!(name, "read" | "glob" | "grep" | "list")
}

fn is_final_kind(kind: PartKind) -> bool {
    matches!(kind, PartKind::Text | PartKind::File)
}

/// `group_message_parts`(CR1 纯函数,N1):turn 内 part →
/// - **最终回复**:尾部连续 text/file → [`PartGroup::Final`](主体);
/// - **辅助时间线**(其前):连续 read/glob/grep/list 折叠成 [`PartGroup::Context`];其余 → [`PartGroup::Aux`]。
///
/// 不变量:每个下标恰好出现一次;Context 组成员全是检索工具且极大连续段;Final 仅含尾部 text/file。
#[must_use]
pub fn group_message_parts(parts: &[PartRef]) -> Vec<PartGroup> {
    // 尾部连续 text/file = 最终回复区;之前 = 辅助时间线。
    let mut final_start = parts.len();
    while final_start > 0 && is_final_kind(parts[final_start - 1].kind) {
        final_start -= 1;
    }

    let mut out = Vec::new();
    let mut i = 0;
    while i < final_start {
        let p = &parts[i];
        if p.kind == PartKind::Tool && is_context_tool(&p.tool) {
            // 折叠极大连续检索段。
            let mut group = vec![i];
            let mut j = i + 1;
            while j < final_start
                && parts[j].kind == PartKind::Tool
                && is_context_tool(&parts[j].tool)
            {
                group.push(j);
                j += 1;
            }
            out.push(PartGroup::Context(group));
            i = j;
        } else {
            out.push(PartGroup::Aux(i));
            i += 1;
        }
    }
    for idx in final_start..parts.len() {
        out.push(PartGroup::Final(idx));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{
        compaction_render, default_registry, diff_parse_lines, group_message_parts,
        reasoning_render, tool_render, DiffKind, PartGroup, PartRef,
    };
    use crate::content::{StyleRole, StyledSpan};
    use crate::partrender::{assert_renderfn_conforms, PartKind, RenderCtx, RenderPart};

    fn part(tag: &str, text: &str, json: Option<&str>) -> RenderPart {
        RenderPart {
            kind_tag: tag.to_owned(),
            text: text.to_owned(),
            payload_json: json.map(str::to_owned),
        }
    }

    fn joined(spans: &[StyledSpan]) -> String {
        spans.iter().map(StyledSpan::text).collect()
    }

    fn has_role(spans: &[StyledSpan], role: StyleRole) -> bool {
        spans.iter().any(|s| s.role() == role)
    }

    /// 渲染输出转可读快照串(N4):每行 `角色 | 文本`(\n 转义),稳定可 diff。
    fn dump(spans: &[StyledSpan]) -> String {
        spans
            .iter()
            .map(|s| format!("{:?} | {}", s.role(), s.text().replace('\n', "\\n")))
            .collect::<Vec<_>>()
            .join("\n")
    }

    // ── N3 契约一致性闸:每个 specific 渲染器必过(plan23 §3.7 N3 / 0033 §3)。
    #[test]
    fn reasoning_conforms() {
        assert_renderfn_conforms(reasoning_render);
    }
    #[test]
    fn compaction_conforms() {
        assert_renderfn_conforms(compaction_render);
    }
    #[test]
    fn tool_conforms() {
        assert_renderfn_conforms(tool_render);
    }

    // ── R1
    #[test]
    fn reasoning_synthesizes_thinking_header_and_dims_body() {
        let p = part("reasoning", "先看 **要点** 再决定", None);
        let spans = reasoning_render(PartKind::Reasoning, &p, &RenderCtx::default());
        let s = joined(&spans);
        assert!(s.contains("Thinking"), "缺合成标题: {s}");
        assert!(s.contains("先看"), "正文丢了: {s}");
        assert!(has_role(&spans, StyleRole::Reasoning), "正文未弱化");
    }

    #[test]
    fn reasoning_folded_keeps_only_header() {
        let p = part("reasoning", "一大段思考过程", None);
        let ctx = RenderCtx {
            width: 0.0,
            folded: true,
        };
        let spans = reasoning_render(PartKind::Reasoning, &p, &ctx);
        assert_eq!(spans.len(), 1, "折叠态应只留标题");
        assert!(!joined(&spans).contains("一大段"), "折叠态不应出正文");
    }

    #[test]
    fn compaction_has_label_and_rule() {
        let spans = compaction_render(
            PartKind::Compaction,
            &part("compaction", "", None),
            &RenderCtx::default(),
        );
        assert!(joined(&spans).contains("压缩"), "缺压缩标签");
        assert!(has_role(&spans, StyleRole::Rule), "缺分隔线锚点");
    }

    // ── R2
    #[test]
    fn tool_title_badge_and_output() {
        let p = part(
            "tool:bash · completed",
            "",
            Some(r#"{"input":{"cmd":"ls"},"output":"a.txt\nb.txt"}"#),
        );
        let spans = tool_render(PartKind::Tool, &p, &RenderCtx::default());
        let s = joined(&spans);
        assert!(s.contains("bash"), "缺工具名: {s}");
        assert!(s.contains("[done]"), "缺完成徽章: {s}");
        assert!(s.contains("a.txt"), "缺 output: {s}");
        assert!(has_role(&spans, StyleRole::ToolTitle));
        assert!(has_role(&spans, StyleRole::ToolOutput));
    }

    #[test]
    fn tool_running_hides_body() {
        let p = part(
            "tool:bash · running",
            "",
            Some(r#"{"input":{"cmd":"ls"},"output":"x"}"#),
        );
        let spans = tool_render(PartKind::Tool, &p, &RenderCtx::default());
        let s = joined(&spans);
        assert!(s.contains("[running]"), "缺 running 徽章: {s}");
        assert!(!s.contains('x'), "运行中不应露 output: {s}");
    }

    #[test]
    fn tool_error_shows_error_text() {
        let p = part("tool:bash · error", "", Some(r#"{"error":"boom"}"#));
        let spans = tool_render(PartKind::Tool, &p, &RenderCtx::default());
        let s = joined(&spans);
        assert!(s.contains("[error]") && s.contains("boom"), "错误未显: {s}");
    }

    #[test]
    fn tool_unparseable_json_keeps_content() {
        // 非对象 JSON → 原样兜底,不丢内容。
        let p = part("tool:custom · done", "", Some("[1,2,3]"));
        let spans = tool_render(PartKind::Tool, &p, &RenderCtx::default());
        assert!(joined(&spans).contains("[1,2,3]"), "原始内容丢了");
    }

    // ── R3
    #[test]
    fn diff_parse_classifies_lines() {
        let d = "@@ -1,2 +1,2 @@\n ctx\n-old\n+new\n+++ b/f\n";
        let lines = diff_parse_lines(d);
        let kinds: Vec<DiffKind> = lines.iter().map(|l| l.kind).collect();
        assert_eq!(
            kinds,
            vec![
                DiffKind::Hunk,
                DiffKind::Context,
                DiffKind::Removed,
                DiffKind::Added,
                DiffKind::Context, // +++ 文件头不算新增
            ]
        );
        assert_eq!(lines[2].text, "old"); // 去掉 - 标记
        assert_eq!(lines[3].text, "new"); // 去掉 + 标记
    }

    #[test]
    fn tool_diff_renders_added_removed_and_summary() {
        let p = part(
            "tool:edit · completed",
            "",
            Some(r#"{"metadata":{"filediff":"@@ -1 +1 @@\n-old\n+new\n"}}"#),
        );
        let spans = tool_render(PartKind::Tool, &p, &RenderCtx::default());
        let s = joined(&spans);
        assert!(s.contains("+1 -1"), "缺变更摘要: {s}");
        assert!(has_role(&spans, StyleRole::DiffAdded), "缺新增行角色");
        assert!(has_role(&spans, StyleRole::DiffRemoved), "缺删除行角色");
    }

    // ── R4
    #[test]
    fn grouping_folds_context_and_marks_final() {
        let parts = vec![
            PartRef::new(PartKind::Reasoning, ""),
            PartRef::new(PartKind::Tool, "read"),
            PartRef::new(PartKind::Tool, "grep"),
            PartRef::new(PartKind::Tool, "bash"),
            PartRef::new(PartKind::Text, ""),
        ];
        let groups = group_message_parts(&parts);
        assert_eq!(
            groups,
            vec![
                PartGroup::Aux(0),
                PartGroup::Context(vec![1, 2]), // read+grep 折叠
                PartGroup::Aux(3),              // bash 独立
                PartGroup::Final(4),            // 尾部 text
            ]
        );
    }

    #[test]
    fn grouping_trailing_text_file_all_final() {
        let parts = vec![
            PartRef::new(PartKind::Tool, "bash"),
            PartRef::new(PartKind::File, ""),
            PartRef::new(PartKind::Text, ""),
        ];
        let groups = group_message_parts(&parts);
        assert_eq!(
            groups,
            vec![PartGroup::Aux(0), PartGroup::Final(1), PartGroup::Final(2)]
        );
    }

    // ── N6 覆盖:specific 注册 + 输出 ≠ 兜底
    #[test]
    fn registry_covers_and_differs_from_fallback() {
        let reg = default_registry();
        for kind in [PartKind::Reasoning, PartKind::Compaction, PartKind::Tool] {
            assert!(reg.has_specific(kind), "{kind:?} 未注册");
        }
        // specific 输出 ≠ 兜底(以 reasoning 为例:合成 Thinking 标题,非 "[reasoning]")。
        let p = part("reasoning", "x", None);
        let specific = joined(&reg.render(PartKind::Reasoning, &p, &RenderCtx::default()));
        let fallback = joined(&crate::partrender::fallback_render(
            PartKind::Reasoning,
            &p,
            &RenderCtx::default(),
        ));
        assert_ne!(specific, fallback, "specific 应区别于兜底");
        assert!(specific.contains("Thinking"));
    }

    // ── N4:各 kind 渲染输出快照(StyledSpan 角色+文本),随 status/折叠态确定(plan23 §3.7 N4)。
    #[test]
    fn render_snapshots() {
        let ctx = RenderCtx::default();
        insta::assert_snapshot!(
            "reasoning",
            dump(&reasoning_render(
                PartKind::Reasoning,
                &part("reasoning", "先 `读码` 再 **决定**", None),
                &ctx
            ))
        );
        insta::assert_snapshot!(
            "compaction",
            dump(&compaction_render(
                PartKind::Compaction,
                &part("compaction", "", None),
                &ctx
            ))
        );
        insta::assert_snapshot!(
            "tool_bash_done",
            dump(&tool_render(
                PartKind::Tool,
                &part(
                    "tool:bash · completed",
                    "",
                    Some(r#"{"input":{"cmd":"ls -la"},"output":"a.txt\nb.txt"}"#)
                ),
                &ctx
            ))
        );
        insta::assert_snapshot!(
            "tool_running",
            dump(&tool_render(
                PartKind::Tool,
                &part(
                    "tool:grep · running",
                    "",
                    Some(r#"{"input":{"pattern":"foo"}}"#)
                ),
                &ctx
            ))
        );
        insta::assert_snapshot!(
            "tool_edit_diff",
            dump(&tool_render(
                PartKind::Tool,
                &part(
                    "tool:edit · completed",
                    "",
                    Some(r#"{"metadata":{"filediff":"@@ -1,2 +1,2 @@\n ctx\n-old\n+new\n"}}"#)
                ),
                &ctx
            ))
        );
    }

    use proptest::prelude::*;

    proptest! {
        // N2:diff 解析确定 + 增删计数 = 朴素 char 重数(R8)。
        #[test]
        fn diff_parse_deterministic_and_counts(diff in "[+\\- @a-z\\n]{0,200}") {
            let a = diff_parse_lines(&diff);
            let b = diff_parse_lines(&diff);
            prop_assert_eq!(&a, &b);
            // 朴素重数:行首 '+' 且非 "+++" / '-' 且非 "---" / 非 "@@"。
            let naive_add = diff.lines().filter(|l|
                l.starts_with('+') && !l.starts_with("+++") && !l.starts_with("@@")).count();
            let parsed_add = a.iter().filter(|l| l.kind == DiffKind::Added).count();
            prop_assert_eq!(naive_add, parsed_add);
        }

        // N1:分组确定 + 全覆盖(每下标恰一次)+ Final 仅尾部 text/file。
        #[test]
        fn grouping_deterministic_total_coverage(
            seq in proptest::collection::vec(0u8..5, 0..30)
        ) {
            let parts: Vec<PartRef> = seq.iter().map(|&t| {
                let kind = match t {
                    0 => PartKind::Text,
                    1 => PartKind::Reasoning,
                    2 => PartKind::Tool,
                    3 => PartKind::File,
                    _ => PartKind::Compaction,
                };
                // tool 一半是检索类(触发折叠),一半 bash。
                let tool = if kind == PartKind::Tool { "read" } else { "" };
                PartRef::new(kind, tool)
            }).collect();

            let a = group_message_parts(&parts);
            let b = group_message_parts(&parts);
            prop_assert_eq!(&a, &b);

            // 全覆盖:展开所有 group 的下标 = 0..n 各一次。
            let mut seen = vec![false; parts.len()];
            for g in &a {
                match g {
                    PartGroup::Aux(i) | PartGroup::Final(i) => {
                        prop_assert!(!seen[*i]);
                        seen[*i] = true;
                    }
                    PartGroup::Context(ids) => for i in ids {
                        prop_assert!(!seen[*i]);
                        seen[*i] = true;
                    }
                }
            }
            prop_assert!(seen.iter().all(|&b| b));

            // Final 仅含 text/file。
            for g in &a {
                if let PartGroup::Final(i) = g {
                    prop_assert!(matches!(parts[*i].kind, PartKind::Text | PartKind::File));
                }
            }
        }
    }
}
