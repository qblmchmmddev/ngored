use std::sync::{Arc, RwLock};

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind};
use log::debug;
use ratatui::{
    layout::{Constraint, Flex, Layout, Rect, Size},
    style::{Modifier, Stylize},
    text::Line,
    widgets::{Block, BorderType, Paragraph, StatefulWidget, Widget},
};
use ratatui_image::{Resize, StatefulImage, picker::Picker, protocol::StatefulProtocol};
use tokio::sync::mpsc::Sender;
use tui_scrollview::{ScrollView, ScrollViewState, ScrollbarVisibility};

use crate::{
    app::AppEvent, component::Component, model::post::Post, ngored_error::NgoredError,
    reddit_api::RedditApi,
};

pub struct PostDetailState {
    loading: bool,
    post: Post,
    scroll_state: ScrollViewState,
    image: Option<StatefulProtocol>,
}

pub struct PostDetailComponent {
    reddit_api: Arc<RedditApi>,
    app_event_sender: Sender<AppEvent>,
    state: Arc<RwLock<PostDetailState>>,
    picker: Arc<Picker>,
}

impl PostDetailComponent {
    pub fn new(
        reddit_api: Arc<RedditApi>,
        picker: Arc<Picker>,
        app_event_sender: Sender<AppEvent>,
    ) -> Self {
        let state = PostDetailState {
            loading: false,
            post: Post::default(),
            scroll_state: ScrollViewState::default(),
            image: None,
        };
        Self {
            reddit_api,
            app_event_sender,
            state: Arc::new(RwLock::new(state)),
            picker,
        }
    }

    pub fn load(&self, post: Post) {
        {
            let state = self.state.read().unwrap();
            if state.post.id == post.id || state.loading {
                return;
            }
        }

        self.state.write().unwrap().post = post;
        tokio::spawn({
            let state = self.state.clone();
            let reddit_api = self.reddit_api.clone();
            let app_event_sender = self.app_event_sender.clone();
            let picker = self.picker.clone();
            async move {
                {
                    let mut state = state.write().unwrap();
                    state.loading = true;
                    state.scroll_state.scroll_to_top();
                }
                app_event_sender.send(AppEvent::Draw).await.unwrap();

                debug!("a");
                let image = {
                    let i = state.read().unwrap().post.preview_image_url.clone();
                    debug!("load iamge {:?}", i);
                    if let Some(image_url) = i {
                        let image_bytes = {
                            reddit_api
                                .clone()
                                .client
                                .get(image_url)
                                .send()
                                .await
                                .unwrap()
                                .bytes()
                                .await
                                .unwrap()
                        };
                        let image_source = image::load_from_memory(&image_bytes).unwrap();
                        Some(picker.new_resize_protocol(image_source))
                    } else {
                        None
                    }
                };
                {
                    let mut state = state.write().unwrap();
                    state.image = image;
                    state.loading = false;
                }
                app_event_sender.send(AppEvent::Draw).await.unwrap();
            }
        });
    }
}

impl Component for PostDetailComponent {
    async fn handle_event(&mut self, event: &Event) -> Result<(), NgoredError> {
        match event {
            Event::Key(KeyEvent {
                code: KeyCode::Char(char),
                kind: KeyEventKind::Press,
                ..
            }) => match char {
                'h' => {
                    self.app_event_sender
                        .send(AppEvent::ClosePostDetail)
                        .await?;
                }
                'j' => {
                    self.state.write().unwrap().scroll_state.scroll_down();
                    self.app_event_sender.send(AppEvent::Draw).await?;
                }
                'k' => {
                    self.state.write().unwrap().scroll_state.scroll_up();
                    self.app_event_sender.send(AppEvent::Draw).await?;
                }
                _ => {}
            },
            _ => {}
        }
        Ok(())
    }
    fn draw(&mut self, frame: &mut ratatui::Frame) {
        let root_area = frame.area();
        let root_buf = frame.buffer_mut();
        let (title, loading, body) = {
            let state = self.state.read().unwrap();
            (
                state.post.title.clone(),
                state.loading,
                state.post.body.clone(),
            )
        };
        let block = Block::bordered().border_type(BorderType::Rounded);
        let inner_block = block.inner(root_area);
        block.render(root_area, root_buf);
        if loading {
            let [center_vertically] = Layout::vertical([Constraint::Length(1)])
                .flex(Flex::Center)
                .areas(inner_block);
            let [center] = Layout::horizontal([Constraint::Length(10)])
                .flex(Flex::Center)
                .areas(center_vertically);
            Paragraph::new("Loading...").render(center, root_buf);
        } else {
            let [inner_block_no_scrollbar, _] =
                Layout::horizontal([Constraint::Fill(1), Constraint::Length(1)]).areas(inner_block);
            let mut height = 0;
            let title_wrap = textwrap::wrap(&title, inner_block_no_scrollbar.width as usize - 1);

            let title_lines = title_wrap
                .into_iter()
                .map(|i| Line::from(i))
                .collect::<Vec<Line>>();
            height += title_lines.len() as u16;

            let image_size = if let Some(image) = &self.state.read().unwrap().image {
                let [image_area] =
                    Layout::vertical([Constraint::Percentage(50)]).areas(inner_block_no_scrollbar);
                image.size_for(Resize::Scale(None), image_area)
            } else {
                Rect::ZERO
            };
            height += image_size.height;

            let body_wrap = textwrap::wrap(&body, inner_block_no_scrollbar.width as usize);
            let body_lines = body_wrap
                .into_iter()
                .map(|i| Line::from(i))
                .collect::<Vec<Line>>();
            height += body_lines.len() as u16;

            let mut scrollview = ScrollView::new(Size::new(inner_block.width, height as u16 + 2))
                .horizontal_scrollbar_visibility(ScrollbarVisibility::Never);
            let scrollview_area = scrollview.area();
            let [scrollview_area, _for_scrollbar] =
                Layout::horizontal([Constraint::Fill(1), Constraint::Length(1)])
                    .areas(scrollview_area);
            let scrollview_buf = scrollview.buf_mut();

            let [title_area, _, image_area, _, body_area] = Layout::vertical([
                Constraint::Length(title_lines.len() as u16),
                Constraint::Length(1),
                Constraint::Length(image_size.height),
                Constraint::Length(if image_size.height > 0 { 1 } else { 0 }),
                Constraint::Fill(1),
            ])
            .areas(scrollview_area);

            Paragraph::new(title_lines)
                .add_modifier(Modifier::BOLD)
                .render(title_area, scrollview_buf);

            let mut state = self.state.write().unwrap();
            if let Some(image) = &mut state.image {
                let [image_center] = Layout::horizontal([Constraint::Length(image_size.width)])
                    .flex(Flex::Center)
                    .areas(image_area);
                let image_widget = StatefulImage::new().resize(Resize::Scale(None));
                image_widget.render(image_center, scrollview_buf, image);
            }

            Paragraph::new(body_lines).render(body_area, scrollview_buf);
            scrollview.render(inner_block, root_buf, &mut state.scroll_state);
        }
    }
}
