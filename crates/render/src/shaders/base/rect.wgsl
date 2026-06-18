// rect.wgsl — 矩形/圆角图元(Plan 4B 装饰底/条 + 4C3 调试框)。
//
// 与 glyph.wgsl 共用 Globals(同相机/视口),独立管线(无 atlas 采样)。圆角/描边用
// 圆角矩形 SDF + fwidth 抗锯齿,故任意缩放边缘锐利。文字**之前**绘制 → 作背景。

struct Globals {
    viewport: vec2<f32>,
    time_ms: f32,
    fade_ms: f32,
    cam_pan: vec2<f32>,
    cam_zoom: f32,
    _pad: f32,
};

@group(0) @binding(0) var<uniform> globals: Globals;

struct InstanceIn {
    @location(0) pos: vec2<f32>,    // 左上角世界 px
    @location(1) size: vec2<f32>,   // 宽高 px
    @location(2) color: vec4<f32>,  // RGBA
    @location(3) radius: f32,       // 圆角半径 px
    @location(4) stroke: f32,       // 描边宽 px;0 = 实心填充
};

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) local: vec2<f32>,                 // 以矩形中心为原点的世界 px
    @location(1) @interpolate(flat) halfsz: vec2<f32>,
    @location(2) @interpolate(flat) color: vec4<f32>,
    @location(3) @interpolate(flat) radius: f32,
    @location(4) @interpolate(flat) stroke: f32,
};

// 圆角矩形有符号距离场(p、b、内陷 r 均为世界 px)。
fn sd_round_box(p: vec2<f32>, b: vec2<f32>, r: f32) -> f32 {
    let q = abs(p) - b + vec2<f32>(r, r);
    return min(max(q.x, q.y), 0.0) + length(max(q, vec2<f32>(0.0, 0.0))) - r;
}

@vertex
fn vs_main(@builtin(vertex_index) vid: u32, inst: InstanceIn) -> VsOut {
    var corners = array<vec2<f32>, 4>(
        vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 0.0),
        vec2<f32>(0.0, 1.0), vec2<f32>(1.0, 1.0),
    );
    let c = corners[vid];
    let world = inst.pos + c * inst.size;
    let screen = (world - globals.cam_pan) * globals.cam_zoom;
    let ndc = vec2<f32>(
        screen.x / globals.viewport.x * 2.0 - 1.0,
        1.0 - screen.y / globals.viewport.y * 2.0,
    );
    var out: VsOut;
    out.clip = vec4<f32>(ndc, 0.0, 1.0);
    out.local = (c - vec2<f32>(0.5, 0.5)) * inst.size;
    out.halfsz = inst.size * 0.5;
    out.color = inst.color;
    out.radius = min(inst.radius, min(out.halfsz.x, out.halfsz.y));
    out.stroke = inst.stroke;
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let d = sd_round_box(in.local, in.halfsz, in.radius);
    let aa = max(fwidth(d), 0.0001);
    let inside = 1.0 - smoothstep(-aa, aa, d);
    // stroke>0:仅内边一圈(outer 减去内陷 stroke 的实心)→ 调试框/边。
    let inner = 1.0 - smoothstep(-aa, aa, d + in.stroke);
    let cov = select(inside, inside - inner, in.stroke > 0.0);
    return vec4<f32>(in.color.rgb, in.color.a * cov);
}
