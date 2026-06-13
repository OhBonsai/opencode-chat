//! observe — panic hook + tracing-wasm(testing §1)。
//!
//! 让 Rust panic 在浏览器 console 显示 backtrace,日志走 tracing→console。幂等(只装一次)。

use std::sync::Once;

static INIT: Once = Once::new();

/// 安装 panic hook 与 tracing 订阅(可安全多次调用)。
pub(crate) fn init() {
    INIT.call_once(|| {
        console_error_panic_hook::set_once();
        tracing_wasm::set_as_global_default();
    });
}
