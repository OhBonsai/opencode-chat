// markdown/widget.wgsl — markdown 语义组件 pipeline 入口(0026/Plan 11 §2)。
//
// 一条 pipeline 画所有 markdown 组件,fragment 按实例 component id 分派到对应组件函数
// (box=0,后续 slider=1 …)。组件函数在 markdown/<name>.wgsl,base/sdf.wgsl 提供共享形状。
// backend.rs 拼接顺序:base/sdf.wgsl → markdown/box.wgsl → 本文件(声明先于使用)。
// 与 rect/glyph 共用 Globals(同相机/视口);无 atlas、无 storage(WebGL2 友好)。

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
    @location(0) pos: vec2<f32>,      // 左上角世界 px
    @location(1) size: vec2<f32>,     // 宽高 px
    @location(2) color: vec4<f32>,    // 组件主色 RGBA
    @location(3) params: vec4<f32>,   // 组件参数(box:radius,stroke,check,_)
    @location(4) component: u32,      // 组件 id:0=box
};

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) local: vec2<f32>,                  // 框中心为原点的世界 px
    @location(1) @interpolate(flat) halfsz: vec2<f32>,
    @location(2) @interpolate(flat) color: vec4<f32>,
    @location(3) @interpolate(flat) params: vec4<f32>,
    @location(4) @interpolate(flat) component: u32,
};

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
    out.params = inst.params;
    out.component = inst.component;
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    // ⚠ WebGPU 禁止在非均匀控制流里求导数(fwidth):组件选择子 in.component 是 per-instance flat
    // = 非均匀,故**不能**用 `switch in.component { md_x() }` 包裹会用 fwidth 的组件(否则 Chrome
    // 黑屏)。正确做法:**无条件**求每个组件结果(各自 fwidth 在 top-level、均匀),再 `select` 选
    // (select 是表达式、非控制流)。多算几个组件每片元成本极低。
    let c_box = md_box(in.local, in.halfsz, in.color, in.params);
    let c_rule = md_rule(in.local, in.halfsz, in.color, in.params);
    let c_cat = md_rule_cat(in.local, in.halfsz, in.color, globals.time_ms);
    var o = select(c_box, c_rule, in.component == 1u);
    o = select(o, c_cat, in.component == 2u);
    return o;
}
