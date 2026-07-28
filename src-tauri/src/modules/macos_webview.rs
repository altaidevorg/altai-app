//! macOS WKWebView tweaks that must be applied at configuration time.
//!
//! Apple Intelligence Writing Tools (and the macOS 27 Siri AI selection
//! popover that rides on them) attach to every WKWebView by default. The
//! only supported opt-out is setting `writingToolsBehavior = .none` on the
//! `WKWebViewConfiguration` *before* the webview is created — it cannot be
//! changed later. See WWDC24-10168 / `WKWebViewConfiguration.writingToolsBehavior`.

use objc2::rc::Retained;
use objc2::runtime::NSObjectProtocol;
use objc2::{msg_send, sel};
use objc2_foundation::MainThreadMarker;
use objc2_web_kit::WKWebViewConfiguration;

/// Build a fresh `WKWebViewConfiguration` with Writing Tools fully disabled.
///
/// wry merges its own scheme handlers / data-store settings into whatever
/// configuration we pass via `WebviewWindowBuilder::with_webview_configuration`,
/// so this is safe to hand to every ALTAI window on macOS.
pub fn config_without_writing_tools() -> Retained<WKWebViewConfiguration> {
    let mtm = MainThreadMarker::new().expect("WKWebViewConfiguration requires the main thread");
    // SAFETY: MainThreadMarker proves we're on the main thread, which is the
    // only requirement for `WKWebViewConfiguration::new`.
    let config = unsafe { WKWebViewConfiguration::new(mtm) };
    // objc2-web-kit does not yet expose `setWritingToolsBehavior:` (macOS 15+),
    // so call it dynamically. NSWritingToolsBehaviorNone == -1.
    disable_writing_tools(&config);
    config
}

fn disable_writing_tools(config: &WKWebViewConfiguration) {
    // NSWritingToolsBehaviorNone
    const NONE: isize = -1;
    let sel = sel!(setWritingToolsBehavior:);
    if !config.respondsToSelector(sel) {
        // Pre-macOS 15: Writing Tools / Siri AI popover don't exist.
        return;
    }
    // SAFETY: selector exists on WKWebViewConfiguration (macOS 15+) and takes
    // a single NSInteger / NSWritingToolsBehavior argument.
    unsafe {
        let _: () = msg_send![config, setWritingToolsBehavior: NONE];
    }
}
