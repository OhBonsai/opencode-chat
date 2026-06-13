---
name: render-write
description: 写 wgpu / WGSL 渲染代码 — 自动加载渲染铁律。触发场景:用户说 "/render-write"、"写 shader"、"改渲染"、改 crates/render(M8 scene / M9 effects / M10 render)时执行。
---

# /render-write · 写渲染代码(wgpu / WGSL)

> 触发先 Read DEVMEM § 2。先过 `/rust-write` 的 AR/CR 铁律(渲染也是 Rust),本 skill 加渲染专属。
> 涉及模块:M8 scene · M9 effects · M10 render。

## 渲染铁律 RD

| # | 铁律 | 来源 |
|---|---|---|
| **RD1** | 渲染只读状态,禁回写 store(同 AR1) | 0002 |
| **RD2** | 逐字符效果靠 WGSL `time - spawn_time`,CPU 零参与;禁每帧 CPU 改每个 instance | 0002 §5 |
| **RD3** | 关闭效果 = 参数置零(profile full/reduced/off),禁 `if enabled` 分支 | 0002 §5.1 |
| **RD4** | 效果 `age>=duration` 后恒等;hit-test/选区/滚动只读 settled 几何(同 AR3) | 0002 §5.1 |
| **RD5** | instance 按 y 有序,视口裁剪二分,只 draw 可见区间;禁全量 draw | 0002 §6 |
| **RD6** | 屏外块释放 instance 只留高度+文本;atlas LRU 淘汰屏外字形 | 0002/0004 |
| **RD7** | atlas 预分配多页 + CJK 分桶 + emoji RGBA 单独页 | 0004 §7.4 |
| **RD8** | 相机 world unit = CSS pixel,统一正文/卡片/overlay/无障碍镜像坐标系 | 0007 §3 |
| **RD9** | RenderBackend trait 选后端(WebGPU/WebGL2/Canvas2D),禁 cfg 堆(同 CR3) | 0003 §5 |
| **RD10** | wgpu memory/buffer grow 后重建视图;WGSL 经 naga 构建期校验 | — |
| **RD11** | tween 池只写 presentation 字段,不写 model(同 AR2) | 0002 §5.1 |
| **RD12** | 图片/mermaid/SVG 走浏览器光栅化 → 纹理,wasm 只持元数据;禁打包 resvg/字体 | 0004 §7 / 0007 |

## 4 步流程

1. **读 context**:相关 decision(0002/0004/0007)+ 现有 shader/管线
2. **定层**:改 scene(组织)/ effects(动画)/ render(后端)哪层
3. **写 + 自检**:
   - 新效果?→ 进 WGSL,加 profile 参数,off=零(RD2/RD3),验证恒等收敛(RD4)
   - 改 draw?→ 视口裁剪二分(RD5),不全量
   - 新字形需求?→ atlas 分桶/分页(RD7)
   - 新后端能力?→ 走 RenderBackend trait(RD9)
4. **验证**:off-profile 渲染到纹理 readback → 与 golden PNG diff(确定性);
   `cargo clippy` + naga 校验 WGSL

## 反模式

- ❌ 每帧 CPU 遍历改 instance 做动画(违 RD2)
- ❌ `if effect_on { … }` 切代码路径(违 RD3,用 profile 置零)
- ❌ 全量 draw 不裁剪(违 RD5)
- ❌ 打包字体/resvg 做 mermaid(违 RD12,交浏览器)
- ❌ 渲染路径写 store(违 RD1)
