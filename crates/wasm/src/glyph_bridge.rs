//! glyph_bridge(M8)— 调 JS 侧光栅器生成单个 grapheme 的 R8 tile(Plan 3 K / 0015)。
//!
//! JS 契约:`(cluster: string, style: number, kind: number) => Uint8Array`(长度 =
//! `TILE_PX²` 的 R8 单通道)。`kind`:0=位图覆盖率(alpha)/ 1=TinySDF(距离场,0.5≈边缘)。
//! 固定 tile 尺寸(源缩放无关)。

use js_sys::Uint8Array;
use wasm_bindgen::{JsCast, JsValue};

/// 调 JS 生成 R8 tile(按 `kind` 出覆盖率或距离场);失败/类型不符 → `None`(调用方跳过)。
pub(crate) fn rasterize(
    raster_fn: &js_sys::Function,
    cluster: &str,
    style: u32,
    kind: u32,
) -> Option<Vec<u8>> {
    let ret = raster_fn
        .call3(
            &JsValue::NULL,
            &JsValue::from_str(cluster),
            &JsValue::from_f64(f64::from(style)),
            &JsValue::from_f64(f64::from(kind)),
        )
        .ok()?;
    let arr = ret.dyn_into::<Uint8Array>().ok()?;
    Some(arr.to_vec())
}
