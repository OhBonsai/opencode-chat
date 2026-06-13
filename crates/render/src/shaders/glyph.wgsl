// glyph.wgsl — 实例化字形 + 着色器淡入(M9/M10)。
//
// 一个 instance = 一个 grapheme quad(triangle-strip 4 顶点)。淡入完全靠
// `time_ms - spawn_time` 在 GPU 算,CPU 零参与(0002 §5)。fade_ms=0 即关闭(参数置零,
// 非分支,满足 AR3 恒等收敛)。

struct Globals {
    viewport: vec2<f32>,   // 画布像素尺寸
    time_ms: f32,          // 当前帧时间
    fade_ms: f32,          // 淡入时长;0 = 不淡入
};

@group(0) @binding(0) var<uniform> globals: Globals;
@group(0) @binding(1) var atlas_tex: texture_2d<f32>;
@group(0) @binding(2) var atlas_samp: sampler;

struct InstanceIn {
    @location(0) pos: vec2<f32>,       // 左上角 world px
    @location(1) size: vec2<f32>,      // 字形宽高 px
    @location(2) uv: vec4<f32>,        // atlas: u0,v0,u1,v1
    @location(3) spawn_time: f32,
};

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) alpha: f32,
};

@vertex
fn vs_main(@builtin(vertex_index) vid: u32, inst: InstanceIn) -> VsOut {
    // strip 角点:0=(0,0) 1=(1,0) 2=(0,1) 3=(1,1)
    var corners = array<vec2<f32>, 4>(
        vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 0.0),
        vec2<f32>(0.0, 1.0), vec2<f32>(1.0, 1.0),
    );
    let c = corners[vid];
    let px = inst.pos + c * inst.size;
    // world px(左上原点)→ NDC(中心原点,y 向上)
    let ndc = vec2<f32>(
        px.x / globals.viewport.x * 2.0 - 1.0,
        1.0 - px.y / globals.viewport.y * 2.0,
    );
    var out: VsOut;
    out.clip = vec4<f32>(ndc, 0.0, 1.0);
    out.uv = vec2<f32>(mix(inst.uv.x, inst.uv.z, c.x), mix(inst.uv.y, inst.uv.w, c.y));
    let age = globals.time_ms - inst.spawn_time;
    // fade_ms<=0 时 alpha 直接为 1(max 保护除零)。
    out.alpha = select(clamp(age / max(globals.fade_ms, 1.0), 0.0, 1.0), 1.0, globals.fade_ms <= 0.0);
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let texel = textureSample(atlas_tex, atlas_samp, in.uv);
    // 字形位图:文字用白色描边、emoji 自带彩色;直接取 rgb,coverage 取 a。
    return vec4<f32>(texel.rgb, texel.a * in.alpha);
}
