//! partrender — part 渲染契约(决策 0033):分派注册表 + 通用兜底。
//!
//! 这是 **Plan 22(数据 + 兜底)与 Plan 23(specific 漂亮渲染器)的唯一接触面**,抽成独立契约
//! 让二者并行开发。设计铁律:
//! - **CR1**:纯逻辑、零平台依赖、native 可测;输出对齐既有 content→layout 契约(0001)= [`StyledSpan`]。
//! - **R8**:[`RenderFn`] 是纯函数,同输入同输出 → 兜底与 specific 共用渲染快照测试、入录像重放 oracle。
//! - **数据驱动(0006/0032)**:加渲染器 = `register` 一行,不动事件/状态/`build_frame` 骨架。
//!
//! 不变量:`RenderRegistry::new()` 全兜底 → 任何 part(含 `Unknown`)都渲染、不 panic(AR12);
//! Plan 23 逐类 `register` 覆盖,未覆盖继续兜底 → **UI 始终完整,只是越来越漂亮**。

// `&RenderCtx` 是契约统一签名(ctx 将随 theme/status 增长 → 始终走引用);小 Copy 类型按引用传
// 会触发 pedantic 的 trivially_copy_pass_by_ref,此处按设计放行(同 RenderFn 类型别名形态)。
#![allow(clippy::trivially_copy_pass_by_ref)]

use std::collections::HashMap;

use crate::content::{parse_markdown, plain, StyleRole, StyledSpan};

/// part 的渲染种类(分派键)。未知 → [`PartKind::Unknown`] 走兜底(AR12 向前兼容)。
/// tool 的工具名不进此枚举:走 [`RenderPart::kind_tag`] 字符串 + Plan 23 二级分派。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PartKind {
    /// assistant 文本(markdown)。
    Text,
    /// 推理 / 思考(弱化区)。
    Reasoning,
    /// 工具调用(卡片;Plan 23 再按工具名分派)。
    Tool,
    /// 文件附件 / 嵌入。
    File,
    /// 上下文压缩通知(分隔线)。
    Compaction,
    /// 未注册 / 未知类型:走兜底,绝不丢、绝不 panic。
    Unknown,
}

/// 渲染面向的 part 投影(Plan 22 的 store 据 part 填;渲染器只读)。
/// 与 protocol `Part` **解耦**:契约只认这张投影,Plan 22 负责从 store 填它。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RenderPart {
    /// 身份标签(肉眼可读「是什么 + 状态」),如 `"tool:bash · running"` / `"reasoning"` / `"compaction"`。
    pub kind_tag: String,
    /// `text`/`reasoning` 的正文(markdown);非文本 part 可空。
    pub text: String,
    /// 结构化 part 的 JSON dump(tool 的 input/output 等);兜底据此把内容原样展示。
    pub payload_json: Option<String>,
}

/// 渲染器只读上下文。Plan 23 的 specific 渲染器据此排版/上色;兜底用得很少。
/// 字段**向后只增不改**(扩 theme/status 时追加)。
#[derive(Debug, Clone, Copy, Default)]
pub struct RenderCtx {
    /// 可用排版宽度(px);0 = 未知(兜底不依赖)。
    pub width: f32,
    /// 折叠态(by node id 的 presentation 输入;兜底忽略)。
    pub folded: bool,
}

/// 渲染器签名:`(kind, part 投影, 上下文) → StyledSpan 序列`(0001 输出契约,喂既有 layout)。
/// **纯函数(CR1/R8)**:同输入同输出。
pub type RenderFn = fn(PartKind, &RenderPart, &RenderCtx) -> Vec<StyledSpan>;

/// 通用兜底渲染器(Plan 22):任何 part → **身份标签 + 原始内容**(markdown / JSON 代码块)。
/// 丑但完整 —— 肉眼能看出「是什么 + 内容是什么」。Plan 23 用 specific 渲染器逐类覆盖。
#[must_use]
pub fn fallback_render(kind: PartKind, part: &RenderPart, _ctx: &RenderCtx) -> Vec<StyledSpan> {
    let mut out: Vec<StyledSpan> = Vec::new();

    // 身份标签:弱化色(Quote = 引用/弱化,0001),后跟换行。
    if !part.kind_tag.is_empty() {
        out.push(StyledSpan::new(
            format!("[{}]\n", part.kind_tag),
            StyleRole::Quote,
        ));
    }

    match kind {
        // 压缩 = 分隔线锚点:发零墨空格,render 据 Rule 角色画整宽细线(4B1)。
        PartKind::Compaction => out.push(StyledSpan::new(" ", StyleRole::Rule)),
        // 文本 / 推理:正文走 markdown(Plan 23 再弱化/折叠 reasoning)。
        PartKind::Text | PartKind::Reasoning => out.extend(parse_markdown(&part.text)),
        // 其余结构化 part:正文(若有)+ JSON 代码块(内容原样不丢)。
        PartKind::Tool | PartKind::File | PartKind::Unknown => {
            if !part.text.is_empty() {
                out.extend(parse_markdown(&part.text));
            }
            if let Some(json) = &part.payload_json {
                out.push(StyledSpan::new(json.clone(), StyleRole::CodeBlock));
            }
        }
    }

    if out.is_empty() {
        out = plain("(empty)");
    }
    out
}

/// 渲染分派注册表:`PartKind → RenderFn`,缺省走 [`fallback_render`]。
/// Plan 22 只装兜底;Plan 23 `register` specific 覆盖某 kind。**加渲染器 = 注册一行。**
pub struct RenderRegistry {
    specific: HashMap<PartKind, RenderFn>,
    fallback: RenderFn,
}

impl Default for RenderRegistry {
    fn default() -> Self {
        Self {
            specific: HashMap::new(),
            fallback: fallback_render,
        }
    }
}

impl RenderRegistry {
    /// 全兜底注册表(Plan 22 起步形态)。
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// 注册某 kind 的 specific 渲染器(Plan 23),覆盖兜底。
    pub fn register(&mut self, kind: PartKind, f: RenderFn) {
        self.specific.insert(kind, f);
    }

    /// 渲染一个 part:命中 specific 则用之,否则兜底。纯函数路径。
    #[must_use]
    pub fn render(&self, kind: PartKind, part: &RenderPart, ctx: &RenderCtx) -> Vec<StyledSpan> {
        let f = self.specific.get(&kind).copied().unwrap_or(self.fallback);
        f(kind, part, ctx)
    }

    /// 该 kind 是否已有 specific 渲染器(测试 / 可观测:Plan 23 覆盖进度)。
    #[must_use]
    pub fn has_specific(&self, kind: PartKind) -> bool {
        self.specific.contains_key(&kind)
    }
}

/// **渲染器契约一致性闸(0033 §3 不变量)**:任何注册进 [`RenderRegistry`] 的 [`RenderFn`] ——
/// Plan 22 的 [`fallback_render`] 与 Plan 23 的**每个 specific 渲染器** —— 都必须过此断言。
/// 这是保护 Plan 22↔23 并行不互相破坏的闸:某渲染器若 panic / 非确定 / 丢内容,这里立刻红。
///
/// 用法(Plan 23 的渲染器测试,跨模块复用):
/// `crate::partrender::assert_renderfn_conforms(my_specific_render);`
///
/// 断言三条:① 不 panic(任意对抗性 part);② 确定性(同输入同输出,R8);③ 非空输入→非空输出。
#[cfg(test)]
pub(crate) fn assert_renderfn_conforms(f: RenderFn) {
    let ctx = RenderCtx::default();
    for (kind, part) in adversarial_render_parts() {
        let a = f(kind, &part, &ctx); // 若 panic,此调用即让测试失败
        let b = f(kind, &part, &ctx);
        assert_eq!(a, b, "渲染器非确定: kind={kind:?} part={part:?}");
        let has_input =
            !part.kind_tag.is_empty() || !part.text.is_empty() || part.payload_json.is_some();
        if has_input {
            assert!(
                !a.is_empty(),
                "非空输入却空输出: kind={kind:?} part={part:?}"
            );
        }
    }
}

/// 契约一致性用的对抗性 part 样本(空 / 只标签 / 只 json / 畸形 unicode / 超长)× 全 [`PartKind`]。
#[cfg(test)]
pub(crate) fn adversarial_render_parts() -> Vec<(PartKind, RenderPart)> {
    let mk = |tag: &str, text: &str, json: Option<&str>| RenderPart {
        kind_tag: tag.to_owned(),
        text: text.to_owned(),
        payload_json: json.map(str::to_owned),
    };
    let bodies = [
        mk("", "", None),                                               // 全空
        mk("tag", "", None),                                            // 只标签
        mk("tag", "", Some("{}")),                                      // 空 json
        mk("tool:bash · running", "", Some("{\"cmd\":\"ls -la\\n\"}")), // 典型工具
        mk("reasoning", "# 标题\n**粗** `码`", None),                   // markdown
        mk("x", "🦀\u{200b}混合", Some("[1,2,3]")),                     // emoji/零宽/CJK 混合
        mk("long", &"a".repeat(2000), None),                            // 超长
    ];
    let kinds = [
        PartKind::Text,
        PartKind::Reasoning,
        PartKind::Tool,
        PartKind::File,
        PartKind::Compaction,
        PartKind::Unknown,
    ];
    let mut out = Vec::new();
    for k in kinds {
        for b in &bodies {
            out.push((k, b.clone()));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{
        assert_renderfn_conforms, fallback_render, PartKind, RenderCtx, RenderFn, RenderPart,
        RenderRegistry, StyleRole, StyledSpan,
    };

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

    #[test]
    fn fallback_labels_and_keeps_content() {
        // 兜底:身份标签 + 原始内容都在(丑但完整)。
        let p = part("tool:bash · running", "", Some("{\"cmd\":\"ls -la\"}"));
        let spans = fallback_render(PartKind::Tool, &p, &RenderCtx::default());
        let s = joined(&spans);
        assert!(s.contains("tool:bash · running"), "身份标签丢了: {s}");
        assert!(s.contains("\"cmd\":\"ls -la\""), "内容丢了: {s}");
    }

    #[test]
    fn fallback_deterministic() {
        // R8:同输入同输出。
        let p = part("reasoning", "先看 **要点** 再决定", None);
        let a = fallback_render(PartKind::Reasoning, &p, &RenderCtx::default());
        let b = fallback_render(PartKind::Reasoning, &p, &RenderCtx::default());
        assert_eq!(a, b);
    }

    #[test]
    fn registry_specific_overrides_fallback_others_keep_fallback() {
        fn pretty(_k: PartKind, _p: &RenderPart, _c: &RenderCtx) -> Vec<StyledSpan> {
            vec![StyledSpan::new("PRETTY", StyleRole::Normal)]
        }
        let f: RenderFn = pretty;

        let mut reg = RenderRegistry::new();
        let p = part("tool:bash", "", Some("{}"));

        // 未注册 → 兜底(含身份标签)。
        assert!(
            joined(&reg.render(PartKind::Tool, &p, &RenderCtx::default())).contains("tool:bash")
        );

        // 注册 specific → 覆盖该 kind。
        reg.register(PartKind::Tool, f);
        assert_eq!(
            joined(&reg.render(PartKind::Tool, &p, &RenderCtx::default())),
            "PRETTY"
        );
        assert!(reg.has_specific(PartKind::Tool));

        // 未覆盖的 kind 仍走兜底 → UI 始终完整。
        assert!(!reg.has_specific(PartKind::Reasoning));
        let rp = part("reasoning", "思考", None);
        assert!(
            joined(&reg.render(PartKind::Reasoning, &rp, &RenderCtx::default())).contains("思考")
        );
    }

    #[test]
    fn unknown_kind_falls_back_not_panic() {
        // AR12:未知类型不丢、不 panic、有兜底。
        let p = part("custom:foo", "", Some("{\"a\":1}"));
        let spans = RenderRegistry::new().render(PartKind::Unknown, &p, &RenderCtx::default());
        assert!(!spans.is_empty());
        assert!(joined(&spans).contains("custom:foo"));
    }

    #[test]
    fn empty_part_yields_placeholder() {
        let p = part("", "", None);
        let spans = fallback_render(PartKind::Unknown, &p, &RenderCtx::default());
        assert!(!spans.is_empty());
    }

    #[test]
    fn fallback_conforms_to_contract() {
        // 兜底过契约一致性闸:不 panic + 确定 + 内容不丢(0033 §3)。
        // Plan 23 每个 specific 渲染器的测试同样调 `assert_renderfn_conforms(...)`。
        assert_renderfn_conforms(fallback_render);
    }

    use proptest::prelude::*;

    proptest! {
        // 兜底对任意 part 投影确定(R8);随机覆盖补充固定对抗样本。
        #[test]
        fn fallback_deterministic_prop(
            tag in "[\\PC]{0,40}",
            text in "[\\PC]{0,200}",
            json in proptest::option::of("[\\PC]{0,80}"),
        ) {
            let part = RenderPart { kind_tag: tag, text, payload_json: json };
            for kind in [PartKind::Text, PartKind::Tool, PartKind::Compaction, PartKind::Unknown] {
                let a = fallback_render(kind, &part, &RenderCtx::default());
                let b = fallback_render(kind, &part, &RenderCtx::default());
                prop_assert_eq!(a, b);
            }
        }
    }
}
