// markdown/rule.wgsl — 分隔线组件(`---`,0026/Plan 11)。中间亮、两端淡出的渐变细线。
//
// 纯组件函数,无 entry/binding;由 markdown/widget.wgsl 按 component id=1 调用。
// quad = 整行宽 × 数像素高(留 AA 余量);线居 quad 纵向中线。
// params:.x=线半厚 px  .y=两端淡出起点(占半宽比例,0..1;0 = 全程渐变)  .z/.w 保留。

fn md_rule(local: vec2<f32>, half: vec2<f32>, color: vec4<f32>, params: vec4<f32>) -> vec4<f32> {
    let half_th = max(params.x, 0.5);
    // 纵向:到水平中线距离 → 细线 + fwidth 屏幕空间 AA(top-level 调用 → 均匀控制流)。
    let dy = abs(local.y);
    let aa = max(fwidth(local.y), 0.0001);
    let cov_v = 1.0 - smoothstep(half_th - aa, half_th + aa, dy);
    // 横向:中间亮、两端平滑淡出(u∈[-1,1];中央 `bright` 比例全亮,向两端 smoothstep 落到 0)。
    let u = local.x / max(half.x, 1.0);
    let bright = clamp(params.y, 0.0, 0.95);
    let fade = 1.0 - smoothstep(bright, 1.0, abs(u));
    return vec4<f32>(color.rgb, color.a * cov_v * fade);
}
