// markdown/rule_cat.wgsl — 喵喵分隔线(`---` 默认组件,0026/Plan 11)。
//
// 移植自 bitless 的 Shadertoy "Cat Division"(https://www.shadertoy.com/),GLSL→WGSL 忠实移植。
// 形状/噪声技法源自 Inigo Quilez(sdParabola、smin:https://iquilezles.org/articles/distfunctions2d/、
// .../smin/)、Dave_Hoskins(hash without sin:https://www.shadertoy.com/view/4djSRW)、
// SnoopethDuckDuck(multi-wave thc:https://www.shadertoy.com/view/mlyGWt)。本项目 MIT / 非盈利。
//
// 适配为**横向分割线带**:一条波浪线 + 沿线**多只猫错时升起/停留/沉下**(出现-消失周期)、左右滑动、
// 眨眼。由 markdown/widget.wgsl 按 component id=2 调用,传 globals.time_ms 作动画时钟。
// 渲染:把"线 ∪ 猫"SDF 取 |k| 描边(线条画),眼睛/鼻子实心填充,两端淡出。
//
// ── 旋钮(肉眼定版后改)──
//   UNIT_PX   每 cat-unit 多少像素(越大猫越大)
//   CELL      相邻猫槽间距(cat-units;越小猫越密)
//   LINE_FRAC 分割线在 quad 的纵向位置(0=顶 1=底)
//   CAT_STROKE 轮廓半宽(cat-units)

const TIME_CICLE: f32 = 10.0;
const UNIT_PX: f32 = 12.0;
const CELL: f32 = 24.0;
const LINE_FRAC: f32 = 0.62;
const CAT_STROKE: f32 = 0.06;
const PI: f32 = 3.1415927;

struct CatP { pos: vec3<f32>, size: vec3<f32>, yP: f32 }

// multi-wave thc(SnoopethDuckDuck):tanh(a*cos(b))/tanh(a)。
fn thc(a: f32, b: f32) -> f32 { return tanh(a * cos(b)) / tanh(a); }

// hash without sin(Dave_Hoskins)。
fn hash11(p0: f32) -> f32 {
    var p = fract(p0 * 0.1031);
    p *= p + 33.33;
    p *= p + p;
    return fract(p);
}
fn hash22(p: vec2<f32>) -> vec2<f32> {
    var p3 = fract(vec3<f32>(p.xyx) * vec3<f32>(0.1031, 0.1030, 0.0973));
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.xx + p3.yz) * p3.zy);
}
// gradient noise。
fn noise2(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    return mix(mix(dot(hash22(i + vec2<f32>(0.0, 0.0)) - 0.5, f - vec2<f32>(0.0, 0.0)),
                   dot(hash22(i + vec2<f32>(1.0, 0.0)) - 0.5, f - vec2<f32>(1.0, 0.0)), u.x),
               mix(dot(hash22(i + vec2<f32>(0.0, 1.0)) - 0.5, f - vec2<f32>(0.0, 1.0)),
                   dot(hash22(i + vec2<f32>(1.0, 1.0)) - 0.5, f - vec2<f32>(1.0, 1.0)), u.x), u.y);
}

// IQ smin(Cat Division 同款 k 缩放)。
fn cat_smin(a: f32, b: f32, k0: f32) -> f32 {
    let k = k0 * 4.0;
    let h = max(k - abs(a - b), 0.0) / k;
    return min(a, b) - h * h * k * (1.0 / 4.0);
}

// IQ 抛物线 SDF(开口 +y)。
fn sd_parabola(pos0: vec2<f32>, k: f32) -> f32 {
    var pos = pos0;
    pos.x = abs(pos.x);
    let p = (pos.y * k - 0.5) / 3.0;
    let q = pos.x * k / 4.0;
    let h = q * q - p * p * p;
    var x: f32;
    if (h > 0.0) {
        let r = pow(q + sqrt(h), 1.0 / 3.0);
        x = r + p / r;
    } else {
        let r = sqrt(max(p, 1.0e-6));
        x = 2.0 * r * cos(acos(clamp(q / (p * r), -1.0, 1.0)) / 3.0);
    }
    let d = pos - vec2<f32>(x, x * x) / k;
    return length(d) * sign(d.x);
}

// 一只猫(bitless cat()):身体+头/耳两抛物线 smin;cp 携位置/尺寸/升降(出现-消失)。
fn cat(p0: vec2<f32>, cp: CatP) -> f32 {
    var p = p0;
    p.y += cp.yP;
    var x = mix(cp.pos.x, cp.pos.y, cp.pos.z) * 20.0;
    let y = -4.0 + cp.size.x;
    var k = (-0.5 - cp.yP) * 0.3;
    x += x * k;          // 升降时缩放
    p.x += p.x * k;
    k = 2.0;
    p.x += mix(0.0, cp.pos.x - cp.pos.y, 0.5 - abs(cp.pos.z - 0.5)) * p.y * 1.5; // 滑动 skew
    return -cat_smin(
        -sd_parabola(-p - vec2<f32>(x, y), k + cp.size.y),
        sd_parabola(p + vec2<f32>(x, -1.5 - cp.size.z), k + cp.size.y * 0.5),
        0.01,
    );
}

// 眼睛+鼻子(bitless catEyes),随时间眨眼/鼻子微动;line 为该处线高(对齐)。
fn cat_eyes(p0: vec2<f32>, cp: CatP, line: f32, t: f32) -> f32 {
    var p = p0;
    p += vec2<f32>(
        mix(cp.pos.x, cp.pos.y, cp.pos.z) * 10.0
            + mix(0.0, cp.pos.x - cp.pos.y, 0.5 - abs(cp.pos.z - 0.5)) * p.y * 1.5,
        cp.yP - 0.3 + cp.size.z * 2.0 + line * 0.2,
    );
    p.x *= 1.0 + cp.size.z;
    let f = length(abs(p + vec2<f32>(sin(t * 2.0) * 0.2, 0.0)) + vec2<f32>(-0.3, abs(sin(t * 2.0) * 0.05))) - 0.1;
    p += vec2<f32>(sin(t * 2.0) * 0.3, 0.2);
    let kk = sin(atan2(p.x, p.y) - 1.6) / 3.12415 * 12.0;
    return min(f, length(p) - 0.15 + cp.size.z * 0.2 + sin(kk) * 0.025);
}

fn md_rule_cat(local: vec2<f32>, half: vec2<f32>, color: vec4<f32>, time_ms: f32) -> vec4<f32> {
    let t = time_ms * 0.001;
    // quad → cat-space:y 向上、0 在分割线(LINE_FRAC 处);x 连续横跨全宽。
    let line_local_y = (LINE_FRAC * 2.0 - 1.0) * half.y;
    let p = vec2<f32>(local.x / UNIT_PX, (line_local_y - local.y) / UNIT_PX);

    // 沿 x 分槽:每槽一只猫,各自 hash + 时间周期(错时出现/消失);线是全局连续的波浪。
    let id = floor(p.x / CELL);
    let cx = p.x - (id + 0.5) * CELL;             // 槽内居中横坐标
    let lT = t + hash11(id) * 1.0e3;
    let n = hash11(floor(lT / TIME_CICLE));
    var cp: CatP;
    cp.pos = vec3<f32>(n - 0.5, hash11(floor(lT / TIME_CICLE + 1.0)) - 0.5,
                       smoothstep(0.4, 0.6, fract(lT / TIME_CICLE)));
    cp.yP = (1.0 - thc(5.0, (lT + TIME_CICLE / 2.0) / TIME_CICLE * 2.0 * PI)) * 4.0 - 0.5;
    cp.size = vec3<f32>(hash11(n) - 1.5, hash11(n + 0.1) - 0.5, hash11(n + 0.2) * 0.25);

    // 波浪分割线(全局 x 连续)+ 该槽的猫(局部 cx)。
    let gx = p.x * 2.0;
    let py = p.y * 2.0;
    let line = py
        - noise2(vec2<f32>(gx * 0.2 + sin(t * 0.4 + id) * 0.5, t * 0.6 + id * 0.5)) * 8.0 * smoothstep(15.0, 0.0, abs(cx * 2.0))
        + noise2(vec2<f32>(gx, t + id)) * 0.5;
    let lc = vec2<f32>(cx * 2.0, py);           // 身体用**翻倍**坐标(对应原作 f() 内 p*=2)
    let catd = cat(lc + vec2<f32>(0.0, line * 0.25), cp);
    let k = cat_smin(line, catd, 0.1);

    // 线条画:|k| 描边;眼睛/鼻子实心填充。
    let aa = max(fwidth(k), 0.001);
    let outline = smoothstep(aa, -aa, abs(k) - CAT_STROKE);
    // 眼睛/鼻子用**未翻倍**坐标(原作 catEyes(lc),body 在 2× 空间、eyes 在 1× 空间,各自常量已调好)。
    let e = cat_eyes(vec2<f32>(cx, p.y), cp, line, t);
    let ae = max(fwidth(e), 0.001);
    let eyes = smoothstep(ae, -ae, e);

    let cov = max(outline, eyes); // 不做两端淡出,整条等亮
    return vec4<f32>(color.rgb, color.a * cov);
}
