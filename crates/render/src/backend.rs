//! backend(M10)— `RenderBackend` trait + `WebGpuBackend`(SDF tile,Plan 3 K)。
//!
//! 后端用 trait 选择(CR3);WebGL2/Canvas2D 降级留后续(同 wgpu API)。surface 由组装方
//! (wasm)从 canvas 建好后注入,故本 crate 不依赖 web-sys。
//!
//! Plan 3 K:atlas 存 SDF tile(R8 多页),实例由组装方按可见集逐帧构建(它持 JS 光栅器),
//! 后端只负责 atlas 槽位管理 + 上传 + 绘制。

use crate::atlas::{Alloc, MsdfAtlas, SdfAtlas, Slot};
use crate::effects::Globals;
use crate::scene::{GpuInstance, ImageInstance, PanelInstance, RectInstance, WidgetInstance};

/// 共享 SDF 形状库(0026):前置拼接到需要它的 pipeline 源(rect/panel/markdown widget)。
/// WGSL"声明先于使用",故置于调用方之前。glyph 不需(用自带 sdf_coverage)。
const SDF_LIB: &str = include_str!("shaders/base/sdf.wgsl");

/// build 期把共享 SDF 库前置拼接到 pipeline 源(0026 §3.2,最简模块化,零预处理器依赖)。
fn with_sdf(srcs: &[&str]) -> String {
    let mut out = String::from(SDF_LIB);
    for s in srcs {
        out.push('\n');
        out.push_str(s);
    }
    out
}

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
    /// 上传一张 `w×h` RGBA 图(Plan 14 ②③)→ 返回 `tex_id`(1 起;0 = 失败/不支持)。默认 no-op。
    fn upload_image(&mut self, _rgba: &[u8], _w: u32, _h: u32) -> u32 {
        0
    }
    /// 绘制本帧。`rects` 作背景先于 `glyphs`(同相机/裁剪,Plan 4B);`time_ms`/`fade_ms`
    /// 驱动淡入;`cam_pan`/`cam_zoom` 是 2D 相机(L)。
    #[allow(clippy::too_many_arguments)] // reason: 多类背景图元 + 相机参数;拆 struct 反而绕
    fn draw(
        &mut self,
        glyphs: &[GpuInstance],
        rects: &[RectInstance],
        panels: &[PanelInstance],
        params: &[f32],
        widgets: &[WidgetInstance],
        images: &[ImageInstance],
        image_tex_ids: &[u32],
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

/// 参数化 SDF 面板管线(Plan 6 / 0018)。需 fragment storage buffer;无此能力(WebGL2)时为
/// `None`,面板降级不画(装饰回退由上层处理,WebGL2 兜底 = data texture 留后续)。
struct PanelPipeline {
    pipeline: wgpu::RenderPipeline,
    bind_layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
    inst_buf: Option<wgpu::Buffer>,
    inst_cap: u64,
    params_buf: wgpu::Buffer,
    params_cap: u64,
}

fn make_panel_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    globals_buf: &wgpu::Buffer,
    params_buf: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("panel-bind-group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: globals_buf.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: params_buf.as_entire_binding(),
            },
        ],
    })
}

/// 建面板管线(0018)。需 fragment storage buffer;不支持(WebGL2)→ `None`。
fn make_panel(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    globals_buf: &wgpu::Buffer,
) -> Option<PanelPipeline> {
    if device.limits().max_storage_buffers_per_shader_stage < 1 {
        tracing::info!(target: "M8", "无 fragment storage buffer(WebGL2?)→ 面板图元降级不画(0018 兜底留后续)");
        return None;
    }
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("panel-shader"),
        source: wgpu::ShaderSource::Wgsl(
            with_sdf(&[include_str!("shaders/base/panel.wgsl")]).into(),
        ),
    });
    let bind_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("panel-bind-layout"),
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
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    });
    let params_buf = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("panel-params"),
        size: 1024, // 起始小容量(256 f32),不够时 ensure 增长
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let bind_group = make_panel_bind_group(device, &bind_layout, globals_buf, &params_buf);
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("panel-pipeline-layout"),
        bind_group_layouts: &[&bind_layout],
        push_constant_ranges: &[],
    });
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("panel-pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[PanelInstance::layout()],
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
    Some(PanelPipeline {
        pipeline,
        bind_layout,
        bind_group,
        inst_buf: None,
        inst_cap: 0,
        params_buf,
        params_cap: 256,
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
    /// markdown 组件管线(0026/Plan 11);bind group 复用 `rect_bind_group`(同绑 globals)。
    widget_pipeline: wgpu::RenderPipeline,
    widget_buf: Option<wgpu::Buffer>,
    widget_cap: u64,
    /// 图片纹理管线(Plan 14 ②):group0 复用 `rect_bind_group`(globals),group1 = per-image 纹理。
    image_pipeline: wgpu::RenderPipeline,
    image_tex_layout: wgpu::BindGroupLayout,
    image_sampler: wgpu::Sampler,
    /// 已上传图片:`tex_id-1` → (纹理, group1 bind group)。v1 不淘汰(§7 记基准,超阈值再 LRU)。
    images: Vec<(wgpu::Texture, wgpu::BindGroup)>,
    image_buf: Option<wgpu::Buffer>,
    image_cap: u64,
    panel: Option<PanelPipeline>,
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
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/base/glyph.wgsl").into()),
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
            source: wgpu::ShaderSource::Wgsl(
                with_sdf(&[include_str!("shaders/base/rect.wgsl")]).into(),
            ),
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

        // markdown 组件管线(0026/Plan 11):一条 pipeline 画所有 markdown 组件,fragment 按
        // component id 分派(box=0…)。仅绑 globals(复用 rect 的 bind layout/group:同为单 uniform)。
        // 源 = base/sdf + box + widget 拼接(声明先于使用)。无 atlas/storage → WebGL2 友好。
        let widget_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("markdown-widget-shader"),
            source: wgpu::ShaderSource::Wgsl(
                with_sdf(&[
                    include_str!("shaders/markdown/box.wgsl"),
                    include_str!("shaders/markdown/rule.wgsl"),
                    include_str!("shaders/markdown/rule_cat.wgsl"),
                    include_str!("shaders/markdown/widget.wgsl"),
                ])
                .into(),
            ),
        });
        let widget_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("markdown-widget-pipeline-layout"),
                bind_group_layouts: &[&rect_bind_layout],
                push_constant_ranges: &[],
            });
        let widget_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("markdown-widget-pipeline"),
            layout: Some(&widget_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &widget_shader,
                entry_point: Some("vs_main"),
                buffers: &[WidgetInstance::layout()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &widget_shader,
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

        // 图片管线(Plan 14 ②):纹理 quad。group 0 = globals(复用 rect_bind_layout/group,单 uniform);
        // group 1 = per-image texture + sampler(每张图一组 bind group,draw 时换绑,warp 同范式)。
        let image_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("image-shader"),
            source: wgpu::ShaderSource::Wgsl(
                with_sdf(&[include_str!("shaders/base/image.wgsl")]).into(),
            ),
        });
        let image_tex_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("image-tex-layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let image_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("image-sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let image_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("image-pipeline-layout"),
                bind_group_layouts: &[&rect_bind_layout, &image_tex_layout],
                push_constant_ranges: &[],
            });
        let image_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("image-pipeline"),
            layout: Some(&image_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &image_shader,
                entry_point: Some("vs_main"),
                buffers: &[ImageInstance::layout()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &image_shader,
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

        let panel = make_panel(&device, format, &globals_buf);

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
            widget_pipeline,
            widget_buf: None,
            widget_cap: 0,
            image_pipeline,
            image_tex_layout,
            image_sampler,
            images: Vec::new(),
            image_buf: None,
            image_cap: 0,
            panel,
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

    fn ensure_widget_buffer(&mut self, needed: u64) {
        if self.widget_cap >= needed && self.widget_buf.is_some() {
            return;
        }
        let cap = needed.next_power_of_two().max(64);
        self.widget_buf = Some(self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("markdown-widget-instances"),
            size: cap * std::mem::size_of::<WidgetInstance>() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));
        self.widget_cap = cap;
    }

    fn ensure_image_buffer(&mut self, needed: u64) {
        if self.image_cap >= needed && self.image_buf.is_some() {
            return;
        }
        let cap = needed.next_power_of_two().max(16);
        self.image_buf = Some(self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("image-instances"),
            size: cap * std::mem::size_of::<ImageInstance>() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));
        self.image_cap = cap;
    }

    /// 上传面板参数(storage buffer,增量改脏区:每帧整写,容量不足才重建+重绑)+ 实例(0018 §2)。
    fn upload_panels(&mut self, panels: &[PanelInstance], params: &[f32]) {
        let Some(p) = &mut self.panel else { return };
        if panels.is_empty() {
            return;
        }
        let need_params = params.len().max(1) as u64;
        if p.params_cap < need_params {
            let cap = need_params.next_power_of_two().max(256);
            p.params_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("panel-params"),
                size: cap * 4,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            p.params_cap = cap;
            p.bind_group = make_panel_bind_group(
                &self.device,
                &p.bind_layout,
                &self.globals_buf,
                &p.params_buf,
            );
        }
        if !params.is_empty() {
            self.queue
                .write_buffer(&p.params_buf, 0, bytemuck::cast_slice(params));
        }
        let need_inst = panels.len() as u64;
        if p.inst_cap < need_inst || p.inst_buf.is_none() {
            let cap = need_inst.next_power_of_two().max(64);
            p.inst_buf = Some(self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("panel-instances"),
                size: cap * std::mem::size_of::<PanelInstance>() as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }));
            p.inst_cap = cap;
        }
        if let Some(buf) = &p.inst_buf {
            self.queue
                .write_buffer(buf, 0, bytemuck::cast_slice(panels));
        }
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

    fn upload_image(&mut self, rgba: &[u8], w: u32, h: u32) -> u32 {
        if w == 0 || h == 0 || rgba.len() < (w as usize) * (h as usize) * 4 {
            return 0;
        }
        let size = wgpu::Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        };
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("image-tex"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb, // getImageData = sRGB 字节 → 着色器线性化
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &rgba[..(w as usize) * (h as usize) * 4],
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(w * 4),
                rows_per_image: Some(h),
            },
            size,
        );
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("image-bind-group"),
            layout: &self.image_tex_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.image_sampler),
                },
            ],
        });
        self.images.push((texture, bind_group));
        self.images.len() as u32 // tex_id = 1 起(0 留作"无")
    }

    #[allow(clippy::too_many_arguments)]
    fn draw(
        &mut self,
        glyphs: &[GpuInstance],
        rects: &[RectInstance],
        panels: &[PanelInstance],
        params: &[f32],
        widgets: &[WidgetInstance],
        images: &[ImageInstance],
        image_tex_ids: &[u32],
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
        if !widgets.is_empty() {
            self.ensure_widget_buffer(widgets.len() as u64);
            if let Some(buf) = &self.widget_buf {
                self.queue
                    .write_buffer(buf, 0, bytemuck::cast_slice(widgets));
            }
        }
        if !images.is_empty() {
            self.ensure_image_buffer(images.len() as u64);
            if let Some(buf) = &self.image_buf {
                self.queue
                    .write_buffer(buf, 0, bytemuck::cast_slice(images));
            }
        }
        self.upload_panels(panels, params);

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
            // 背景:面板(SDF 容器,6/0018)→ 矩形 → 文字(Plan 4B)。
            if let Some(p) = &self.panel {
                if let Some(buf) = &p.inst_buf {
                    if !panels.is_empty() {
                        pass.set_pipeline(&p.pipeline);
                        pass.set_bind_group(0, &p.bind_group, &[]);
                        pass.set_vertex_buffer(0, buf.slice(..));
                        pass.draw(0..4, 0..panels.len() as u32);
                    }
                }
            }
            if let Some(buf) = &self.rect_buf {
                if !rects.is_empty() {
                    pass.set_pipeline(&self.rect_pipeline);
                    pass.set_bind_group(0, &self.rect_bind_group, &[]);
                    pass.set_vertex_buffer(0, buf.slice(..));
                    pass.draw(0..4, 0..rects.len() as u32);
                }
            }
            // markdown 组件(0026/Plan 11):rect 之后、文字之前(框/勾作背景,marker 字格零墨不挡)。
            if let Some(buf) = &self.widget_buf {
                if !widgets.is_empty() {
                    pass.set_pipeline(&self.widget_pipeline);
                    pass.set_bind_group(0, &self.rect_bind_group, &[]);
                    pass.set_vertex_buffer(0, buf.slice(..));
                    pass.draw(0..4, 0..widgets.len() as u32);
                }
            }
            // 图片纹理 quad(Plan 14 ②):图作底、文字压上。每图换 group1(per-image 纹理),逐实例绘。
            if let Some(buf) = &self.image_buf {
                if !images.is_empty() {
                    pass.set_pipeline(&self.image_pipeline);
                    pass.set_bind_group(0, &self.rect_bind_group, &[]); // globals
                    pass.set_vertex_buffer(0, buf.slice(..));
                    for (i, &tex_id) in image_tex_ids.iter().enumerate() {
                        let Some((_, bg)) = tex_id
                            .checked_sub(1)
                            .and_then(|idx| self.images.get(idx as usize))
                        else {
                            continue; // 无效 tex_id(未上传)
                        };
                        pass.set_bind_group(1, bg, &[]);
                        pass.draw(0..4, i as u32..i as u32 + 1);
                    }
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
    use super::with_sdf;

    /// 构建期校验 WGSL(naga),不依赖 GPU。校验的是**拼接后**的最终源(0026:含 base/sdf.wgsl)。
    // 测试断言助手:失败即 panic 是预期行为(workspace `panic="warn"` + `-D warnings` 会拦,故局部 allow)。
    #[allow(clippy::panic)]
    fn assert_valid_wgsl(src: &str, what: &str) {
        let module = naga::front::wgsl::parse_str(src)
            .unwrap_or_else(|e| panic!("{what} WGSL 解析失败: {e}"));
        let mut validator = naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        );
        validator
            .validate(&module)
            .unwrap_or_else(|e| panic!("{what} WGSL 校验失败: {e}"));
    }

    #[test]
    fn glyph_shader_is_valid_wgsl() {
        assert_valid_wgsl(include_str!("shaders/base/glyph.wgsl"), "glyph");
    }

    #[test]
    fn panel_shader_is_valid_wgsl() {
        assert_valid_wgsl(
            &with_sdf(&[include_str!("shaders/base/panel.wgsl")]),
            "panel",
        );
    }

    #[test]
    fn rect_shader_is_valid_wgsl() {
        assert_valid_wgsl(&with_sdf(&[include_str!("shaders/base/rect.wgsl")]), "rect");
    }

    /// Plan 14 ②:图片纹理 quad shader(base/sdf + image)拼接产物合法(naga 解析 + 校验)。
    #[test]
    fn image_shader_is_valid_wgsl() {
        assert_valid_wgsl(
            &with_sdf(&[include_str!("shaders/base/image.wgsl")]),
            "image",
        );
    }

    /// 0026/Plan 11:markdown widget pipeline = base/sdf + box + rule + rule_cat + widget 拼接合法。
    /// 须与实际 `widget_pipeline` 的 include 列表一致(widget.wgsl 调 `md_rule_cat`,故含 rule_cat)。
    #[test]
    fn markdown_widget_shader_is_valid_wgsl() {
        assert_valid_wgsl(
            &with_sdf(&[
                include_str!("shaders/markdown/box.wgsl"),
                include_str!("shaders/markdown/rule.wgsl"),
                include_str!("shaders/markdown/rule_cat.wgsl"),
                include_str!("shaders/markdown/widget.wgsl"),
            ]),
            "markdown-widget",
        );
    }

    /// base/sdf.wgsl 单独也应是合法 WGSL(纯函数库,无 entry)。
    #[test]
    fn base_sdf_lib_is_valid_wgsl() {
        assert_valid_wgsl(super::SDF_LIB, "base/sdf");
    }
}
