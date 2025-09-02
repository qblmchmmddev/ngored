use crossterm::event;
use ratatui::Frame;

use crate::ngored_error::NgoredError;

#[cfg(debug_assertions)]
pub mod debug;

pub trait Component {
    async fn handle_key_press(&mut self, code: event::KeyCode) -> Result<(), NgoredError> {
        let _ = code;
        Ok(())
    }
    fn draw(&self, frame: &mut Frame) {
        let _ = frame;
    }
}
