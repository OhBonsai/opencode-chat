//! clock — `Clock` seam 的 wasm 实现:`performance.now()`(R8 时间源)。

use opencode_chat_core::Clock;

pub(crate) struct WebClock {
    perf: web_sys::Performance,
}

impl WebClock {
    pub(crate) fn new() -> Option<Self> {
        let perf = web_sys::window()?.performance()?;
        Some(Self { perf })
    }
}

impl Clock for WebClock {
    fn now_ms(&self) -> f64 {
        self.perf.now()
    }
}
