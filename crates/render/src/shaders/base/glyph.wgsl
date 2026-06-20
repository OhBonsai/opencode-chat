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
@group(0) @binding(3) var msdf_tex: texture_2d_array<f32>;  // 离线烘焙 MSDF(0015,RGBA 页)

struct InstanceIn {
    @location(0) pos: vec2<f32>,       // 左上角世界 px
    @location(1) size: vec2<f32>,      // 宽高 px
    @location(2) uv: vec4<f32>,        // tile UV: u0,v0,u1,v1
    @location(3) spawn_time: f32,
    @location(4) style: u32,           // StyleRole
    @location(5) layer: u32,           // atlas 页(纹理数组层)
    @location(6) kind: u32,            // 字形源:0=位图覆盖率 1=TinySDF 2=MSDF 3=RGBA
    @location(7) anim: u32,            // 进场动画 profile id(0025/Plan10 §3b;core 据角色+reveal风格选)
    @location(8) alpha: f32,           // 静态 alpha 乘子(Plan 15:代码块行窗边缘淡入淡出;默认 1)
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
        case 17u: { return vec3<f32>(0.86, 0.88, 0.92); }    // TableCell(表体:中性)
        case 18u: { return vec3<f32>(0.97, 0.98, 1.0); }     // TableHeader(表头:略亮区分)
        case 19u: { return vec3<f32>(1.0, 1.0, 1.0); }       // TableStrong(表体强调:最亮)
        case 20u: { return vec3<f32>(0.86, 0.88, 0.92); }    // TableEm(表体斜体:同表体中性,靠斜体区分)
        case 21u: { return vec3<f32>(0.30, 0.33, 0.40); }    // TableSep(列分隔符:弱化,与网格 rect 同灰)
        case 24u: { return vec3<f32>(0.45, 0.70, 1.0); }     // FootnoteRef(脚注引用:Link 色小号)
        case 25u: { return vec3<f32>(0.55, 0.58, 0.65); }    // FootnoteDef(脚注定义标记:弱化)
        case 43u: { return vec3<f32>(0.42, 0.46, 0.55); }    // CodeLineNum(代码行号:弱化灰,Plan 15 ②)
        // 代码语法高亮 8 色塌缩(research 路 A);CodePlain = 复用 case 5(CodeBlock 绿)。
        case 44u: { return vec3<f32>(0.78, 0.57, 0.92); }    // CodeKeyword(紫)
        case 45u: { return vec3<f32>(0.40, 0.78, 0.95); }    // CodeType(青)
        case 46u: { return vec3<f32>(0.45, 0.70, 1.00); }    // CodeFunc(蓝)
        case 47u: { return vec3<f32>(0.62, 0.84, 0.52); }    // CodeString(绿黄)
        case 48u: { return vec3<f32>(0.45, 0.50, 0.58); }    // CodeComment(灰)
        case 49u: { return vec3<f32>(0.94, 0.66, 0.45); }    // CodeNumber(橙)
        case 50u: { return vec3<f32>(0.78, 0.82, 0.88); }    // CodePunct(浅灰白)
        default: { return vec3<f32>(0.90, 0.90, 0.92); }     // Normal / Rule(零墨)
    }
}

// ── SDF 节点动画(0025 / Plan 10 §3,相位 3:per-element profile by StyleRole)──
// 进场 = 缓动 alpha + 绕字心 scale-in(+translate 钩子),由 spawn_time + fade_ms 驱动;settled(e=1)零影响、
// 瞬显(fade<=0)跳过。**per-element**:按字的 StyleRole 查 profile —— 表头/标题(pop+回弹+稍慢)≠ 正文。
// 用现有 instance.style 区分 → 零 buffer 改动;若要按 reveal 上下文(整表/行框)再分 = 相位 3b(per-instance 字段)。
const ANIM_ENTER_SCALE: f32 = 0.1; // 正文进场起始缩放(<1 由小放大);1.0 关闭
const ANIM_ENTER_RISE: f32 = 0.0;  // 进场上浮 px(>0 由下而上);0 关闭

struct EnterProfile { scale_from: f32, rise: f32, dur_mul: f32, curve: u32 }

fn ease_out_cubic(t: f32) -> f32 { let u = 1.0 - t; return 1.0 - u * u * u; }
fn ease_out_back(t: f32) -> f32 { let c1 = 1.70158; let c3 = c1 + 1.0; let u = t - 1.0; return 1.0 + c3 * u * u * u + c1 * u * u; }
fn apply_curve(curve: u32, t: f32) -> f32 { if (curve == 1u) { return ease_out_back(t); } return ease_out_cubic(t); }

// 进场 profile 表(按 core 给的 id 查;id 与 app::enter_profile_id 对齐,3b 数据驱动)。
// 0=正文逐字;1=表头/标题 pop(0.4→1 回弹,1.5×);2=整表风格的表头(0.2→1 回弹,2× 更大更慢)。
fn enter_profile_by_id(id: u32) -> EnterProfile {
    if (id == 2u) { return EnterProfile(0.2, 0.0, 2.0, 1u); }
    if (id == 1u) { return EnterProfile(0.4, 0.0, 1.5, 1u); }
    return EnterProfile(ANIM_ENTER_SCALE, ANIM_ENTER_RISE, 1.0, 0u);
}

@vertex
fn vs_main(@builtin(vertex_index) vid: u32, inst: InstanceIn) -> VsOut {
    var corners = array<vec2<f32>, 4>(
        vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 0.0),
        vec2<f32>(0.0, 1.0), vec2<f32>(1.0, 1.0),
    );
    let c = corners[vid];
    // per-element 进场(按 core 给的 profile id 查表,3b);e = 缓动 (now-spawn)/(fade*dur_mul);fade<=0 → e=1 跳过。
    let prof = enter_profile_by_id(inst.anim);
    let age = globals.time_ms - inst.spawn_time;
    let dur = max(globals.fade_ms * prof.dur_mul, 1.0);
    let e = select(apply_curve(prof.curve, clamp(age / dur, 0.0, 1.0)), 1.0, globals.fade_ms <= 0.0);
    // 绕字心 scale-in(+上浮);settled(e=1)还原 inst.pos + c*size。回弹曲线 e 可略 >1 → scale 过冲再落定(pop)。
    let half = vec2<f32>(0.5, 0.5);
    let s = mix(prof.scale_from, 1.0, e);
    let world = inst.pos + inst.size * (half + (c - half) * s)
              + vec2<f32>(0.0, (1.0 - e) * prof.rise);
    // 世界坐标 → 相机 → 屏幕 px → NDC(Plan 3 L)。
    let screen = (world - globals.cam_pan) * globals.cam_zoom;
    let ndc = vec2<f32>(
        screen.x / globals.viewport.x * 2.0 - 1.0,
        1.0 - screen.y / globals.viewport.y * 2.0,
    );
    var out: VsOut;
    out.clip = vec4<f32>(ndc, 0.0, 1.0);
    out.uv = vec2<f32>(mix(inst.uv.x, inst.uv.z, c.x), mix(inst.uv.y, inst.uv.w, c.y));
    out.alpha = clamp(e, 0.0, 1.0) * inst.alpha; // 缓动淡入 × 静态 alpha(Plan 15 行窗边缘淡);emoji(kind3)同走
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
    // 两源都在统一控制流里采(textureSample/fwidth 需 uniform control flow),再按 kind 选;
    // 非 MSDF 实例的 msdf 采样作废(dummy 1×1 或越界层被 clamp,安全)。
    let r8 = textureSample(atlas_tex, atlas_samp, in.uv, i32(in.layer));
    let m = textureSample(msdf_tex, atlas_samp, in.uv, i32(in.layer)).rgb;
    let cov_sdf = sdf_coverage(r8.r);
    let cov_msdf = sdf_coverage(median3(m));
    var cov: f32;
    switch in.kind {
        case 0u: { cov = r8.r; }              // 位图覆盖率:.r 直采
        case 2u: { cov = cov_msdf; }          // MSDF:median 距离场
        case 3u: { return vec4<f32>(r8.rgb, r8.a * in.alpha); } // RGBA 彩字(emoji):直采真彩,fade 走 alpha(0015 §7.2)
        default: { cov = cov_sdf; }           // TinySDF
    }
    return vec4<f32>(in.tint, cov * in.alpha);
}
