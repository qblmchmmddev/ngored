use crossterm::event::Event;
use ratatui::Frame;

use crate::ngored_error::NgoredError;

#[cfg(debug_assertions)]
pub mod debug;

pub mod postdetail;
pub mod postlist;
pub mod sublist;

pub trait Component {
    async fn handle_event(&mut self, event: &Event) -> Result<(), NgoredError> {
        let _ = event;
        Ok(())
    }

    fn draw(&mut self, frame: &mut Frame) {
        let _ = frame;
    }
}
