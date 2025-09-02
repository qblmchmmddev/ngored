use crate::{app::App, ngored_error::NgoredError};

#[cfg(feature = "dhat-heap")]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

mod app;
mod component;
mod ngored_error;

#[tokio::main]
async fn main() -> Result<(), NgoredError> {
    #[cfg(feature = "dhat-heap")]
    let _profiler = dhat::Profiler::new_heap();
    #[cfg(debug_assertions)]
    {
        use log::debug;
        tui_logger::init_logger(log::LevelFilter::Trace)?;
        tui_logger::set_default_level(log::LevelFilter::Debug);
        debug!("App started")
    }

    let mut terminal = ratatui::init();
    let app_result = App::new().run(&mut terminal).await;

    ratatui::restore();

    app_result
}
