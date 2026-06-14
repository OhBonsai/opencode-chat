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
    @location(6) kind: u32,            // 字形源:0=位图覆盖率 1=TinySDF 2=MSDF 3=RGBA
};

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) alpha: f32,
    @location(2) tint: vec3<f32>,
    @location(3) @interpolate(flat) layer: u32,
    @location(4) @interpolate(flat) kind: u32,
};

fn style_color(s: u32) -> vec3<f32> {
    switch s {
        case 1u: { return vec3<f32>(1.0, 1.0, 1.0); }        // Bold
        case 2u: { return vec3<f32>(0.85, 0.85, 0.90); }     // Italic
        case 3u: { return vec3<f32>(1.0, 1.0, 1.0); }        // BoldItalic
        case 4u, 5u: { return vec3<f32>(0.60, 0.85, 0.70); } // Code / CodeBlock
        case 6u, 10u, 11u, 12u, 13u, 14u: { return vec3<f32>(0.55, 0.78, 1.0); } // Heading H1–H6
        case 7u: { return vec3<f32>(0.45, 0.70, 1.0); }      // Link
        case 8u, 9u: { return vec3<f32>(0.62, 0.62, 0.68); } // Quote / ListMarker
        case 16u: { return vec3<f32>(0.78, 0.82, 0.90); }    // AlertLabel(类型色靠左条;文字取亮中性)
        default: { return vec3<f32>(0.90, 0.90, 0.92); }     // Normal / Rule(零墨)
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
    out.kind = inst.kind;
    return out;
}

// 单通道距离场覆盖率(TinySDF):smoothstep + fwidth 屏幕空间梯度 → 任意缩放锐利。
// ③ 阈值下移到 0.46:浅字深底会"视觉变细",下移加粗找回字重。
// ② AA 带收窄到 0.6×fwidth:整 fwidth 当半宽约 2px 过渡偏软,收窄更锐。
fn sdf_coverage(d: f32) -> f32 {
    let aa = max(fwidth(d), 0.0001);
    return smoothstep(0.46 - 0.6 * aa, 0.46 + 0.6 * aa, d);
}

fn median3(c: vec3<f32>) -> f32 {
    return max(min(c.r, c.g), min(max(c.r, c.g), c.b));
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let tex = textureSample(atlas_tex, atlas_samp, in.uv, i32(in.layer));
    var cov: f32;
    switch in.kind {
        // 0=位图覆盖率:R8 存的就是 alpha 覆盖率,直采(1× 锐、不缩放)。
        case 0u: { cov = tex.r; }
        // 2=MSDF:median(r,g,b) 还原距离场再 smoothstep(拐角锐;baked RGB 页)。
        case 2u: { cov = sdf_coverage(median3(tex.rgb)); }
        // 3=RGBA 彩字(emoji):直接输出,不上色。
        case 3u: { return tex * in.alpha; }
        // 1=TinySDF(默认):单通道距离场。
        default: { cov = sdf_coverage(tex.r); }
    }
    return vec4<f32>(in.tint, cov * in.alpha);
}
