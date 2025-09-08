use std::{
    borrow::Cow,
    sync::{Arc, RwLock},
};

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    layout::{Alignment, Constraint, Flex, Layout},
    style::{Color, Modifier, Stylize},
    text::{Line, Text},
    widgets::{Block, BorderType, Paragraph, StatefulWidget, Widget},
};
use tokio::sync::mpsc::Sender;
use tui_widget_list::{ListBuilder, ListState, ListView};

use crate::{
    app::AppEvent, component::Component, model::post::Post, ngored_error::NgoredError,
    reddit_api::RedditApi,
};

pub struct PostlistState {
    loading: bool,
    sub: String,
    items: Vec<Post>,
    list_state: ListState,
}

pub struct PostlistComponent {
    reddit_api: Arc<RedditApi>,
    app_event_sender: Sender<AppEvent>,
    state: Arc<RwLock<PostlistState>>,
}

impl PostlistComponent {
    pub fn new(reddit_api: Arc<RedditApi>, app_event_sender: Sender<AppEvent>) -> Self {
        let state = PostlistState {
            loading: false,
            sub: String::default(),
            items: Vec::default(),
            list_state: ListState::default(),
        };
        Self {
            reddit_api,
            app_event_sender,
            state: Arc::new(RwLock::new(state)),
        }
    }

    pub fn load(&mut self, sub: String) {
        {
            let state = self.state.read().unwrap();
            if state.sub == sub || state.loading {
                return;
            }
        }
        self.state.write().unwrap().sub = sub.clone();

        tokio::spawn({
            let state = self.state.clone();
            let reddit_api = self.reddit_api.clone();
            let app_event_sender = self.app_event_sender.clone();
            async move {
                {
                    let mut state = state.write().unwrap();
                    state.loading = true;
                    state.items.clear();
                }
                app_event_sender.send(AppEvent::Draw).await.unwrap();

                let res = { reddit_api.get_posts(&sub).await };

                {
                    let mut state = state.write().unwrap();
                    state.items = res
                        .as_listing()
                        .children
                        .into_iter()
                        .map(|i| Post::from(i.as_post()))
                        .collect();
                    state.loading = false;
                    state.list_state.select(Some(0));
                }
                app_event_sender.send(AppEvent::Draw).await.unwrap();
            }
        });
    }
}

impl Component for PostlistComponent {
    async fn handle_event(&mut self, event: &Event) -> Result<(), NgoredError> {
        match event {
            Event::Key(KeyEvent {
                code: KeyCode::Char(char),
                kind: KeyEventKind::Press,
                ..
            }) => match char {
                'h' => {
                    self.state.write().unwrap().list_state.select(Some(0));
                    self.app_event_sender.send(AppEvent::ClosePostList).await?;
                }
                'j' => {
                    self.state.write().unwrap().list_state.next();
                    self.app_event_sender.send(AppEvent::Draw).await?
                }
                'k' => {
                    self.state.write().unwrap().list_state.previous();
                    self.app_event_sender.send(AppEvent::Draw).await?
                }
                'l' => {
                    let state = self.state.read().unwrap();
                    if let Some(selected_index) = state.list_state.selected {
                        self.app_event_sender
                            .send(AppEvent::OpenPostDetail(
                                state.items[selected_index].clone(),
                            ))
                            .await?
                    }
                }
                _ => {}
            },
            _ => {}
        }
        Ok(())
    }

    fn draw(&mut self, frame: &mut ratatui::Frame) {
        let area = frame.area();
        let buf = frame.buffer_mut();
        let block = Block::bordered()
            .border_type(BorderType::Rounded)
            .title(self.state.read().unwrap().sub.clone());
        if self.state.read().unwrap().loading {
            block.render(area, buf);
            let text = Text::raw("Loading...");
            let [area] = Layout::vertical([Constraint::Length(text.height() as u16)])
                .flex(Flex::Center)
                .areas(area);
            Paragraph::new(text)
                .alignment(Alignment::Center)
                .render(area, buf);
        } else {
            let posts = self.state.read().unwrap().items.clone();
            let builder = ListBuilder::new(|ctx| {
                let width = ctx.cross_axis_size as usize;
                let post = posts.get(ctx.index).unwrap();
                let mut post_item = PostItem::new(post, width);
                if ctx.is_selected {
                    post_item.set_background(Color::DarkGray);
                }
                let height = post_item.height();
                (post_item, height as u16)
            });
            let item_len = { self.state.read().unwrap().items.len() };
            let list = ListView::new(builder, item_len).block(block);
            // .highlight_style(
            //     Style::default()
            //         .bg(Color::Blue)
            //         .add_modifier(Modifier::BOLD),
            // );

            StatefulWidget::render(list, area, buf, &mut self.state.write().unwrap().list_state);
        }
    }
}

pub struct PostItem {
    pub username: String,
    pub title_lines: Vec<String>,
    pub body_lines: Vec<String>,
    pub background: Option<Color>,
    pub score: i64,
    pub num_comments: u64,
}

impl PostItem {
    pub fn new(post: &Post, width: usize) -> Self {
        let username = post.author.clone();
        let title_lines = textwrap::wrap(&post.title, width)
            .iter()
            .map(|i| i.to_string())
            .collect();
        let mut body_wrap = textwrap::wrap(&post.body, width);
        if body_wrap.len() > 4 {
            body_wrap.truncate(4);
            let mut new_last = body_wrap[3].to_string();
            if new_last.len() > 3 {
                let cut = new_last.len() - 3;
                new_last.replace_range(cut.., "...");
            } else {
                new_last = "...".to_string();
            }
            body_wrap[3] = Cow::Owned(new_last);
        }
        let body_lines = body_wrap.iter().map(|i| i.to_string()).collect();
        let score = post.score;
        let num_comments = post.num_comments;

        Self {
            username,
            title_lines,
            body_lines,
            background: None,
            score,
            num_comments,
        }
    }

    pub fn height(&self) -> usize {
        self.title_lines.len()
         + 1 //Spacing
         + self.body_lines.len()
         + 2 //block border
    }

    fn set_background(&mut self, background: Color) {
        self.background = Some(background);
    }
}

impl Widget for PostItem {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        let mut block = Block::bordered()
            .border_type(BorderType::Rounded)
            .title(format!("u/{}", self.username).italic())
            .title_bottom(format!("👍🏻{}", self.score.to_string()))
            .title_bottom(format!("💬{}", self.num_comments.to_string()));

        if let Some(background) = self.background {
            block = block.bg(background);
        }

        let [title_area, body_area] = Layout::vertical([
            Constraint::Length(self.title_lines.len() as u16 + 1),
            Constraint::Fill(1),
        ])
        .areas(block.inner(area));
        block.render(area, buf);

        Paragraph::new(
            self.title_lines
                .iter()
                .map(|i| Line::from(i.clone()))
                .collect::<Vec<Line>>(),
        )
        .add_modifier(Modifier::BOLD)
        .render(title_area, buf);
        Paragraph::new(
            self.body_lines
                .iter()
                .map(|i| Line::from(i.clone()))
                .collect::<Vec<Line>>(),
        )
        .render(body_area, buf);
    }
}
