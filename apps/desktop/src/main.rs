use iced::{Application, Settings, Size};
use tracing::info;

mod app;
mod recording;
mod ui;

use app::FrameApp;

fn main() -> iced::Result {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter("info,frame=debug")
        .init();

    info!("Starting Frame v{}", env!("CARGO_PKG_VERSION"));

    // Run the application
    FrameApp::run(Settings {
        window: iced::window::Settings {
            size: Size::new(1200.0, 800.0),
            min_size: Some(Size::new(800.0, 600.0)),
            position: iced::window::Position::Centered,
            ..Default::default()
        },
        ..Default::default()
    })?;

    Ok(())
}
