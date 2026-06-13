//! opencode-chat-render — wgpu 渲染后端(M8 scene / M9 effects / M10 render)。
//!
//! 输入 core 的 [`FrameData`](opencode_chat_core::FrameData)(语义字形),输出像素。
//! 着色器淡入靠 `time - spawn_time` 在 GPU 算,CPU 零参与(0002 §5)。
//!
//! 铁律:CR3 后端用 trait 选择(见 [`RenderBackend`]);CR4 instance 平铺零拷贝
//! (见 [`scene::GpuInstance`]);本 crate 不依赖 web-sys,surface 由 wasm 注入。

mod atlas;
mod backend;
mod effects;
mod scene;

pub use atlas::GlyphAtlas;
pub use backend::{RenderBackend, RenderError, WebGpuBackend};
pub use effects::{EffectProfile, Globals};
pub use scene::{build_instances, GpuInstance};
