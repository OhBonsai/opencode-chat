//! content(M6)— Plan1 仅纯文本直通。
//!
//! 完整管线(标签扫描 + markdown + 高亮,0004/0006)留 Plan2;本期把一段文本原样
//! 包成单个 [`StyledSpan`],样式角色后续再加。

/// 一段带样式角色的文本 run(content→layout 的契约,architecture §五.3)。
///
/// Plan1 只有 `text` 字段;Plan2 会加 role/attrs(jcode 风格)。
#[derive(Clone, Debug, PartialEq)]
pub struct StyledSpan {
    text: String,
}

impl StyledSpan {
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into() }
    }

    /// run 文本(只读访问,R4)。
    pub fn text(&self) -> &str {
        &self.text
    }
}

/// 纯文本直通:整段文本 → 单个 span。
pub fn plain(text: &str) -> Vec<StyledSpan> {
    if text.is_empty() {
        Vec::new()
    } else {
        vec![StyledSpan::new(text)]
    }
}
