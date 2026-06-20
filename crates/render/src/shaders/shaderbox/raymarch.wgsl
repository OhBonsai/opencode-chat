// shaderbox/raymarch.wgsl — 收编 0024 §4B raymarch 为一个 shader-id(Plan 16 ⑤,留位)。
// 小区域 3D SDF raymarch(**自写**:标准 sphere/box + smin smooth-union;参考 0024 §4B 思路)。
// **平台 caps(护栏5)**:`RM_MAX_STEPS` 步数封顶 + `RM_SURF` 精度。WebGPU 桌面用此默认;WebGL2/
// 移动端降级时应换更小的 cap + 降精度(运行时探测换 shader/const,v1 单 cap 占位,见 §2.3/§6)。
// params:`p0.rgb`(非零)= 物体基色;`time` 驱动旋转/形变。

const RM_MAX_STEPS: i32 = 48;   // 护栏5:步数封顶(移动端应降到 ~24)
const RM_MAX_DIST: f32 = 8.0;
const RM_SURF: f32 = 0.002;

fn rm_sphere(p: vec3<f32>, r: f32) -> f32 { return length(p) - r; }
fn rm_box(p: vec3<f32>, b: vec3<f32>) -> f32 {
    let q = abs(p) - b;
    return length(max(q, vec3<f32>(0.0))) + min(max(q.x, max(q.y, q.z)), 0.0);
}
fn rm_smin(a: f32, b: f32, k: f32) -> f32 {
    let h = clamp(0.5 + 0.5 * (b - a) / k, 0.0, 1.0);
    return mix(b, a, h) - k * h * (1.0 - h);
}
fn rm_rot_y(p: vec3<f32>, a: f32) -> vec3<f32> {
    let c = cos(a);
    let s = sin(a);
    return vec3<f32>(c * p.x + s * p.z, p.y, -s * p.x + c * p.z);
}
fn rm_map(p0: vec3<f32>, t: f32) -> f32 {
    let p = rm_rot_y(p0, t);
    let s = rm_sphere(p - vec3<f32>(0.35 * sin(t), 0.0, 0.0), 0.45);
    let b = rm_box(p + vec3<f32>(0.35 * sin(t), 0.0, 0.0), vec3<f32>(0.32));
    return rm_smin(s, b, 0.25);
}
fn rm_normal(p: vec3<f32>, t: f32) -> vec3<f32> {
    let e = vec2<f32>(0.001, 0.0);
    return normalize(vec3<f32>(
        rm_map(p + e.xyy, t) - rm_map(p - e.xyy, t),
        rm_map(p + e.yxy, t) - rm_map(p - e.yxy, t),
        rm_map(p + e.yyx, t) - rm_map(p - e.yyx, t),
    ));
}
fn shade(c: ShadeCtx) -> vec4<f32> {
    let uv = (c.uv - 0.5) * 2.0; // -1..1
    let ro = vec3<f32>(0.0, 0.0, -2.2);
    let rd = normalize(vec3<f32>(uv.x, -uv.y, 1.6));
    var d = 0.0;
    var hit = false;
    var p = ro;
    for (var i = 0; i < RM_MAX_STEPS; i = i + 1) {
        p = ro + rd * d;
        let dist = rm_map(p, c.time);
        if (dist < RM_SURF) {
            hit = true;
            break;
        }
        d = d + dist;
        if (d > RM_MAX_DIST) {
            break;
        }
    }
    if (!hit) {
        return vec4<f32>(0.0, 0.0, 0.0, 0.0); // 背景透明 → over 下层
    }
    let n = rm_normal(p, c.time);
    let l = normalize(vec3<f32>(0.6, 0.8, -0.5));
    let diff = clamp(dot(n, l), 0.0, 1.0);
    var base = vec3<f32>(0.4, 0.6, 1.0);
    if (c.p0.x + c.p0.y + c.p0.z > 0.0) {
        base = c.p0.rgb;
    }
    let col = base * (0.2 + 0.8 * diff) + vec3<f32>(0.3) * pow(diff, 16.0);
    return vec4<f32>(col, 1.0);
}
