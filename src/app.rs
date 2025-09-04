use std::time::Duration;

use crossterm::event::{Event, EventStream, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{DefaultTerminal, Frame};
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio_stream::StreamExt;

#[cfg(debug_assertions)]
use crate::component::debug::DebugComponent;

use crate::{
    component::{Component, sublist::SublistComponent},
    ngored_error::NgoredError,
};

pub enum AppEvent {
    Quit,
    Draw,
    #[cfg(debug_assertions)]
    ToggleShowDebug,
}
pub struct App {
    #[cfg(debug_assertions)]
    show_debug: bool,
    #[cfg(debug_assertions)]
    debug_component: DebugComponent,
    running: bool,
    app_event_sender: Sender<AppEvent>,
    app_event_receiver: Receiver<AppEvent>,
    sublist: SublistComponent,
}

impl App {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel(100);
        Self {
            #[cfg(debug_assertions)]
            debug_component: DebugComponent::new(),
            #[cfg(debug_assertions)]
            show_debug: false,
            running: true,
            sublist: SublistComponent::new(sender.clone()),
            app_event_sender: sender,
            app_event_receiver: receiver,
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
                        terminal.draw(|f| self.debug_component.draw(f))?;
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

    fn draw(&mut self, frame: &mut Frame) {
        self.sublist.draw(frame);
    }

    async fn handle_event(&mut self, event: &Event) -> Result<(), NgoredError> {
        match event {
            Event::Key(KeyEvent {
                kind: KeyEventKind::Press,
                code: KeyCode::Char('q'),
                ..
            }) => self.app_event_sender.send(AppEvent::Quit).await?,
            Event::Key(KeyEvent {
                kind: KeyEventKind::Press,
                code: KeyCode::Char('`'),
                ..
            }) => {
                self.app_event_sender
                    .send(AppEvent::ToggleShowDebug)
                    .await?
            }
            _ => {
                #[cfg(debug_assertions)]
                if self.show_debug {
                    self.debug_component.handle_event(event).await?;
                } else {
                    self.sublist.handle_event(event).await?;
                }

                #[cfg(not(debug_assertions))]
                self.sublist.handle_key_press(code).await?;
            }
        }
        Ok(())
    }
}
