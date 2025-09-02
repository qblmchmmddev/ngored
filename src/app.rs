use crossterm::event::{self, Event, KeyEventKind};
use log::debug;
use ratatui::{DefaultTerminal, Frame, widgets::Widget};

use crate::ngored_error::NgoredError;

pub struct App {
    exit: bool,
}

impl App {
    pub fn new() -> Self {
        Self { exit: false }
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<(), NgoredError> {
        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        #[cfg(debug_assertions)]
        {
            use ratatui::widgets::Block;
            use tui_logger::TuiLoggerWidget;

            let area = frame.area();
            let buf = frame.buffer_mut();
            TuiLoggerWidget::default()
                .block(Block::bordered())
                .render(area, buf);
        }
    }

    fn handle_events(&mut self) -> Result<(), NgoredError> {
        match event::read()? {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_press(key_event.code)
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_key_press(&mut self, code: event::KeyCode) {
        match code {
            event::KeyCode::Char(char) => match char {
                'q' => self.exit = true,
                _ => debug!("{} press", char),
            },
            _ => {}
        }
    }
}
