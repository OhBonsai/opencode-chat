const T = [["KaTeX_Main", "KaTeX_Main-Regular", "normal", "normal"], ["KaTeX_Main", "KaTeX_Main-Bold", "bold", "normal"], ["KaTeX_Main", "KaTeX_Main-Italic", "normal", "italic"], ["KaTeX_Main", "KaTeX_Main-BoldItalic", "bold", "italic"], ["KaTeX_Math", "KaTeX_Math-Italic", "normal", "italic"], ["KaTeX_Math", "KaTeX_Math-BoldItalic", "bold", "italic"], ["KaTeX_AMS", "KaTeX_AMS-Regular", "normal", "normal"], ["KaTeX_Size1", "KaTeX_Size1-Regular", "normal", "normal"], ["KaTeX_Size2", "KaTeX_Size2-Regular", "normal", "normal"], ["KaTeX_Size3", "KaTeX_Size3-Regular", "normal", "normal"], ["KaTeX_Size4", "KaTeX_Size4-Regular", "normal", "normal"], ["KaTeX_Caligraphic", "KaTeX_Caligraphic-Regular", "normal", "normal"], ["KaTeX_Fraktur", "KaTeX_Fraktur-Regular", "normal", "normal"], ["KaTeX_SansSerif", "KaTeX_SansSerif-Regular", "normal", "normal"], ["KaTeX_Script", "KaTeX_Script-Regular", "normal", "normal"], ["KaTeX_Typewriter", "KaTeX_Typewriter-Regular", "normal", "normal"]];
let e = null;
function K(n = "/infinite-chat/fonts/katex/") {
  return e || (e = (async () => {
    const r = document.fonts;
    r && await Promise.all(T.map(async ([o, l, t, i]) => {
      try {
        const a = new FontFace(o, `url(${n}${l}.woff2) format("woff2")`, { weight: t, style: i });
        await a.load(), r.add(a);
      } catch (a) {
        console.warn(`[math-fonts] ${l} \u52A0\u8F7D\u5931\u8D25`, a);
      }
    }));
  })(), e);
}
export {
  K as loadMathFonts
};
