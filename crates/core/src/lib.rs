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
mod boxlayout;
mod camera;
mod codeblock;
mod content;
mod embed;
mod frame;
mod fsm;
mod highlight;
mod math;
mod nodes;
mod partrender;
mod partspecific;
mod protocol;
mod record;
mod resilience;
mod reveal;
mod seam;
mod shaderbox;
mod smoother;
mod spatial;
mod store;
mod support;
mod theme;

pub use app::{Engine, FrameStats, TableStyle, VisibleMessage, VisibleTextRun};
pub use camera::{Camera2D, Rect};
pub use content::{
    content_gate, parse_markdown, parse_markdown_embeds, parse_markdown_nodes, plain, EmbedRegion,
    StyleRole, StyledSpan, TableRegion,
};
pub use embed::{Embed, EmbedState};
pub use frame::{
    FrameData, FrameEmbed, FrameGlyph, FrameImage, FramePanel, FrameRect, FrameShaderBox,
    FrameWidget, PANEL_AO, PANEL_GRID, WIDGET_BOX, WIDGET_RULE, WIDGET_RULE_CAT,
};
pub use fsm::{next_status, Blocker, FsmInput, SessionStatus, TurnStatus, TurnTracker};
pub use math::{
    font_role, katex_font_base, layout_math, math_to_frame, MathGlyph, MathLayout, MathRule,
};
pub use nodes::{glyph_key, Node, NodeKind, NodeTree};
pub use partrender::{
    fallback_render, group_message_parts, is_context_tool, Bucket, PartKind, RenderCtx, RenderFn,
    RenderPart, RenderRegistry,
};
pub use partspecific::{default_registry, diff_parse_lines, DiffKind, DiffLine};
pub use protocol::{
    decode, parse_snapshot, Envelope, Event, Part, ProtocolError, SnapshotMessage, TextPartData,
};
pub use record::{Player, Record, Recorder};
pub use resilience::{is_quota_error, merge_ordered, should_bottom_out, temp_should_replace};
pub use reveal::{
    block_kind, is_nodespawn, is_structural, layout_gate, ordering_for, resolve_tree, GlyphPlan,
    Ordering, RevealScheduler, RevealUnit, TableStyleKind, DEFAULT_REVEAL_CPS,
};
pub use seam::{
    Clock, Connection, LayoutEngine, LayoutResult, MeasuredSize, PlacedGlyph, RawEvent, RenderSink,
    TablePanel,
};
pub use shaderbox::{
    shaderbox_exceeds_area_cap, IconId, ShaderId, ICON_COUNT, SHADERBOX_MAX_EDGE_PX,
    SHADERBOX_THROTTLE_MS,
};
pub use smoother::{Revealed, Smoother};
pub use spatial::SpatialGrid;
pub use store::{Role, Store};
pub use support::{
    push_event, CollectSink, EventQueue, MonospaceLayout, NullSink, QueueConnection,
};
