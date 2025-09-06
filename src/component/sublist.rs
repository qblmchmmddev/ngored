use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    layout::{Constraint, Flex, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, BorderType, List, ListState, Paragraph, StatefulWidget, Widget},
};
use tokio::sync::mpsc::Sender;
use tui_input::{Input, backend::crossterm::EventHandler};

use crate::{app::AppEvent, component::Component, config::Config, ngored_error::NgoredError};

pub struct SublistComponent {
    app_event_sender: Sender<AppEvent>,
    subs: Vec<String>,
    list_state: ListState,
    adding: bool,
    sub_input: Input,
}

impl SublistComponent {
    pub fn new(subs: Vec<String>, app_event_sender: Sender<AppEvent>) -> Self {
        SublistComponent {
            app_event_sender,
            subs: subs,
            list_state: ListState::default().with_selected(Some(0)),
            adding: false,
            sub_input: Input::default(),
        }
    }
}

impl Component for SublistComponent {
    async fn handle_event(&mut self, event: &Event) -> Result<(), NgoredError> {
        if self.adding {
            match event {
                Event::Key(KeyEvent {
                    code: KeyCode::Esc,
                    kind: KeyEventKind::Press,
                    ..
                }) => {
                    self.adding = false;
                    self.sub_input.reset();
                    self.app_event_sender.send(AppEvent::Draw).await?;
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Enter,
                    kind: KeyEventKind::Press,
                    ..
                }) => {
                    self.adding = false;
                    let new_sub = self.sub_input.value_and_reset();
                    if !new_sub.is_empty() && !self.subs.contains(&new_sub) {
                        self.subs.push(new_sub);
                        Config::new(self.subs.clone()).save();
                        if self.list_state.selected().is_none() {
                            self.list_state.select(Some(0));
                        }
                    }
                    self.app_event_sender.send(AppEvent::Draw).await?;
                }
                _ => {
                    if self.sub_input.handle_event(event).is_some() {
                        self.app_event_sender.send(AppEvent::Draw).await?;
                    }
                }
            }
        } else {
            match event {
                Event::Key(KeyEvent {
                    code,
                    kind: KeyEventKind::Press,
                    ..
                }) => match code {
                    KeyCode::Char('j') => {
                        self.list_state.select_next();
                        self.app_event_sender.send(AppEvent::Draw).await?;
                    }
                    KeyCode::Char('k') => {
                        self.list_state.select_previous();
                        self.app_event_sender.send(AppEvent::Draw).await?;
                    }
                    KeyCode::Char('a') => {
                        self.adding = true;
                        self.app_event_sender.send(AppEvent::Draw).await?;
                    }
                    KeyCode::Char('d') => {
                        if let Some(selected_index) = self.list_state.selected() {
                            self.subs.remove(selected_index);
                            Config::new(self.subs.clone()).save();
                        }
                        self.app_event_sender.send(AppEvent::Draw).await?;
                    }
                    KeyCode::Char('l') => {
                        if let Some(selected_index) = self.list_state.selected() {
                            if let Some(sub) = self.subs.get(selected_index) {
                                self.app_event_sender
                                    .send(AppEvent::OpenPostList(sub.clone()))
                                    .await?;
                            }
                        }
                    }
                    _ => {}
                },
                _ => {}
            }
        }
        Ok(())
    }

    fn draw(&mut self, frame: &mut ratatui::Frame) {
        let area = frame.area();
        let buf = frame.buffer_mut();
        let selected_style = Style::new()
            .bg(Color::DarkGray)
            .add_modifier(Modifier::BOLD);
        let list = List::new(self.subs.clone())
            .highlight_style(selected_style)
            .block(
                Block::bordered()
                    .border_type(BorderType::Rounded)
                    .title("Sublist"),
            );
        StatefulWidget::render(list, area, buf, &mut self.list_state);
        if self.adding {
            let popup_block = Block::bordered().title("Add New Sub");

            let [center_vertical] = Layout::vertical([Constraint::Length(3)])
                .flex(Flex::Center)
                .areas(area);
            let [center] = Layout::horizontal([Constraint::Percentage(75)])
                .flex(Flex::Center)
                .areas(center_vertical);
            Paragraph::new(self.sub_input.value())
                .block(popup_block)
                .render(center, buf);
            let scroll = self
                .sub_input
                .visual_scroll(center.width.max(3) as usize - 3);
            let x = self.sub_input.visual_cursor().max(scroll) - scroll + 1;
            frame.set_cursor_position((center.x + x as u16, center.y + 1));
        }
    }
}
