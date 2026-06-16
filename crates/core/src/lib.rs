//! infinite-chat-core — 平台无关的对话渲染内核(M2/M3/M5/M6/M13)。
//!
//! 设计铁律(见 spec/dev-practices.md §4):
//! - **CR1**:本 crate 零 `wasm-bindgen`/`web-sys`/`wgpu` 依赖,保 native 可测。
//! - **CR2**:网络/排版/时钟/渲染等平台能力一律走 [`seam`] 中的 trait 注入。
//! - **R8/R9**:不碰 `Instant::now`/裸 `rand`;时间以 `dt_ms` 注入,逐帧累加,
//!   保证录像重放确定性(见 [`record`])。
//! - **AR4**:`delta` 乐观追加必配 `message.part.updated` 全量对账(见 [`store`])。
//! - **AR7**:吐字单位 = grapheme cluster,不按码点切(见 [`smoother`])。
//! - **AR12**:未知事件/Part 类型 → `Ignored`,不 panic(见 [`protocol`])。
//!
//! 每帧编排见 [`app::Engine`]。

mod app;
mod camera;
mod content;
mod frame;
mod fsm;
mod nodes;
mod protocol;
mod record;
mod reveal;
mod seam;
mod smoother;
mod spatial;
mod store;
mod support;
mod theme;

pub use app::{Engine, FrameStats, TableStyle};
pub use camera::{Camera2D, Rect};
pub use content::{
    content_gate, parse_markdown, parse_markdown_nodes, plain, StyleRole, StyledSpan, TableRegion,
};
pub use frame::{FrameData, FrameGlyph, FramePanel, FrameRect, PANEL_AO, PANEL_GRID};
pub use fsm::{TurnStatus, TurnTracker};
pub use nodes::{glyph_key, Node, NodeKind, NodeTree};
pub use protocol::{
    decode, parse_snapshot, Envelope, Event, Part, ProtocolError, SnapshotMessage, TextPartData,
};
pub use record::{Player, Record, Recorder};
pub use reveal::{block_kind, is_structural, layout_gate, RevealUnit};
pub use seam::{
    Clock, Connection, LayoutEngine, LayoutResult, PlacedGlyph, RawEvent, RenderSink, TablePanel,
};
pub use smoother::{Revealed, Smoother};
pub use spatial::SpatialGrid;
pub use store::Store;
pub use support::{CollectSink, MonospaceLayout, NullSink};
