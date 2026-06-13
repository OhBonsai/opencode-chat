//! atlas(M8)— glyph 字形图集:一张纹理 + UV 表 + shelf 装箱(Plan1)。
//!
//! 字形位图由平台侧(wasm:OffscreenCanvas)光栅化后传入,这里负责装箱进单张 GPU
//! 纹理并记录 UV。Plan1 装满即停(无 LRU/多页),CJK/emoji 量级足够;Plan2 再扩。

use std::collections::HashMap;

/// 图集边长(px)。1024² RGBA = 4MB,Plan1 足够。
const ATLAS_SIZE: u32 = 1024;
/// 字形间留白,避免采样溢出。
const PAD: u32 = 1;

/// 单页字形图集。
pub struct GlyphAtlas {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    sampler: wgpu::Sampler,
    /// shelf 装箱游标。
    cursor_x: u32,
    cursor_y: u32,
    shelf_h: u32,
    full: bool,
    /// 字形 key(grapheme cluster)→ UV [u0,v0,u1,v1]。
    map: HashMap<String, [f32; 4]>,
}

impl GlyphAtlas {
    pub fn new(device: &wgpu::Device) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("glyph-atlas"),
            size: wgpu::Extent3d {
                width: ATLAS_SIZE,
                height: ATLAS_SIZE,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("glyph-sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        Self {
            texture,
            view,
            sampler,
            cursor_x: PAD,
            cursor_y: PAD,
            shelf_h: 0,
            full: false,
            map: HashMap::new(),
        }
    }

    pub fn view(&self) -> &wgpu::TextureView {
        &self.view
    }

    pub fn sampler(&self) -> &wgpu::Sampler {
        &self.sampler
    }

    pub fn has(&self, key: &str) -> bool {
        self.map.contains_key(key)
    }

    pub fn uv(&self, key: &str) -> Option<[f32; 4]> {
        self.map.get(key).copied()
    }

    /// 把一张 RGBA8 位图装箱进图集并记录 UV。已存在或装满则忽略。
    pub fn upload(&mut self, queue: &wgpu::Queue, key: &str, rgba: &[u8], w: u32, h: u32) {
        if self.full || self.has(key) || w == 0 || h == 0 || w > ATLAS_SIZE {
            return;
        }
        // 换行到新 shelf。
        if self.cursor_x + w + PAD > ATLAS_SIZE {
            self.cursor_y += self.shelf_h + PAD;
            self.cursor_x = PAD;
            self.shelf_h = 0;
        }
        if self.cursor_y + h + PAD > ATLAS_SIZE {
            self.full = true;
            tracing::warn!(target: "M8", "glyph atlas 已满,丢弃字形 {key:?}");
            return;
        }
        let (x, y) = (self.cursor_x, self.cursor_y);
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d { x, y, z: 0 },
                aspect: wgpu::TextureAspect::All,
            },
            rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(w * 4),
                rows_per_image: Some(h),
            },
            wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
        );
        self.cursor_x += w + PAD;
        self.shelf_h = self.shelf_h.max(h);
        let s = ATLAS_SIZE as f32;
        self.map.insert(
            key.to_owned(),
            [
                x as f32 / s,
                y as f32 / s,
                (x + w) as f32 / s,
                (y + h) as f32 / s,
            ],
        );
    }
}
