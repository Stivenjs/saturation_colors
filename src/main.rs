mod app;
mod application;
mod domain;
mod error;
mod memory;
mod platform;
mod tray;

use app::ColorApp;

fn main() -> eframe::Result {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("saturation_colors=info"));
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 760.0])
            .with_min_inner_size([900.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Saturation Colors",
        options,
        Box::new(|creation_context| Ok(Box::new(ColorApp::new(creation_context)))),
    )
}
