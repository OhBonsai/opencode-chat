//! layout_bridge(M7)— `LayoutEngine` 的 wasm 实现:调 JS 侧排版(measureText + 折行)。
//!
//! 每帧最多一次批调用(AR10):整段可见文本一次过界。传**带角色的 run**(runTexts/runRoles,
//! 4A4 按角色度量 + 标题分级)+ **表格结构**(`tables`,0014 B/plan5 §5F:run 区间 + 列对齐,
//! 供像素两趟对齐 + 格内折行);JS 返回平铺 `Float32Array` `[x,y,w,h]*N`(每 grapheme 一组,
//! CR4 零拷贝)。glyph 顺序须与输入 grapheme 严格 1:1。

use infinite_chat_core::{LayoutEngine, LayoutResult, PlacedGlyph, StyledSpan, TableRegion};
use js_sys::{Array, Float32Array, Object, Reflect, Uint32Array};
use wasm_bindgen::{JsCast, JsValue};

pub(crate) struct LayoutBridge {
    /// JS:`(runTexts: string[], runRoles: Uint32Array, maxWidth: number, tables?) => Float32Array`。
    layout_fn: js_sys::Function,
}

impl LayoutBridge {
    pub(crate) fn new(layout_fn: js_sys::Function) -> Self {
        Self { layout_fn }
    }
}

/// 表格结构 → JS:`Array<{ rows: Array<Array<[startRun, endRun]>>, aligns: number[] }>`(plan5 §5F)。
fn tables_to_js(tables: &[TableRegion]) -> Array {
    let out = Array::new();
    for t in tables {
        let obj = Object::new();
        let rows = Array::new();
        for row in &t.rows {
            let row_js = Array::new();
            for &(s, e) in row {
                let cell = Array::new();
                cell.push(&JsValue::from_f64(f64::from(s)));
                cell.push(&JsValue::from_f64(f64::from(e)));
                row_js.push(&cell);
            }
            rows.push(&row_js);
        }
        let _ = Reflect::set(&obj, &JsValue::from_str("rows"), &rows);
        let aligns = Array::new();
        for &a in &t.aligns {
            aligns.push(&JsValue::from_f64(f64::from(a)));
        }
        let _ = Reflect::set(&obj, &JsValue::from_str("aligns"), &aligns);
        out.push(&obj);
    }
    out
}

impl LayoutEngine for LayoutBridge {
    fn layout(
        &mut self,
        spans: &[StyledSpan],
        tables: &[TableRegion],
        max_width: f32,
    ) -> LayoutResult {
        if spans.is_empty() {
            return LayoutResult::default();
        }
        // 构建带角色的 run(text[] + role[]);grapheme 顺序与 app 侧 StyledSpan 一致。
        let texts = Array::new();
        let mut roles_vec = Vec::with_capacity(spans.len());
        for s in spans {
            texts.push(&JsValue::from_str(s.text()));
            roles_vec.push(s.role().as_u32());
        }
        let roles = Uint32Array::new_with_length(roles_vec.len() as u32);
        roles.copy_from(&roles_vec);

        // 4 参(call3 上限 3,改 apply):texts, roles, maxWidth, tables(0014 B)。
        let args = Array::new();
        args.push(&texts);
        args.push(&roles);
        args.push(&JsValue::from_f64(f64::from(max_width)));
        args.push(&tables_to_js(tables));
        let ret = self.layout_fn.apply(&JsValue::NULL, &args);

        let Ok(typed) = ret.and_then(|v| v.dyn_into::<Float32Array>().map_err(|_| JsValue::NULL))
        else {
            tracing::warn!(target: "M7", "layout 返回非 Float32Array");
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
