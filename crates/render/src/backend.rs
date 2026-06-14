//! backend(M10)— `RenderBackend` trait + `WebGpuBackend`(SDF tile,Plan 3 K)。
//!
//! 后端用 trait 选择(CR3);WebGL2/Canvas2D 降级留后续(同 wgpu API)。surface 由组装方
//! (wasm)从 canvas 建好后注入,故本 crate 不依赖 web-sys。
//!
//! Plan 3 K:atlas 存 SDF tile(R8 多页),实例由组装方按可见集逐帧构建(它持 JS 光栅器),
//! 后端只负责 atlas 槽位管理 + 上传 + 绘制。

use crate::atlas::{Alloc, MsdfAtlas, SdfAtlas, Slot};
use crate::effects::Globals;
use crate::scene::{GpuInstance, RectInstance};

/// 渲染后端抽象(CR3)。实例由调用方构建(它持平台光栅器),后端管 atlas + 绘制。
pub trait RenderBackend {
    /// 画布尺寸变化时重配 surface。
    fn resize(&mut self, width: u32, height: u32);
    /// 每帧开头:清 atlas 钉住集。
    fn atlas_begin_frame(&mut self);
    /// 钉住本帧可见字形(不被 LRU 淘汰)。
    fn atlas_pin(&mut self, key: &str);
    /// 取/分配字形槽;`is_new` 时需 [`atlas_upload`](RenderBackend::atlas_upload)。
    fn atlas_alloc(&mut self, key: &str) -> Alloc;
    /// 上传一张 SDF tile 到槽。
    fn atlas_upload(&mut self, slot: Slot, sdf: &[u8]);
    /// atlas 可观测:(占用, 容量, 累计淘汰)。默认 0(非 GPU 后端可不实现)。
    fn atlas_stats(&self) -> (usize, usize, u64) {
        (0, 0, 0)
    }
    /// (重)建 MSDF 静态图集为 `w×h×pages`(0015)。默认 no-op。
    fn msdf_init(&mut self, _w: u32, _h: u32, _pages: u32) {}
    /// 上传一整页 MSDF RGBA 像素到第 `page` 层。默认 no-op。
    fn msdf_upload(&mut self, _page: u32, _rgba: &[u8]) {}
    /// MSDF 图集是否已加载(决定能否解析 MSDF 源)。默认 false。
    fn msdf_loaded(&self) -> bool {
        false
    }
    /// 绘制本帧。`rects` 作背景先于 `glyphs`(同相机/裁剪,Plan 4B);`time_ms`/`fade_ms`
    /// 驱动淡入;`cam_pan`/`cam_zoom` 是 2D 相机(L)。
    fn draw(
        &mut self,
        glyphs: &[GpuInstance],
        rects: &[RectInstance],
        time_ms: f32,
        fade_ms: f32,
        cam_pan: [f32; 2],
        cam_zoom: f32,
    ) -> Result<(), RenderError>;
}

#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    #[error("没有可用的 GPU 适配器: {0}")]
    NoAdapter(String),
    #[error("请求 device 失败: {0}")]
    Device(String),
    #[error("surface 无可用纹理格式")]
    NoFormat,
    #[error("获取 surface 纹理失败: {0}")]
    Surface(#[from] wgpu::SurfaceError),
}

const CLEAR: wgpu::Color = wgpu::Color {
    r: 0.05,
    g: 0.06,
    b: 0.09,
    a: 1.0,
};

/// 构建 glyph bind group(globals + R8 atlas + sampler + MSDF 图集)。MSDF 纹理重建后需重调。
fn make_glyph_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    globals_buf: &wgpu::Buffer,
    atlas: &SdfAtlas,
    msdf: &MsdfAtlas,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("glyph-bind-group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: globals_buf.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(atlas.view()),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Sampler(atlas.sampler()),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: wgpu::BindingResource::TextureView(msdf.view()),
            },
        ],
    })
}

/// WebGPU 后端。
pub struct WebGpuBackend {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    pipeline: wgpu::RenderPipeline,
    globals_buf: wgpu::Buffer,
    bind_layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
    atlas: SdfAtlas,
    msdf: MsdfAtlas,
    instance_buf: Option<wgpu::Buffer>,
    instance_cap: u64,
    rect_pipeline: wgpu::RenderPipeline,
    rect_bind_group: wgpu::BindGroup,
    rect_buf: Option<wgpu::Buffer>,
    rect_cap: u64,
}

impl WebGpuBackend {
    /// 用注入的 instance + surface 初始化(surface 须 `'static`,由 canvas 建好)。
    ///
    /// # Errors
    /// 无 GPU 适配器 / device 创建失败 / surface 无格式时返回 [`RenderError`]。
    pub async fn new(
        instance: &wgpu::Instance,
        surface: wgpu::Surface<'static>,
        width: u32,
        height: u32,
    ) -> Result<Self, RenderError> {
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .map_err(|e| RenderError::NoAdapter(e.to_string()))?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("infinite-chat-device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::default(),
                trace: wgpu::Trace::Off,
            })
            .await
            .map_err(|e| RenderError::Device(e.to_string()))?;

        let caps = surface.get_capabilities(&adapter);
        let format = caps
            .formats
            .iter()
            .copied()
            .find(wgpu::TextureFormat::is_srgb)
            .or_else(|| caps.formats.first().copied())
            .ok_or(RenderError::NoFormat)?;
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: width.max(1),
            height: height.max(1),
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: caps
                .alpha_modes
                .first()
                .copied()
                .unwrap_or(wgpu::CompositeAlphaMode::Auto),
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let atlas = SdfAtlas::new(&device);
        let msdf = MsdfAtlas::dummy(&device);
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("glyph-shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/glyph.wgsl").into()),
        });

        let bind_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("glyph-bind-layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2Array,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                // 3:MSDF 静态图集(RGBA D2Array,0015);未加载时为 1×1 占位。
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2Array,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
        });

        let globals_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("globals"),
            size: std::mem::size_of::<Globals>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group = make_glyph_bind_group(&device, &bind_layout, &globals_buf, &atlas, &msdf);

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("glyph-pipeline-layout"),
            bind_group_layouts: &[&bind_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("glyph-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[GpuInstance::layout()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // 矩形管线(Plan 4B):仅绑 globals(无 atlas),独立 WGSL + 顶点布局。
        let rect_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("rect-shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/rect.wgsl").into()),
        });
        let rect_bind_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("rect-bind-layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let rect_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("rect-bind-group"),
            layout: &rect_bind_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: globals_buf.as_entire_binding(),
            }],
        });
        let rect_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("rect-pipeline-layout"),
            bind_group_layouts: &[&rect_bind_layout],
            push_constant_ranges: &[],
        });
        let rect_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("rect-pipeline"),
            layout: Some(&rect_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &rect_shader,
                entry_point: Some("vs_main"),
                buffers: &[RectInstance::layout()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &rect_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        Ok(Self {
            surface,
            device,
            queue,
            config,
            pipeline,
            globals_buf,
            bind_layout,
            bind_group,
            atlas,
            msdf,
            instance_buf: None,
            instance_cap: 0,
            rect_pipeline,
            rect_bind_group,
            rect_buf: None,
            rect_cap: 0,
        })
    }

    fn ensure_instance_buffer(&mut self, needed: u64) {
        if self.instance_cap >= needed && self.instance_buf.is_some() {
            return;
        }
        let cap = needed.next_power_of_two().max(256);
        self.instance_buf = Some(self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("glyph-instances"),
            size: cap * std::mem::size_of::<GpuInstance>() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));
        self.instance_cap = cap;
    }

    fn ensure_rect_buffer(&mut self, needed: u64) {
        if self.rect_cap >= needed && self.rect_buf.is_some() {
            return;
        }
        let cap = needed.next_power_of_two().max(64);
        self.rect_buf = Some(self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("rect-instances"),
            size: cap * std::mem::size_of::<RectInstance>() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));
        self.rect_cap = cap;
    }
}

impl RenderBackend for WebGpuBackend {
    fn resize(&mut self, width: u32, height: u32) {
        self.config.width = width.max(1);
        self.config.height = height.max(1);
        self.surface.configure(&self.device, &self.config);
    }

    fn atlas_begin_frame(&mut self) {
        self.atlas.begin_frame();
    }

    fn atlas_pin(&mut self, key: &str) {
        self.atlas.pin(key);
    }

    fn atlas_alloc(&mut self, key: &str) -> Alloc {
        self.atlas.alloc(key)
    }

    fn atlas_stats(&self) -> (usize, usize, u64) {
        self.atlas.stats()
    }

    fn atlas_upload(&mut self, slot: Slot, sdf: &[u8]) {
        self.atlas.upload(&self.queue, slot, sdf);
    }

    fn msdf_init(&mut self, w: u32, h: u32, pages: u32) {
        self.msdf.init(&self.device, w, h, pages);
        // 纹理换了 → 重建 bind group 指向新 view。
        self.bind_group = make_glyph_bind_group(
            &self.device,
            &self.bind_layout,
            &self.globals_buf,
            &self.atlas,
            &self.msdf,
        );
    }

    fn msdf_upload(&mut self, page: u32, rgba: &[u8]) {
        self.msdf.upload_page(&self.queue, page, rgba);
    }

    fn msdf_loaded(&self) -> bool {
        self.msdf.loaded()
    }

    fn draw(
        &mut self,
        glyphs: &[GpuInstance],
        rects: &[RectInstance],
        time_ms: f32,
        fade_ms: f32,
        cam_pan: [f32; 2],
        cam_zoom: f32,
    ) -> Result<(), RenderError> {
        let globals = Globals {
            viewport: [self.config.width as f32, self.config.height as f32],
            time_ms,
            fade_ms,
            cam_pan,
            cam_zoom,
            pad: 0.0,
        };
        self.queue
            .write_buffer(&self.globals_buf, 0, bytemuck::bytes_of(&globals));

        if !glyphs.is_empty() {
            self.ensure_instance_buffer(glyphs.len() as u64);
            if let Some(buf) = &self.instance_buf {
                self.queue
                    .write_buffer(buf, 0, bytemuck::cast_slice(glyphs));
            }
        }
        if !rects.is_empty() {
            self.ensure_rect_buffer(rects.len() as u64);
            if let Some(buf) = &self.rect_buf {
                self.queue.write_buffer(buf, 0, bytemuck::cast_slice(rects));
            }
        }

        let surface_tex = self.surface.get_current_texture()?;
        let view = surface_tex
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("frame-encoder"),
            });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("chat-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(CLEAR),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            // 背景:矩形先于文字(Plan 4B)。
            if let Some(buf) = &self.rect_buf {
                if !rects.is_empty() {
                    pass.set_pipeline(&self.rect_pipeline);
                    pass.set_bind_group(0, &self.rect_bind_group, &[]);
                    pass.set_vertex_buffer(0, buf.slice(..));
                    pass.draw(0..4, 0..rects.len() as u32);
                }
            }
            if let Some(buf) = &self.instance_buf {
                if !glyphs.is_empty() {
                    pass.set_pipeline(&self.pipeline);
                    pass.set_bind_group(0, &self.bind_group, &[]);
                    pass.set_vertex_buffer(0, buf.slice(..));
                    pass.draw(0..4, 0..glyphs.len() as u32);
                }
            }
        }
        self.queue.submit(std::iter::once(encoder.finish()));
        surface_tex.present();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    /// 构建期校验 WGSL(naga),不依赖 GPU。
    #[test]
    fn glyph_shader_is_valid_wgsl() {
        let src = include_str!("shaders/glyph.wgsl");
        let module = naga::front::wgsl::parse_str(src).expect("WGSL 解析失败");
        let mut validator = naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        );
        validator.validate(&module).expect("WGSL 校验失败");
    }

    #[test]
    fn rect_shader_is_valid_wgsl() {
        let src = include_str!("shaders/rect.wgsl");
        let module = naga::front::wgsl::parse_str(src).expect("WGSL 解析失败");
        let mut validator = naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        );
        validator.validate(&module).expect("WGSL 校验失败");
    }
}
