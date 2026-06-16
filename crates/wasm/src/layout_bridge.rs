//! layout_bridge(M7)— `LayoutEngine` 的 wasm 实现:调 JS 侧排版(measureText + 折行)。
//!
//! 每帧最多一次批调用(AR10):整段可见文本一次过界。传**带角色的 run**(runTexts/runRoles,
//! 4A4 按角色度量 + 标题分级)+ **表格结构**(`tables`,0014 B/plan5 §5F:run 区间 + 列对齐,
//! 供像素两趟对齐 + 格内折行);JS 返回平铺 `Float32Array` `[x,y,w,h]*N`(每 grapheme 一组,
//! CR4 零拷贝)。glyph 顺序须与输入 grapheme 严格 1:1。

use infinite_chat_core::{
    LayoutEngine, LayoutResult, PlacedGlyph, StyledSpan, TablePanel, TableRegion,
};
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

        // 返回:`Float32Array`(仅位置)或 `{ positions: Float32Array, tables: Float32Array }`
        //(带各表格面板几何,0018 #5)。两形态都接,后者多取 `tables`。
        let Ok(ret) = ret else {
            tracing::warn!(target: "M7", "layout 调用失败");
            return LayoutResult::default();
        };
        let (positions, table_panels) = parse_layout_ret(&ret);
        let Some(positions) = positions else {
            tracing::warn!(target: "M7", "layout 返回缺 positions");
            return LayoutResult::default();
        };
        let arr = positions.to_vec();

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
            table_panels,
        }
    }
}

/// 解析 layout 返回:`Float32Array` → (positions, 无表);否则取对象 `positions` + 扁平 `tables`。
/// `tables` 编码(每表连续):`[x, y, w, h, header_bottom, n_cols, n_rows, cols…, rows…]`(0018 #5)。
fn parse_layout_ret(ret: &JsValue) -> (Option<Float32Array>, Vec<TablePanel>) {
    if let Ok(fa) = ret.clone().dyn_into::<Float32Array>() {
        return (Some(fa), Vec::new());
    }
    let positions = Reflect::get(ret, &JsValue::from_str("positions"))
        .ok()
        .and_then(|p| p.dyn_into::<Float32Array>().ok());
    let tables = Reflect::get(ret, &JsValue::from_str("tables"))
        .ok()
        .and_then(|t| t.dyn_into::<Float32Array>().ok())
        .map(|t| decode_table_panels(&t.to_vec()))
        .unwrap_or_default();
    (positions, tables)
}

/// 解码扁平表格面板编码(见 [`parse_layout_ret`])→ `Vec<TablePanel>`。越界/不足即停(稳健)。
fn decode_table_panels(flat: &[f32]) -> Vec<TablePanel> {
    let mut out = Vec::new();
    let mut i = 0usize;
    while i + 7 <= flat.len() {
        let x = flat[i];
        let y = flat[i + 1];
        let w = flat[i + 2];
        let h = flat[i + 3];
        let header_bottom = flat[i + 4];
        let n_cols = flat[i + 5].max(0.0) as usize;
        let n_rows = flat[i + 6].max(0.0) as usize;
        i += 7;
        if i + n_cols + n_rows > flat.len() {
            break; // 数据不足 → 丢弃残块
        }
        let cols = flat[i..i + n_cols].to_vec();
        i += n_cols;
        let rows = flat[i..i + n_rows].to_vec();
        i += n_rows;
        out.push(TablePanel {
            x,
            y,
            w,
            h,
            header_bottom,
            cols,
            rows,
        });
    }
    out
}
