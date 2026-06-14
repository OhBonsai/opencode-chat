// glyph.wgsl — SDF 文字图元(Plan 3 K)。
//
// atlas tile 是 R8 单通道**距离场**(0.5 = 字形边缘)。覆盖率用 smoothstep + 屏幕导数
// fwidth 抗锯齿 → **任意缩放清晰**(无边画布硬需求)。保留 spawn_time GPU 淡入、StyleRole
// 上色。富特效(发光/描边/溶解,0007)在此片元层加几行即可(本期不做)。

struct Globals {
    viewport: vec2<f32>,   // 画布像素尺寸
    time_ms: f32,          // 当前帧时间
    fade_ms: f32,          // 淡入时长;0 = 不淡入
    cam_pan: vec2<f32>,    // 相机:屏幕左上角对应的世界坐标
    cam_zoom: f32,         // 相机缩放
    _pad: f32,
};

@group(0) @binding(0) var<uniform> globals: Globals;
@group(0) @binding(1) var atlas_tex: texture_2d_array<f32>;
@group(0) @binding(2) var atlas_samp: sampler;

struct InstanceIn {
    @location(0) pos: vec2<f32>,       // 左上角世界 px
    @location(1) size: vec2<f32>,      // 宽高 px
    @location(2) uv: vec4<f32>,        // tile UV: u0,v0,u1,v1
    @location(3) spawn_time: f32,
    @location(4) style: u32,           // StyleRole
    @location(5) layer: u32,           // atlas 页(纹理数组层)
};

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) alpha: f32,
    @location(2) tint: vec3<f32>,
    @location(3) @interpolate(flat) layer: u32,
};

fn style_color(s: u32) -> vec3<f32> {
    switch s {
        case 1u: { return vec3<f32>(1.0, 1.0, 1.0); }        // Bold
        case 2u: { return vec3<f32>(0.85, 0.85, 0.90); }     // Italic
        case 3u: { return vec3<f32>(1.0, 1.0, 1.0); }        // BoldItalic
        case 4u, 5u: { return vec3<f32>(0.60, 0.85, 0.70); } // Code / CodeBlock
        case 6u: { return vec3<f32>(0.55, 0.78, 1.0); }      // Heading
        case 7u: { return vec3<f32>(0.45, 0.70, 1.0); }      // Link
        case 8u, 9u: { return vec3<f32>(0.62, 0.62, 0.68); } // Quote / ListMarker
        default: { return vec3<f32>(0.90, 0.90, 0.92); }     // Normal
    }
}

@vertex
fn vs_main(@builtin(vertex_index) vid: u32, inst: InstanceIn) -> VsOut {
    var corners = array<vec2<f32>, 4>(
        vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 0.0),
        vec2<f32>(0.0, 1.0), vec2<f32>(1.0, 1.0),
    );
    let c = corners[vid];
    let world = inst.pos + c * inst.size;
    // 世界坐标 → 相机 → 屏幕 px → NDC(Plan 3 L)。
    let screen = (world - globals.cam_pan) * globals.cam_zoom;
    let ndc = vec2<f32>(
        screen.x / globals.viewport.x * 2.0 - 1.0,
        1.0 - screen.y / globals.viewport.y * 2.0,
    );
    var out: VsOut;
    out.clip = vec4<f32>(ndc, 0.0, 1.0);
    out.uv = vec2<f32>(mix(inst.uv.x, inst.uv.z, c.x), mix(inst.uv.y, inst.uv.w, c.y));
    let age = globals.time_ms - inst.spawn_time;
    out.alpha = select(clamp(age / max(globals.fade_ms, 1.0), 0.0, 1.0), 1.0, globals.fade_ms <= 0.0);
    out.tint = style_color(inst.style);
    out.layer = inst.layer;
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    // 距离场覆盖率:0.5 = 边缘;fwidth 给屏幕空间梯度 → 任意缩放锐利。
    let d = textureSample(atlas_tex, atlas_samp, in.uv, i32(in.layer)).r;
    let aa = max(fwidth(d), 0.0001);
    let cov = smoothstep(0.5 - aa, 0.5 + aa, d);
    return vec4<f32>(in.tint, cov * in.alpha);
}
