// panel.wgsl — 参数化 SDF 面板图元(Plan 6 / 0018)。rect.wgsl 的升级泛化:
// 一个 quad,fragment 按参数程序化画 圆角外框 + 横竖网格线 + 表头底 + AO + 底色。
// 变长参数(每列/行占比 + 颜色 + AO)放共享 storage buffer,实例携 [offset,len) 索引取。
// 文字不进本 shader(glyph 管线画字,本图元只画容器);二者共用同源 colX/rowY → 网格严丝合缝。
//
// 参数块(f32,自 param_offset 起):
//   [0..4]  fill rgba   [4..8] line rgba   [8..12] header rgba
//   [12] line_w(px)  [13] ao_strength  [14] header_ratio(0..1)  [15] n_cols  [16] n_rows
//   [17..20] ao_color rgb   [20] ao_width(px,AO 向内淡出距离)
//   [21 .. 21+n_cols]            col_ratios(0..1)
//   [21+n_cols .. +n_rows]       row_ratios(0..1)
// flags:bit0=grid,bit1=ao。线宽/线色/AO 色/AO 宽/AO 强度均为参数(暗色主题 AO 取白,做向内辉光)。

struct Globals {
    viewport: vec2<f32>,
    time_ms: f32,
    fade_ms: f32,
    cam_pan: vec2<f32>,
    cam_zoom: f32,
    _pad: f32,
};

@group(0) @binding(0) var<uniform> globals: Globals;
@group(0) @binding(1) var<storage, read> params: array<f32>;

struct InstanceIn {
    @location(0) pos: vec2<f32>,     // 左上角世界 px
    @location(1) size: vec2<f32>,    // 宽高 px
    @location(2) radius: f32,        // 圆角半径 px
    @location(3) param_offset: u32,  // 参数块在 storage buffer 起点(f32 下标)
    @location(4) param_len: u32,     // 参数块长度
    @location(5) flags: u32,         // bit0=grid bit1=ao
};

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) local: vec2<f32>,                  // 框中心为原点的世界 px
    @location(1) @interpolate(flat) halfsz: vec2<f32>,
    @location(2) @interpolate(flat) radius: f32,
    @location(3) @interpolate(flat) poff: u32,
    @location(4) @interpolate(flat) flags: u32,
};

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
    out.radius = min(inst.radius, min(out.halfsz.x, out.halfsz.y));
    out.poff = inst.param_offset;
    out.flags = inst.flags;
    return out;
}

fn p4(base: u32, i: u32) -> vec4<f32> {
    return vec4<f32>(params[base + i], params[base + i + 1u], params[base + i + 2u], params[base + i + 3u]);
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let base = in.poff;
    let fill = p4(base, 0u);
    let line = p4(base, 4u);
    let header = p4(base, 8u);
    let line_w = params[base + 12u];
    let ao_strength = params[base + 13u];
    let header_ratio = params[base + 14u];
    let n_cols = u32(params[base + 15u]);
    let n_rows = u32(params[base + 16u]);
    let ao_color = vec3<f32>(params[base + 17u], params[base + 18u], params[base + 19u]);
    let ao_width = params[base + 20u];

    // 外框覆盖率(圆角 SDF + fwidth AA)。
    let d = sd_round_box(in.local, in.halfsz, in.radius);
    let aa = max(fwidth(d), 0.0001);
    let inside = 1.0 - smoothstep(-aa, aa, d);
    if inside <= 0.0 {
        discard;
    }

    // 框内归一坐标(0..1,左上原点)。
    let uv = (in.local + in.halfsz) / max(in.halfsz * 2.0, vec2<f32>(1.0, 1.0));

    // 底色;表头区(uv.y < header_ratio)叠表头底。
    var col = fill;
    if header_ratio > 0.0 && uv.y < header_ratio {
        col = vec4<f32>(mix(col.rgb, header.rgb, header.a), max(col.a, header.a));
    }

    // 网格线:到最近竖/横线的世界距离 → smoothstep 成线(列从 storage 取占比)。
    if (in.flags & 1u) != 0u {
        let w = in.halfsz.x * 2.0;
        let h = in.halfsz.y * 2.0;
        var dmin = 1.0e9;
        for (var c = 0u; c < n_cols; c = c + 1u) {
            let lx = params[base + 21u + c] * w;
            dmin = min(dmin, abs(in.local.x + in.halfsz.x - lx));
        }
        for (var r = 0u; r < n_rows; r = r + 1u) {
            let ly = params[base + 21u + n_cols + r] * h;
            dmin = min(dmin, abs(in.local.y + in.halfsz.y - ly));
        }
        let g = 1.0 - smoothstep(0.0, max(line_w, 0.5), dmin);
        col = vec4<f32>(mix(col.rgb, line.rgb, g * line.a), max(col.a, g * line.a));
    }

    // AO:沿内边的辉光(暗色主题取白色 ao_color)。`-d` 在边=0、向内增大;ao_width 内淡出。
    // 用 mix 向 ao_color 靠 + 抬 alpha,使透明填充上也可见(旧版乘法压暗在透明/暗底上不可见)。
    if (in.flags & 2u) != 0u && ao_strength > 0.0 {
        let t = clamp(-d / max(ao_width, 1.0), 0.0, 1.0); // 0 在边、1 在内 ao_width 处
        let glow = (1.0 - t) * ao_strength;               // 边缘最亮、向内淡出
        col = vec4<f32>(mix(col.rgb, ao_color, glow), max(col.a, glow));
    }

    // 外框描边(圆角边一圈)。
    let stroke = max(line_w, 1.0);
    let ring = inside - (1.0 - smoothstep(-aa, aa, d + stroke));
    col = vec4<f32>(mix(col.rgb, line.rgb, clamp(ring, 0.0, 1.0) * line.a), max(col.a, clamp(ring, 0.0, 1.0) * line.a));

    return vec4<f32>(col.rgb, col.a * inside);
}
