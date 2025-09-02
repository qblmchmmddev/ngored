use std::time::Duration;

use crossterm::event::{self, Event, EventStream, KeyEventKind};
#[cfg(debug_assertions)]
use log::LevelFilter;
use log::debug;
use ratatui::{
    DefaultTerminal, Frame,
    widgets::{Paragraph, Widget},
};
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio_stream::StreamExt;
#[cfg(debug_assertions)]
use tui_logger::TuiWidgetState;

use crate::ngored_error::NgoredError;

pub enum AppEvent {
    Quit,
    Draw,
    #[cfg(debug_assertions)]
    ToggleShowDebug,
}
pub struct App {
    running: bool,
    app_event_sender: Sender<AppEvent>,
    app_event_receiver: Receiver<AppEvent>,
    #[cfg(debug_assertions)]
    log_state: TuiWidgetState,
    #[cfg(debug_assertions)]
    show_debug: bool,
}

impl App {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel(100);
        Self {
            running: true,
            app_event_sender: sender,
            app_event_receiver: receiver,
            #[cfg(debug_assertions)]
            log_state: TuiWidgetState::new().set_default_display_level(LevelFilter::Debug),
            #[cfg(debug_assertions)]
            show_debug: false,
        }
    }

    pub async fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<(), NgoredError> {
        let mut events = EventStream::new();
        terminal.draw(|f| self.draw(f))?;

        #[cfg(debug_assertions)]
        let mut interval = {
            let period = Duration::from_secs_f32(1.0 / 30.0);
            tokio::time::interval(period)
        };

        #[cfg(debug_assertions)]
        while self.running {
            tokio::select! {
                Some(Ok(event)) = events.next() => self.handle_event(&event).await?,
                Some(app_event) = self.app_event_receiver.recv() => self.handle_app_event(app_event, terminal).await?,
                _ = interval.tick() => {
                    if self.show_debug {
                        terminal.draw(|f| self.draw_logger(f))?;
                    }
                }
            }
        }

        #[cfg(not(debug_assertions))]
        while self.running {
            tokio::select! {
                Some(Ok(event)) = events.next() => self.handle_event(&event).await?,
                Some(app_event) = self.app_event_receiver.recv() => self.handle_app_event(app_event, terminal).await?,
            }
        }
        Ok(())
    }

    async fn handle_app_event(
        &mut self,
        app_event: AppEvent,
        terminal: &mut DefaultTerminal,
    ) -> Result<(), NgoredError> {
        match app_event {
            AppEvent::Quit => self.running = false,
            AppEvent::Draw => {
                terminal.draw(|frame| self.draw(frame))?;
            }
            #[cfg(debug_assertions)]
            AppEvent::ToggleShowDebug => {
                self.show_debug = !self.show_debug;
                self.app_event_sender.send(AppEvent::Draw).await?;
            }
        };
        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        let area = frame.area();
        let buf = frame.buffer_mut();
        Paragraph::new("Hello, world!").render(area, buf);
    }

    #[cfg(debug_assertions)]
    fn draw_logger(&self, frame: &mut Frame) {
        use ratatui::widgets::Block;
        use tui_logger::TuiLoggerWidget;

        let area = frame.area();
        let buf = frame.buffer_mut();
        TuiLoggerWidget::default()
            .block(Block::bordered())
            .state(&self.log_state)
            .render(area, buf);
    }

    async fn handle_event(&mut self, event: &Event) -> Result<(), NgoredError> {
        match event {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_press(key_event.code).await?
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_key_press(&mut self, code: event::KeyCode) -> Result<(), NgoredError> {
        match code {
            event::KeyCode::Char(char) => match char {
                'q' => self.app_event_sender.send(AppEvent::Quit).await?,
                #[cfg(debug_assertions)]
                '`' => {
                    self.app_event_sender
                        .send(AppEvent::ToggleShowDebug)
                        .await?
                }
                _ => {
                    debug!("{} press", char);
                }
            },
            _ => {}
        }
        Ok(())
    }
}
