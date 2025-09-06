use std::{sync::Arc, time::Duration};

use crossterm::event::{Event, EventStream, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{DefaultTerminal, Frame};
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio_stream::StreamExt;

#[cfg(debug_assertions)]
use crate::component::debug::DebugComponent;

use crate::{
    component::{Component, postlist::PostlistComponent, sublist::SublistComponent},
    config::Config,
    ngored_error::NgoredError,
    reddit_api::RedditApi,
};

pub enum AppEvent {
    Quit,
    Draw,
    #[cfg(debug_assertions)]
    ToggleShowDebug,
    OpenPostList(String),
    ClosePostList,
}

pub enum Screen {
    Sublist,
    Postlist,
}

pub struct App {
    #[cfg(debug_assertions)]
    show_debug: bool,
    #[cfg(debug_assertions)]
    debug_component: DebugComponent,
    running: bool,
    app_event_sender: Sender<AppEvent>,
    app_event_receiver: Receiver<AppEvent>,
    current_screen: Screen,
    sublist: SublistComponent,
    postlist: PostlistComponent,
}

impl App {
    pub fn new() -> Self {
        let reddit_api = Arc::new(RedditApi::new());
        let config = Config::load();
        let (sender, receiver) = mpsc::channel(100);
        Self {
            #[cfg(debug_assertions)]
            debug_component: DebugComponent::new(),
            #[cfg(debug_assertions)]
            show_debug: false,
            running: true,
            current_screen: Screen::Sublist,
            sublist: SublistComponent::new(config.subs, sender.clone()),
            postlist: PostlistComponent::new(reddit_api.clone(), sender.clone()),
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
            AppEvent::OpenPostList(sub) => {
                self.postlist.load(sub);
                self.current_screen = Screen::Postlist;
                self.app_event_sender.send(AppEvent::Draw).await?;
            }
            AppEvent::ClosePostList => {
                self.current_screen = Screen::Sublist;
                self.app_event_sender.send(AppEvent::Draw).await?;
            }
        };
        Ok(())
    }

    fn draw(&mut self, frame: &mut Frame) {
        match self.current_screen {
            Screen::Sublist => self.sublist.draw(frame),
            Screen::Postlist => self.postlist.draw(frame),
        }
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
                    match self.current_screen {
                        Screen::Sublist => self.sublist.handle_event(event).await?,
                        Screen::Postlist => self.postlist.handle_event(event).await?,
                    };
                }

                #[cfg(not(debug_assertions))]
                match self.current_screen {
                    Screen::Sublist => self.sublist.handle_event(event).await?,
                    Screen::Postlist => self.postlist.handle_event(event).await?,
                };
            }
        }
        Ok(())
    }
}
