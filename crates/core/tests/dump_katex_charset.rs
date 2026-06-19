//! 烘焙辅助(Plan 12 ④,非常规测试 → `#[ignore]`):收集 RaTeX 在多样公式上发出的
//! (KaTeX 字族 → 字符集),逐字体写 `scripts/katex/charset/<Base>.txt`(原始 UTF-8)+ `manifest.txt`
//! (字体基名列表)。供 `scripts/bake-katex-msdf.mjs` 只烘真用到的数学字形。
//! 运行:`cargo test -p infinite-chat-core --test dump_katex_charset -- --ignored`
use infinite_chat_core::{katex_font_base, layout_math};
use std::collections::{BTreeMap, BTreeSet};

const CORPUS: &[&str] = &[
    r"E=mc^2",
    r"a^2+b^2=c^2",
    r"\sqrt{2}",
    r"\sqrt{\frac{a^2+b^2}{c}}",
    r"\frac{1}{2}",
    r"\frac{n(n+1)}{2}",
    r"\frac{d}{dx}\sin x = \cos x",
    r"\sum_{i=1}^{n} i",
    r"\sum_{k=1}^n k",
    r"\prod_{i=1}^n i",
    r"\int_0^1 x^2\,dx = \tfrac13",
    r"\int_{-\infty}^{\infty} e^{-x^2}\,dx = \sqrt{\pi}",
    r"\lim_{x\to 0}\frac{\sin x}{x} = 1",
    r"e^{i\pi} + 1 = 0",
    r"\alpha\beta\gamma\delta\epsilon\zeta\eta\theta\iota\kappa\lambda\mu\nu\xi\pi\rho\sigma\tau\phi\chi\psi\omega",
    r"\Gamma\Delta\Theta\Lambda\Xi\Pi\Sigma\Phi\Psi\Omega",
    r"x \le y \ge z \ne w \approx v \equiv u \pm t \times s \div r \cdot q \to p \infty",
    r"\nabla\cdot\vec{F} = \rho",
    r"\partial_t u = \kappa\nabla^2 u",
    r"\hat{x}\bar{y}\tilde{z}\dot{w}",
    r"A \cup B \cap C \subseteq D \in E \notin F \forall \exists \neg \wedge \vee",
    r"\binom{n}{k}",
    r"\vec{v}\cdot\vec{w}",
    r"f'(x)=2x",
    r"0123456789!?.,;:",
    r"abcdefghijklmnopqrstuvwxyz",
    r"ABCDEFGHIJKLMNOPQRSTUVWXYZ",
    r"\left(\frac{a}{b}\right)",
    r"\left[\sum_{i}x_i\right]",
    r"\left\{x\right\}",
    r"|x|+\|y\|",
    r"\mathcal{LF}",
    r"\mathfrak{g}",
    r"\sin\cos\tan\log\ln\exp\max\min\gcd\deg",
];

#[test]
#[ignore = "烘焙辅助,显式运行:cargo test --test dump_katex_charset -- --ignored"]
fn dump() {
    let mut by_font: BTreeMap<&'static str, BTreeSet<char>> = BTreeMap::new();
    for tex in CORPUS {
        for display in [true, false] {
            for g in &layout_math(tex, display).glyphs {
                if let Some(base) = katex_font_base(g.role) {
                    by_font.entry(base).or_default().insert(g.ch);
                }
            }
        }
    }
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../scripts/katex/charset");
    std::fs::create_dir_all(dir).expect("mkdir");
    let mut manifest = String::new();
    for (base, set) in &by_font {
        let s: String = set.iter().collect();
        std::fs::write(format!("{dir}/{base}.txt"), &s).expect("write charset");
        manifest.push_str(base);
        manifest.push('\n');
        eprintln!("{base}: {} chars", set.len());
    }
    std::fs::write(format!("{dir}/manifest.txt"), &manifest).expect("write manifest");
    eprintln!("wrote {} font charsets → {dir}", by_font.len());
}
