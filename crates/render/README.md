# opencode-chat-render

wgpu 渲染后端(M8 scene / M9 effects / M10 render)。输入 core 的 `FrameData`(语义字形),
输出像素。

## 铁律

- **CR3**:后端用 `RenderBackend` trait 选择,不用 `cfg` 堆后端。Plan1 只实现
  `WebGpuBackend`;WebGL2/Canvas2D 降级留 Plan2(同 wgpu API)。
- **CR4**:`GpuInstance` 平铺 `#[repr(C)]` + `bytemuck::Pod`,零拷贝上传。
- **0002 §5**:淡入靠 `time - spawn_time` 在 WGSL 算,CPU 零参与;`EffectProfile::Off`
  即 `fade_ms=0`(参数置零,恒等收敛 AR3)。
- 本 crate **不依赖 web-sys**:surface 由组装方(wasm)从 canvas 建好后注入。

## 模块

| 文件 | 职责 |
|---|---|
| `backend.rs` | `RenderBackend` trait + `WebGpuBackend`(wgpu 管线/帧绘制) |
| `atlas.rs` | glyph 图集:单张纹理 + UV 表 + shelf 装箱 |
| `scene.rs` | `GpuInstance`(Pod)+ `FrameData`→instance 组装 |
| `effects.rs` | `EffectProfile` + `Globals` uniform |
| `shaders/glyph.wgsl` | 实例化字形 + 着色器淡入 |

## 测试

```bash
cargo test -p opencode-chat-render   # naga 构建期校验 glyph.wgsl(不需 GPU)
```

实际像素渲染需浏览器 WebGPU(经 `crates/wasm` + `web/` harness)。
