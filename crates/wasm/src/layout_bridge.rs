//! layout_bridge(M7)— `LayoutEngine` 的 wasm 实现:调 JS 侧 pretext 排版。
//!
//! 每帧最多一次批调用(AR10):整段可见文本一次过界,JS 返回平铺 `Float32Array`
//! `[x,y,w,h]*N`(每 grapheme 一组,CR4 零拷贝)。glyph 顺序须与输入 grapheme 严格 1:1。

use js_sys::Float32Array;
use opencode_chat_core::{LayoutEngine, LayoutResult, PlacedGlyph, StyledSpan};
use wasm_bindgen::{JsCast, JsValue};

pub(crate) struct PretextLayout {
    /// JS:`(text: string, maxWidth: number) => Float32Array`。
    layout_fn: js_sys::Function,
}

impl PretextLayout {
    pub(crate) fn new(layout_fn: js_sys::Function) -> Self {
        Self { layout_fn }
    }
}

impl LayoutEngine for PretextLayout {
    fn layout(&mut self, spans: &[StyledSpan], max_width: f32) -> LayoutResult {
        let text: String = spans.iter().map(StyledSpan::text).collect();
        if text.is_empty() {
            return LayoutResult::default();
        }
        let ret = self.layout_fn.call2(
            &JsValue::NULL,
            &JsValue::from_str(&text),
            &JsValue::from_f64(f64::from(max_width)),
        );
        let Ok(typed) = ret.and_then(|v| v.dyn_into::<Float32Array>().map_err(|_| JsValue::NULL))
        else {
            tracing::warn!(target: "M7", "pretext layout 返回非 Float32Array");
            return LayoutResult::default();
        };
        let arr = typed.to_vec();

        let mut glyphs = Vec::with_capacity(arr.len() / 4);
        let mut block_height = 0.0f32;
        for c in arr.chunks_exact(4) {
            glyphs.push(PlacedGlyph {
                pos: [c[0], c[1]],
                size: [c[2], c[3]],
            });
            block_height = block_height.max(c[1] + c[3]);
        }
        LayoutResult {
            glyphs,
            block_height,
        }
    }
}
