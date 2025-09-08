use std::{
    ops::Deref,
    sync::{Arc, RwLock},
};

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
    app::AppEvent,
    component::Component,
    model::{comment::Comment, post::Post},
    ngored_error::NgoredError,
    reddit_api::RedditApi,
    widget::comment_widget::CommentWidget,
};

pub struct PostDetailState {
    post: Post,
    scroll_state: ScrollViewState,
    image: Option<StatefulProtocol>,
    comments: Vec<Comment>,
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
            post: Post::default(),
            scroll_state: ScrollViewState::default(),
            image: None,
            comments: Vec::default(),
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
            if state.post.id == post.id {
                return;
            }
        }

        self.state.write().unwrap().post = post;
        tokio::spawn({
            let state = self.state.clone();
            let reddit_api = self.reddit_api.clone();
            let app_event_sender = self.app_event_sender.clone();
            let picker = self.picker.clone();
            let (sub, post_id) = {
                let state = state.read().unwrap();
                (state.post.subreddit.clone(), state.post.id.clone())
            };
            async move {
                {
                    let mut state = state.write().unwrap();
                    state.scroll_state.scroll_to_top();
                }
                app_event_sender.send(AppEvent::Draw).await.unwrap();

                tokio::join!(
                    Self::load_image(
                        state.clone(),
                        app_event_sender.clone(),
                        reddit_api.clone(),
                        picker.clone(),
                    ),
                    Self::load_comments(
                        state.clone(),
                        app_event_sender,
                        &sub,
                        &post_id,
                        reddit_api
                    )
                );
            }
        });
    }

    async fn load_image(
        state: Arc<RwLock<PostDetailState>>,
        app_event_sender: Sender<AppEvent>,
        reddit_api: Arc<RedditApi>,
        picker: Arc<Picker>,
    ) {
        let i = state
            .read()
            .unwrap()
            .post
            .preview_image_urls
            .as_ref()
            .and_then(|v| v.last().map(|v| v.clone()));
        if let Some(image_url) = i {
            let image_bytes = {
                reddit_api
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
            {
                let mut state = state.write().unwrap();
                state.image = Some(picker.new_resize_protocol(image_source));
            }
            app_event_sender.send(AppEvent::Draw).await.unwrap();
        }
    }

    async fn load_comments(
        state: Arc<RwLock<PostDetailState>>,
        app_event_sender: Sender<AppEvent>,
        sub: &str,
        post_id: &str,
        reddit_api: Arc<RedditApi>,
    ) {
        let comments = reddit_api.get_post_comment(sub, post_id).await;
        {
            state.write().unwrap().comments = comments
                .as_listing()
                .children
                .into_iter()
                .filter_map(|d| d.as_comment_opt().map(|v| Comment::from(v)))
                .collect();
        }
        app_event_sender.send(AppEvent::Draw).await.unwrap();
    }

    fn reset(&self) {
        let mut state = self.state.write().unwrap();
        state.post = Post::default();
        state.image = None;
        state.comments.clear();
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
                    self.reset();
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
                'J' => {
                    self.state.write().unwrap().scroll_state.scroll_page_down();
                    self.app_event_sender.send(AppEvent::Draw).await?;
                }
                'K' => {
                    self.state.write().unwrap().scroll_state.scroll_page_up();
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
        let (title, body, comments) = {
            let state = self.state.read().unwrap();
            (
                state.post.title.clone(),
                state.post.body.clone(),
                state.comments.clone(),
            )
        };

        let root_block = Block::bordered().border_type(BorderType::Rounded);
        let root_block_inner = root_block.inner(root_area);
        root_block.render(root_area, root_buf);

        if false {
            let [center_vertically] = Layout::vertical([Constraint::Length(1)])
                .flex(Flex::Center)
                .areas(root_block_inner);
            let [center] = Layout::horizontal([Constraint::Length(10)])
                .flex(Flex::Center)
                .areas(center_vertically);
            Paragraph::new("Loading...").render(center, root_buf);
        } else {
            let [root_block_inner_no_scrollbar, _] =
                Layout::horizontal([Constraint::Fill(1), Constraint::Length(1)])
                    .areas(root_block_inner);

            let mut content_height = 0;

            let title_wrap =
                textwrap::wrap(&title, root_block_inner_no_scrollbar.width as usize - 1);
            let title_lines = title_wrap
                .into_iter()
                .map(|i| Line::from(i))
                .collect::<Vec<Line>>();
            content_height += title_lines.len() as u16;

            let image_size = if let Some(image) = &self.state.read().unwrap().image {
                let [image_area] = Layout::vertical([Constraint::Percentage(50)])
                    .areas(root_block_inner_no_scrollbar);
                image.size_for(Resize::Scale(None), image_area)
            } else {
                Rect::ZERO
            };
            content_height += image_size.height;

            let body_wrap = textwrap::wrap(&body, root_block_inner_no_scrollbar.width as usize);
            let body_lines = body_wrap
                .into_iter()
                .map(|i| Line::from(i))
                .collect::<Vec<Line>>();
            let body_height = body_lines.len() as u16;
            content_height += body_height;

            let all_comments: Vec<(usize, Comment)> =
                comments.into_iter().flat_map(|v| v.flatten(0)).collect();

            let comment_widgets: Vec<CommentWidget> = all_comments
                .into_iter()
                .map(|i| {
                    let (depth, comment) = i;
                    let comment_widget = CommentWidget::new(
                        depth as u16,
                        comment,
                        false,
                        root_block_inner_no_scrollbar.width,
                    );
                    comment_widget
                })
                .collect();
            let comments_height = comment_widgets.iter().fold(0, |a, b| a + b.height() as u16);
            content_height += comments_height;

            let mut scrollview =
                ScrollView::new(Size::new(root_block_inner.width, content_height as u16 + 2))
                    .horizontal_scrollbar_visibility(ScrollbarVisibility::Never);
            let scrollview_area = scrollview.area();
            let [scrollview_area, _for_scrollbar] =
                Layout::horizontal([Constraint::Fill(1), Constraint::Length(1)])
                    .areas(scrollview_area);
            let scrollview_buf = scrollview.buf_mut();

            let [title_area, _, image_area, _, body_area, _, comments_area] = Layout::vertical([
                Constraint::Length(title_lines.len() as u16),
                Constraint::Length(1),
                Constraint::Length(image_size.height),
                Constraint::Length(if image_size.height > 0 { 1 } else { 0 }),
                Constraint::Length(body_height),
                Constraint::Length(if body_height > 0 { 1 } else { 0 }),
                Constraint::Length(comments_height),
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

            let mut comments_area = comments_area;
            comment_widgets.into_iter().for_each(|i| {
                let [comment_area, remaining_comments_area] =
                    Layout::vertical([Constraint::Length(i.height() as u16), Constraint::Fill(1)])
                        .areas(comments_area);
                i.render(comment_area, scrollview_buf);
                comments_area = remaining_comments_area;
            });

            scrollview.render(root_block_inner, root_buf, &mut state.scroll_state);
        }
    }
}
