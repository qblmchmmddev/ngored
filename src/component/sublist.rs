use crossterm::event::KeyCode;
use ratatui::{
    style::{Color, Modifier, Style},
    widgets::{Block, BorderType, List, ListState, StatefulWidget},
};
use tokio::sync::mpsc::Sender;

use crate::{app::AppEvent, component::Component, ngored_error::NgoredError};

pub struct SublistComponent {
    app_event_sender: Sender<AppEvent>,
    subs: Vec<String>,
    list_state: ListState,
}

impl SublistComponent {
    pub fn new(app_event_sender: Sender<AppEvent>) -> Self {
        SublistComponent {
            app_event_sender,
            subs: vec!["indonesia".to_string(), "indotech".to_string()],
            list_state: ListState::default().with_selected(Some(0)),
        }
    }
}

impl Component for SublistComponent {
    async fn handle_key_press(&mut self, code: KeyCode) -> Result<(), NgoredError> {
        match code {
            KeyCode::Char('j') => {
                self.list_state.select_next();
                self.app_event_sender.send(AppEvent::Draw).await?;
            }
            KeyCode::Char('k') => {
                self.list_state.select_previous();
                self.app_event_sender.send(AppEvent::Draw).await?;
            }
            _ => {}
        }
        Ok(())
    }
    fn draw(&mut self, frame: &mut ratatui::Frame) {
        let area = frame.area();
        let buf = frame.buffer_mut();
        let selected_style = Style::new().bg(Color::Blue).add_modifier(Modifier::BOLD);
        let list = List::new(self.subs.clone())
            .highlight_style(selected_style)
            .block(
                Block::bordered()
                    .border_type(BorderType::Rounded)
                    .title("Sublist"),
            );
        StatefulWidget::render(list, area, buf, &mut self.list_state);
    }
}
