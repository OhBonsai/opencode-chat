//! highlight(M15 后续 / research code-block-syntax-highlighting · 路 A)— 代码块**轻量语法高亮**:
//! 单遍词法扫描 → 每字符塌缩成 **8 个语义类**([`CodeClass`]),上层映射到 `StyleRole::Code*`、走现有
//! per-glyph role→色(0021 调色板,亮暗免费),render 零改。
//!
//! 引擎取舍(偏离 research 的 syntect、保其"角色塌缩"集成洞察):聊天代码块 = 8 色塌缩,**不需 AST/
//! TextMate 精度**;手写词法器**零新依赖、wasm 极小、native 全可测**,守 [0000] 轻包体 / [0011] 纯 Rust。
//! `highlight(code, lang)` 接口与 research 一致 → 日后要更高精度,换引擎即可、上层不动。
//!
//! 覆盖:行/块注释、字符串(含转义、跨行)、数字、标识符(关键字/类型/函数/普通)、标点。按语言切关键字
//! 表与注释/字符串风格;未知语言走通用 C 系默认(注释 `//`+`/* */`、字符串 `"'` `、常见关键字并集弱命中)。

/// 代码语义类(8 类塌缩;上层映射 `StyleRole::Code*`)。`Plain` = 默认代码字(= 现 `CodeBlock` 观感)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CodeClass {
    Plain,
    Keyword,
    Type,
    Func,
    String,
    Comment,
    Number,
    Punct,
}

/// 一种语言的词法规格(关键字/类型/注释/字符串风格)。`'static` 表,编译期内嵌,零运行时分配。
struct LangSpec {
    keywords: &'static [&'static str],
    types: &'static [&'static str],
    /// 行注释前缀(任一命中 → 到行尾为注释)。
    line_comment: &'static [&'static str],
    /// 块注释 (开, 闭);None = 无。
    block_comment: Option<(&'static str, &'static str)>,
    /// 字符串定界符(任一;以 `\` 转义)。
    strings: &'static [char],
    /// 标识符大写开头是否视作类型(CamelCase 语言:rust/go/java/ts…)。
    cap_is_type: bool,
}

const C_LINE: &[&str] = &["//"];
const HASH_LINE: &[&str] = &["#"];
const C_BLOCK: Option<(&str, &str)> = Some(("/*", "*/"));
const DQ_SQ: &[char] = &['"', '\''];
const DQ_SQ_BT: &[char] = &['"', '\'', '`'];

const RUST_KW: &[&str] = &[
    "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum", "extern",
    "false", "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub",
    "ref", "return", "self", "Self", "static", "struct", "super", "trait", "true", "type",
    "unsafe", "use", "where", "while",
];
const RUST_TY: &[&str] = &[
    "u8", "u16", "u32", "u64", "u128", "usize", "i8", "i16", "i32", "i64", "i128", "isize", "f32",
    "f64", "bool", "char", "str", "String", "Vec", "Option", "Result", "Box",
];

const PY_KW: &[&str] = &[
    "and", "as", "assert", "async", "await", "break", "class", "continue", "def", "del", "elif",
    "else", "except", "False", "finally", "for", "from", "global", "if", "import", "in", "is",
    "lambda", "None", "nonlocal", "not", "or", "pass", "raise", "return", "True", "try", "while",
    "with", "yield",
];
const PY_TY: &[&str] = &[
    "int", "float", "str", "bool", "bytes", "list", "dict", "set", "tuple", "object",
];

const JS_KW: &[&str] = &[
    "async",
    "await",
    "break",
    "case",
    "catch",
    "class",
    "const",
    "continue",
    "debugger",
    "default",
    "delete",
    "do",
    "else",
    "export",
    "extends",
    "false",
    "finally",
    "for",
    "from",
    "function",
    "if",
    "import",
    "in",
    "instanceof",
    "let",
    "new",
    "null",
    "of",
    "return",
    "super",
    "switch",
    "this",
    "throw",
    "true",
    "try",
    "typeof",
    "var",
    "void",
    "while",
    "yield",
    "interface",
    "type",
    "enum",
    "implements",
    "readonly",
];
const JS_TY: &[&str] = &[
    "number", "string", "boolean", "any", "void", "object", "unknown", "never", "Promise", "Array",
];

const GO_KW: &[&str] = &[
    "break",
    "case",
    "chan",
    "const",
    "continue",
    "default",
    "defer",
    "else",
    "fallthrough",
    "for",
    "func",
    "go",
    "goto",
    "if",
    "import",
    "interface",
    "map",
    "package",
    "range",
    "return",
    "select",
    "struct",
    "switch",
    "type",
    "var",
    "nil",
    "true",
    "false",
];
const GO_TY: &[&str] = &[
    "int", "int8", "int16", "int32", "int64", "uint", "uint8", "uint16", "uint32", "uint64",
    "float32", "float64", "bool", "string", "byte", "rune", "error",
];

const C_KW: &[&str] = &[
    "auto",
    "break",
    "case",
    "char",
    "const",
    "continue",
    "default",
    "do",
    "double",
    "else",
    "enum",
    "extern",
    "float",
    "for",
    "goto",
    "if",
    "int",
    "long",
    "register",
    "return",
    "short",
    "signed",
    "sizeof",
    "static",
    "struct",
    "switch",
    "typedef",
    "union",
    "unsigned",
    "void",
    "volatile",
    "while",
    "class",
    "namespace",
    "template",
    "public",
    "private",
    "protected",
    "new",
    "delete",
    "true",
    "false",
    "nullptr",
    "using",
];

const SH_KW: &[&str] = &[
    "if", "then", "else", "elif", "fi", "for", "in", "do", "done", "while", "until", "case",
    "esac", "function", "return", "local", "export", "source", "echo", "set", "cd",
];

const JSON_KW: &[&str] = &["true", "false", "null"];

const EMPTY: &[&str] = &[];

/// 围栏语言串归一 + 取规格。别名:py→python、js/ts/jsx/tsx→js、c/cpp/h→c、sh→bash、rs→rust…。
fn lang_spec(lang: Option<&str>) -> LangSpec {
    let l = lang.unwrap_or("").trim().to_ascii_lowercase();
    match l.as_str() {
        "rust" | "rs" => LangSpec {
            keywords: RUST_KW,
            types: RUST_TY,
            line_comment: C_LINE,
            block_comment: C_BLOCK,
            strings: DQ_SQ,
            cap_is_type: true,
        },
        "python" | "py" => LangSpec {
            keywords: PY_KW,
            types: PY_TY,
            line_comment: HASH_LINE,
            block_comment: None,
            strings: DQ_SQ,
            cap_is_type: true,
        },
        "javascript" | "js" | "jsx" | "typescript" | "ts" | "tsx" => LangSpec {
            keywords: JS_KW,
            types: JS_TY,
            line_comment: C_LINE,
            block_comment: C_BLOCK,
            strings: DQ_SQ_BT,
            cap_is_type: true,
        },
        "go" | "golang" => LangSpec {
            keywords: GO_KW,
            types: GO_TY,
            line_comment: C_LINE,
            block_comment: C_BLOCK,
            strings: DQ_SQ_BT,
            cap_is_type: true,
        },
        "c" | "cpp" | "c++" | "h" | "hpp" | "java" => LangSpec {
            keywords: C_KW,
            types: EMPTY,
            line_comment: C_LINE,
            block_comment: C_BLOCK,
            strings: DQ_SQ,
            cap_is_type: true,
        },
        "bash" | "sh" | "shell" | "zsh" => LangSpec {
            keywords: SH_KW,
            types: EMPTY,
            line_comment: HASH_LINE,
            block_comment: None,
            strings: DQ_SQ_BT,
            cap_is_type: false,
        },
        "json" => LangSpec {
            keywords: JSON_KW,
            types: EMPTY,
            line_comment: EMPTY,
            block_comment: None,
            strings: &['"'],
            cap_is_type: false,
        },
        // 未知/纯文本:通用 C 系(注释 + 字符串 + 数字,关键字空 → 多为 Plain)。
        _ => LangSpec {
            keywords: EMPTY,
            types: EMPTY,
            line_comment: C_LINE,
            block_comment: C_BLOCK,
            strings: DQ_SQ,
            cap_is_type: false,
        },
    }
}

fn is_ident_start(c: char) -> bool {
    c.is_alphabetic() || c == '_'
}
fn is_ident(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// 高亮 `code`(按 `lang`)→ **每字符**一个 [`CodeClass`](`Vec` 长度 = `code.chars().count()`)。
/// 上层按位映射到 `Code*` 角色。单遍 O(n)、无回溯、确定性(同输入同输出)。
pub(crate) fn highlight(code: &str, lang: Option<&str>) -> Vec<CodeClass> {
    let spec = lang_spec(lang);
    let chars: Vec<char> = code.chars().collect();
    let n = chars.len();
    let mut out = vec![CodeClass::Plain; n];
    let starts_with = |i: usize, p: &str| -> bool {
        let pc: Vec<char> = p.chars().collect();
        i + pc.len() <= n && chars[i..i + pc.len()] == pc[..]
    };
    let mut i = 0usize;
    while i < n {
        let c = chars[i];
        // 块注释。
        if let Some((open, close)) = spec.block_comment {
            if starts_with(i, open) {
                let mut j = i + open.chars().count();
                while j < n && !starts_with(j, close) {
                    j += 1;
                }
                let end = (j + close.chars().count()).min(n);
                for cls in out.iter_mut().take(end).skip(i) {
                    *cls = CodeClass::Comment;
                }
                i = end;
                continue;
            }
        }
        // 行注释。
        if spec.line_comment.iter().any(|p| starts_with(i, p)) {
            let mut j = i;
            while j < n && chars[j] != '\n' {
                j += 1;
            }
            for cls in out.iter_mut().take(j).skip(i) {
                *cls = CodeClass::Comment;
            }
            i = j;
            continue;
        }
        // 字符串(以 `\` 转义;跨行直到闭引号或 EOF)。
        if spec.strings.contains(&c) {
            let q = c;
            let mut j = i + 1;
            while j < n {
                if chars[j] == '\\' {
                    j += 2;
                    continue;
                }
                if chars[j] == q {
                    j += 1;
                    break;
                }
                j += 1;
            }
            let end = j.min(n);
            for cls in out.iter_mut().take(end).skip(i) {
                *cls = CodeClass::String;
            }
            i = end;
            continue;
        }
        // 数字(十进制/十六进制/小数/下划线分隔/后缀)。
        if c.is_ascii_digit() {
            let mut j = i + 1;
            while j < n && (chars[j].is_alphanumeric() || chars[j] == '.' || chars[j] == '_') {
                j += 1;
            }
            for cls in out.iter_mut().take(j).skip(i) {
                *cls = CodeClass::Number;
            }
            i = j;
            continue;
        }
        // 标识符 → 关键字 / 类型 / 函数 / 普通。
        if is_ident_start(c) {
            let mut j = i + 1;
            while j < n && is_ident(chars[j]) {
                j += 1;
            }
            let word: String = chars[i..j].iter().collect();
            let class = if spec.keywords.contains(&word.as_str()) {
                CodeClass::Keyword
            } else if spec.types.contains(&word.as_str()) || (spec.cap_is_type && c.is_uppercase())
            {
                CodeClass::Type
            } else {
                // 函数:后随(跳空白)`(` → Func。
                let mut k = j;
                while k < n && (chars[k] == ' ' || chars[k] == '\t') {
                    k += 1;
                }
                if k < n && chars[k] == '(' {
                    CodeClass::Func
                } else {
                    CodeClass::Plain
                }
            };
            for cls in out.iter_mut().take(j).skip(i) {
                *cls = class;
            }
            i = j;
            continue;
        }
        // 标点 / 运算符(非空白、非字母数字)。
        if !c.is_whitespace() {
            out[i] = CodeClass::Punct;
        }
        i += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::CodeClass::{Comment, Func, Keyword, Number, Plain, Punct, String as Str, Type};
    use super::*;

    fn classes(code: &str, lang: &str) -> Vec<CodeClass> {
        highlight(code, Some(lang))
    }

    /// 取某子串首字符的类(便于断言)。
    fn class_at(code: &str, lang: &str, needle: &str) -> CodeClass {
        let idx = code.find(needle).expect("needle in code");
        let char_idx = code[..idx].chars().count();
        classes(code, lang)[char_idx]
    }

    #[test]
    fn rust_keyword_type_func_number() {
        let code = "fn add(x: u32) -> u32 { 42 }";
        assert_eq!(class_at(code, "rust", "fn"), Keyword);
        assert_eq!(class_at(code, "rust", "add"), Func, "add( → 函数");
        assert_eq!(class_at(code, "rust", "u32"), Type);
        assert_eq!(class_at(code, "rust", "42"), Number);
        assert_eq!(class_at(code, "rust", "("), Punct);
        assert_eq!(class_at(code, "rust", "x"), Plain);
    }

    #[test]
    fn strings_and_line_comment() {
        let code = "let s = \"hi\"; // tail";
        assert_eq!(class_at(code, "rust", "\"hi\""), Str);
        assert_eq!(class_at(code, "rust", "// tail"), Comment);
        // 注释吃到行尾。
        let cs = classes(code, "rust");
        assert!(cs.iter().rev().take(4).all(|&c| c == Comment), "行尾皆注释");
    }

    #[test]
    fn block_comment_spans_lines() {
        let code = "a /* multi\nline */ b";
        assert_eq!(class_at(code, "c", "/* multi"), Comment);
        assert_eq!(class_at(code, "c", "line */"), Comment);
        assert_eq!(class_at(code, "c", "b"), Plain, "块注释后恢复");
    }

    #[test]
    fn python_hash_comment_and_keyword() {
        let code = "def f():  # note\n    return None";
        assert_eq!(class_at(code, "python", "def"), Keyword);
        assert_eq!(class_at(code, "python", "# note"), Comment);
        assert_eq!(class_at(code, "python", "return"), Keyword);
        assert_eq!(class_at(code, "python", "None"), Keyword);
        // `#` 不是 rust 注释 → 在 rust 下不是 Comment。
        assert_ne!(class_at("x # y", "rust", "# y"), Comment);
    }

    #[test]
    fn unknown_lang_still_does_strings_numbers_comments() {
        let code = "val = 3.14 // c\n\"q\"";
        assert_eq!(class_at(code, "weirdlang", "3.14"), Number);
        assert_eq!(class_at(code, "weirdlang", "// c"), Comment);
        assert_eq!(class_at(code, "weirdlang", "\"q\""), Str);
        assert_eq!(
            class_at(code, "weirdlang", "val"),
            Plain,
            "未知语言无关键字"
        );
    }

    #[test]
    fn output_length_matches_char_count_with_cjk() {
        let code = "let 名字 = \"值\"; // 注释";
        let cs = highlight(code, Some("rust"));
        assert_eq!(cs.len(), code.chars().count(), "每字符一类(含 CJK)");
    }
}
