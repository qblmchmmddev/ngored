use crossterm::event::KeyCode;
use log::{LevelFilter, debug};
use ratatui::Frame;
use ratatui::text::Line;
use tui_logger::{LogFormatter, TuiLoggerLevelOutput, TuiWidgetEvent, TuiWidgetState};

use crate::{component::Component, ngored_error::NgoredError};

pub struct DebugFormatter;

impl LogFormatter for DebugFormatter {
    fn min_width(&self) -> u16 {
        4
    }

    fn format(&'_ self, _width: usize, evt: &tui_logger::ExtLogRecord) -> Vec<Line<'_>> {
        let mut lines = vec![];
        lines.push(Line::from(format!(
            "{}:{}:{}",
            evt.timestamp.format("%H:%M:%S"),
            evt.file().unwrap_or(""),
            evt.msg()
        )));
        lines
    }
}

pub struct DebugComponent {
    state: TuiWidgetState,
}

impl DebugComponent {
    pub fn new() -> Self {
        DebugComponent {
            state: TuiWidgetState::new().set_default_display_level(LevelFilter::Debug),
        }
    }
}

impl Component for DebugComponent {
    async fn handle_key_press(&mut self, code: KeyCode) -> Result<(), NgoredError> {
        match code {
            KeyCode::Char('j') => self.state.transition(TuiWidgetEvent::NextPageKey),
            KeyCode::Char('k') => self.state.transition(TuiWidgetEvent::PrevPageKey),
            KeyCode::Esc => self.state.transition(TuiWidgetEvent::EscapeKey),
            _ => debug!("{}", code),
        }
        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        use ratatui::widgets::{Block, Widget};
        use tui_logger::TuiLoggerWidget;

        let area = frame.area();
        let buf = frame.buffer_mut();
        TuiLoggerWidget::default()
            .block(Block::bordered())
            .formatter(Box::new(DebugFormatter))
            .output_level(Some(TuiLoggerLevelOutput::Abbreviated))
            .state(&self.state)
            .render(area, buf);
    }
}
