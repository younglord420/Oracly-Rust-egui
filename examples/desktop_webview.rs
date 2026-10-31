//! # Desktop webview integration
//!
//! Opens a native window with a [wry](https://docs.rs/wry) WebView and loads the local HTML in
//! `desktop_webview.html`. The page explains how to host an egui-charts WASM/demo build inside a
//! desktop shell (Tauri, custom wry, and similar). The smoke path needs no network.
//!
//! Native only. On Linux install WebKitGTK 4.1 development packages, for example:
//! `libwebkit2gtk-4.1-dev` on Debian/Ubuntu.
//!
//! Run with:
//! ```sh
//! cargo run --example desktop_webview --features desktop-webview
//! ```

#[cfg(target_arch = "wasm32")]
fn main() {
    eprintln!("desktop_webview is a native-only example (wry + tao).");
}

#[cfg(not(target_arch = "wasm32"))]
fn main() -> wry::Result<()> {
    use tao::{
        event::{Event, WindowEvent},
        event_loop::{ControlFlow, EventLoop},
        window::WindowBuilder,
    };
    use wry::WebViewBuilder;

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("egui-charts: desktop webview")
        .with_inner_size(tao::dpi::LogicalSize::new(880.0, 720.0))
        .build(&event_loop)
        .expect("create window");

    let html = include_str!("desktop_webview.html");
    let builder = WebViewBuilder::new().with_html(html);

    // Keep the webview alive for the event loop.
    #[cfg(any(
        target_os = "windows",
        target_os = "macos",
        target_os = "ios",
        target_os = "android"
    ))]
    let _webview = builder.build(&window)?;
    #[cfg(not(any(
        target_os = "windows",
        target_os = "macos",
        target_os = "ios",
        target_os = "android"
    )))]
    let _webview = {
        use tao::platform::unix::WindowExtUnix;
        use wry::WebViewBuilderExtUnix;
        let vbox = window.default_vbox().expect("tao gtk box");
        builder.build_gtk(vbox)?
    };

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        if let Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } = event
        {
            *control_flow = ControlFlow::Exit;
        }
    });
}
