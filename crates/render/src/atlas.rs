//! atlas(M8)— glyph SDF 图集(Plan 3 K,演进自 Plan 1/2 的位图 atlas)。
//!
//! 两级稀疏间接(0011 §3.3③):**glyph-key → 定长 SDF tile 槽 → 多页 R8 纹理数组**。
//! - tile = 固定 `TILE_PX` 的 **R8 单通道距离场**(SDF 与缩放无关 → 一张 tile 任意 zoom 清晰)。
//! - 槽位分配 + LRU 淘汰是纯 CPU 逻辑([`TileAllocator`],native 可测),与 wgpu 解耦。
//! - 满了开新页(纹理数组层),可见字形钉住不淘汰。
//!
//! SDF 由平台侧(web TinySDF/ESDT)生成后上传;本 crate 不依赖 web-sys。

use std::collections::HashMap;

/// 单个 SDF tile 边长(px)。SDF 缩放无关,一档够正文;极大字号的 MSDF 触发条件见 0011 §6。
pub const TILE_PX: u32 = 128; // 64→128:源分辨率 ×2,大字更锐(须与 layout-bridge TILE_PX 一致)
/// tile 内字形四周留白(px)= SDF 半径裕量。**须与 `layout-bridge.ts` 的 `SDF_BUFFER` 一致**;
/// MSDF 源(0015)据此从方格几何反推 pen 原点 / em 盒。
pub const SDF_BUFFER: u32 = 8;
/// 每页边长 = 多少个 tile(`PAGE_TILES²` 个/页)。
const PAGE_TILES: u32 = 8;
/// 每页边长(px)。
const PAGE_PX: u32 = TILE_PX * PAGE_TILES;
/// 最大页数(纹理数组层数上限)。
const MAX_PAGES: u32 = 8;

/// tile 在图集里的槽位。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Slot {
    pub page: u32,
    pub col: u32,
    pub row: u32,
}

impl Slot {
    /// 该槽在所属页内的归一化 UV `[u0,v0,u1,v1]`(留 1px 内缩防采样溢出)。
    pub fn uv(self) -> [f32; 4] {
        let p = PAGE_PX as f32;
        let x0 = (self.col * TILE_PX) as f32;
        let y0 = (self.row * TILE_PX) as f32;
        let t = TILE_PX as f32;
        [
            (x0 + 0.5) / p,
            (y0 + 0.5) / p,
            (x0 + t - 0.5) / p,
            (y0 + t - 0.5) / p,
        ]
    }
}

/// 分配结果:槽位 + 是否首次分配(首次需上传 tile)。
#[derive(Debug, Clone, Copy)]
pub struct Alloc {
    pub slot: Slot,
    pub is_new: bool,
}

/// 定长 tile 槽分配器 + LRU(纯 CPU,0011 §3.3③)。
///
/// O(1) 分配、零碎片(定长槽);满了开页;再满则淘汰最久未用且**未钉住**的槽。
pub struct TileAllocator {
    /// glyph-key → 槽位。
    map: HashMap<String, Slot>,
    /// 槽位 → glyph-key(反查,淘汰用)。
    occupied: HashMap<(u32, u32, u32), String>,
    /// 已开页数。
    pages: u32,
    /// 下一个未用过的空槽(顺序填充,填满才走淘汰)。
    next: Option<Slot>,
    /// LRU 顺序:队尾最近用;淘汰从队首。
    lru: Vec<String>,
    /// 本帧钉住(可见)的 key,不淘汰。
    pinned: std::collections::HashSet<String>,
    /// 累计淘汰次数(可观测;持续增长 = thrash)。
    evictions: u64,
}

impl Default for TileAllocator {
    fn default() -> Self {
        Self::new()
    }
}

impl TileAllocator {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
            occupied: HashMap::new(),
            pages: 0,
            next: Some(Slot {
                page: 0,
                col: 0,
                row: 0,
            }),
            lru: Vec::new(),
            pinned: std::collections::HashSet::new(),
            evictions: 0,
        }
    }

    /// 容量(当前已开页数 × 每页槽数)。
    pub fn capacity(&self) -> usize {
        (self.pages * PAGE_TILES * PAGE_TILES) as usize
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// 累计淘汰次数(可观测)。
    pub fn evictions(&self) -> u64 {
        self.evictions
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// 清空本帧钉住集(每帧开头调一次)。
    pub fn begin_frame(&mut self) {
        self.pinned.clear();
    }

    /// 钉住一个 key(本帧可见,不可淘汰)。
    pub fn pin(&mut self, key: &str) {
        self.pinned.insert(key.to_owned());
    }

    /// 取或分配 key 的槽位。命中则刷新 LRU;未命中则分配(空槽优先,否则开页,否则淘汰 LRU)。
    pub fn alloc(&mut self, key: &str) -> Alloc {
        if let Some(&slot) = self.map.get(key) {
            self.touch(key);
            return Alloc {
                slot,
                is_new: false,
            };
        }
        let slot = self.free_slot();
        self.map.insert(key.to_owned(), slot);
        self.occupied
            .insert((slot.page, slot.col, slot.row), key.to_owned());
        self.lru.push(key.to_owned());
        Alloc { slot, is_new: true }
    }

    fn touch(&mut self, key: &str) {
        if let Some(pos) = self.lru.iter().position(|k| k == key) {
            let k = self.lru.remove(pos);
            self.lru.push(k);
        }
    }

    /// 找一个可用槽:顺序空槽 → 开新页 → 淘汰 LRU(未钉住)。
    fn free_slot(&mut self) -> Slot {
        if let Some(slot) = self.next {
            self.pages = self.pages.max(slot.page + 1); // 实际放进某页才算开页(惰性)
            self.advance_next();
            return slot;
        }
        // 满:淘汰最久未用且未钉住的。
        if let Some(pos) = self.lru.iter().position(|k| !self.pinned.contains(k)) {
            let victim = self.lru.remove(pos);
            if let Some(slot) = self.map.remove(&victim) {
                self.occupied.remove(&(slot.page, slot.col, slot.row));
                self.evictions += 1;
                return slot;
            }
        }
        // 全钉住(极端):复用第 0 槽(退化,实践不会到)。
        Slot {
            page: 0,
            col: 0,
            row: 0,
        }
    }

    /// 推进顺序填充游标;填满当前所有页则尝试开页;再满则置 None(转淘汰)。
    fn advance_next(&mut self) {
        let Some(cur) = self.next else { return };
        let mut col = cur.col + 1;
        let mut row = cur.row;
        let mut page = cur.page;
        if col >= PAGE_TILES {
            col = 0;
            row += 1;
        }
        if row >= PAGE_TILES {
            row = 0;
            page += 1;
        }
        if page >= MAX_PAGES {
            self.next = None; // 所有页满 → 转淘汰
            return;
        }
        self.next = Some(Slot { page, col, row });
    }
}

/// SDF 图集:R8 多页纹理数组 + [`TileAllocator`]。wgpu 侧;槽位逻辑全在 allocator。
pub struct SdfAtlas {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    sampler: wgpu::Sampler,
    alloc: TileAllocator,
}

impl SdfAtlas {
    pub fn new(device: &wgpu::Device) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("sdf-atlas"),
            size: wgpu::Extent3d {
                width: PAGE_PX,
                height: PAGE_PX,
                depth_or_array_layers: MAX_PAGES,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            // RGBA8:单色源(位图/TinySDF)把值塞 `.r`(shader 读 `.r`),彩色 emoji(kind=3)存真彩
            // (0015 §7.2)。绑定层 `texture_2d_array<f32>` 对 R8/RGBA8 通吃,故只改格式 + 上传字节数。
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            ..Default::default()
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("sdf-sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        Self {
            texture,
            view,
            sampler,
            alloc: TileAllocator::new(),
        }
    }

    pub fn view(&self) -> &wgpu::TextureView {
        &self.view
    }

    pub fn sampler(&self) -> &wgpu::Sampler {
        &self.sampler
    }

    /// 每帧开头:清钉住集。
    pub fn begin_frame(&mut self) {
        self.alloc.begin_frame();
    }

    /// 占用 / 容量 / 累计淘汰(可观测;used≈cap 且 evict 持续增长 = thrash)。
    pub fn stats(&self) -> (usize, usize, u64) {
        (
            self.alloc.len(),
            self.alloc.capacity(),
            self.alloc.evictions(),
        )
    }

    /// 钉住可见 key(本帧不淘汰)。
    pub fn pin(&mut self, key: &str) {
        self.alloc.pin(key);
    }

    /// 取/分配 key 的槽。`is_new` 时调用方需 [`upload`](Self::upload) 该 tile。
    pub fn alloc(&mut self, key: &str) -> Alloc {
        self.alloc.alloc(key)
    }

    /// 上传一张 `TILE_PX²` 的 RGBA8 tile 到指定槽(单色源 `.r` splat / 彩色 emoji 真彩,0015 §7.2)。
    pub fn upload(&mut self, queue: &wgpu::Queue, slot: Slot, sdf: &[u8]) {
        let need = (TILE_PX * TILE_PX * 4) as usize;
        if sdf.len() < need {
            tracing::warn!(target: "M8", "tile 尺寸不足({} < {need}),跳过", sdf.len());
            return;
        }
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: slot.col * TILE_PX,
                    y: slot.row * TILE_PX,
                    z: slot.page,
                },
                aspect: wgpu::TextureAspect::All,
            },
            &sdf[..need],
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(TILE_PX * 4), // RGBA8 → 4 字节/像素
                rows_per_image: Some(TILE_PX),
            },
            wgpu::Extent3d {
                width: TILE_PX,
                height: TILE_PX,
                depth_or_array_layers: 1,
            },
        );
    }
}

/// 离线烘焙的 **MSDF 图集**(0015):RGBA8 多页**静态**纹理。与 [`SdfAtlas`] 不同——没有运行时
/// 分配/淘汰,UV 由 BMFont metrics 直给(平台侧持有);本结构只管纹理生命周期 + 上传。
///
/// 未加载时为 1×1 占位(`loaded()==false`),让 glyph 管线绑定始终合法;加载时 `init` 重建为
/// `w×h×pages` 的 RGBA D2Array,逐页 `upload_page` 灌入(JS 解码 PNG → RGBA 字节)。
pub struct MsdfAtlas {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    loaded: bool,
}

impl MsdfAtlas {
    fn make(
        device: &wgpu::Device,
        w: u32,
        h: u32,
        pages: u32,
    ) -> (wgpu::Texture, wgpu::TextureView) {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("msdf-atlas"),
            size: wgpu::Extent3d {
                width: w.max(1),
                height: h.max(1),
                depth_or_array_layers: pages.max(1),
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm, // 三通道 MSDF + a(可作覆盖)
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            ..Default::default()
        });
        (texture, view)
    }

    /// 1×1×1 占位(未加载)。
    pub fn dummy(device: &wgpu::Device) -> Self {
        let (texture, view) = Self::make(device, 1, 1, 1);
        Self {
            texture,
            view,
            loaded: false,
        }
    }

    pub fn view(&self) -> &wgpu::TextureView {
        &self.view
    }

    pub fn loaded(&self) -> bool {
        self.loaded
    }

    /// (重)建为 `w×h×pages` 的 RGBA D2Array(调用方随后逐页 `upload_page` + 重建 bind group)。
    pub fn init(&mut self, device: &wgpu::Device, w: u32, h: u32, pages: u32) {
        let (texture, view) = Self::make(device, w, h, pages);
        self.texture = texture;
        self.view = view;
        self.loaded = true;
    }

    /// 上传一整页 RGBA 像素(`w*h*4` 字节)到第 `page` 层。
    pub fn upload_page(&self, queue: &wgpu::Queue, page: u32, rgba: &[u8]) {
        let size = self.texture.size();
        let need = (size.width * size.height * 4) as usize;
        if rgba.len() < need {
            tracing::warn!(target: "M8", "MSDF page 尺寸不足({} < {need}),跳过", rgba.len());
            return;
        }
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: 0,
                    y: 0,
                    z: page,
                },
                aspect: wgpu::TextureAspect::All,
            },
            &rgba[..need],
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(size.width * 4),
                rows_per_image: Some(size.height),
            },
            wgpu::Extent3d {
                width: size.width,
                height: size.height,
                depth_or_array_layers: 1,
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alloc_is_stable_for_same_key() {
        let mut a = TileAllocator::new();
        let s1 = a.alloc("x");
        assert!(s1.is_new);
        let s2 = a.alloc("x");
        assert!(!s2.is_new, "同 key 第二次不该算新");
        assert_eq!(s1.slot, s2.slot);
        assert_eq!(a.len(), 1);
    }

    #[test]
    fn fills_then_opens_pages() {
        let mut a = TileAllocator::new();
        let per_page = (PAGE_TILES * PAGE_TILES) as usize;
        for i in 0..per_page {
            assert!(a.alloc(&format!("k{i}")).is_new);
        }
        assert_eq!(a.capacity(), per_page); // 第一页满
        a.alloc("overflow"); // 触发开页
        assert_eq!(a.capacity(), per_page * 2);
    }

    #[test]
    fn evicts_lru_when_full_but_keeps_pinned() {
        let mut a = TileAllocator::new();
        let total = (MAX_PAGES * PAGE_TILES * PAGE_TILES) as usize;
        for i in 0..total {
            a.alloc(&format!("k{i}"));
        }
        // 全满。钉住 k0(最久未用),它不该被淘汰。
        a.begin_frame();
        a.pin("k0");
        let slot_k0 = a.alloc("k0").slot; // touch k0
        a.alloc("new"); // 需淘汰:k0 被钉 → 淘汰 k1
        assert_eq!(a.alloc("k0").slot, slot_k0, "钉住的 k0 不该被淘汰");
        assert!(a.alloc("k1").is_new, "k1 应已被淘汰(需重新分配)");
    }

    #[test]
    fn slot_uv_within_page() {
        let s = Slot {
            page: 0,
            col: 0,
            row: 0,
        };
        let uv = s.uv();
        assert!(uv[0] > 0.0 && uv[2] < 1.0 && uv[0] < uv[2]);
    }
}
