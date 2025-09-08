use std::{
    ops::Deref,
    sync::{Arc, RwLock},
};

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind};
use futures::future::join_all;
use log::debug;
use ratatui::{
    layout::{Constraint, Flex, Layout, Rect, Size},
    style::{Modifier, Stylize},
    text::Line,
    widgets::{Block, BorderType, Paragraph, StatefulWidget, Widget},
};
use ratatui_image::{Resize, StatefulImage, picker::Picker, protocol::StatefulProtocol};
use tokio::{sync::mpsc::Sender, task::JoinHandle};
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
    preview_image: Option<StatefulProtocol>,
    galleries: Option<(usize, Vec<StatefulProtocol>)>,
    comments: Vec<Comment>,
    load_handle: Option<JoinHandle<()>>,
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
            preview_image: None,
            galleries: None,
            comments: Vec::default(),
            load_handle: None,
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
        self.state.write().unwrap().load_handle = Some(tokio::spawn({
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
                    Self::load_gallery_images(
                        state.clone(),
                        app_event_sender.clone(),
                        reddit_api.clone(),
                        picker.clone(),
                    ),
                    Self::load_preivew_image(
                        state.clone(),
                        app_event_sender.clone(),
                        reddit_api.clone(),
                        picker.clone(),
                    ),
                    Self::load_comments(
                        state.clone(),
                        app_event_sender.clone(),
                        &sub,
                        &post_id,
                        reddit_api.clone()
                    )
                );
            }
        }));
    }

    async fn load_preivew_image(
        state: Arc<RwLock<PostDetailState>>,
        app_event_sender: Sender<AppEvent>,
        reddit_api: Arc<RedditApi>,
        picker: Arc<Picker>,
    ) {
        debug!("load_preivew_image start");
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
                state.preview_image = Some(picker.new_resize_protocol(image_source));
            }
            app_event_sender.send(AppEvent::Draw).await.unwrap();
        };
        debug!("load_preivew_image end");
    }

    async fn load_gallery_images(
        state: Arc<RwLock<PostDetailState>>,
        app_event_sender: Sender<AppEvent>,
        reddit_api: Arc<RedditApi>,
        picker: Arc<Picker>,
    ) {
        debug!("load_gallery_images start");
        let gallery_images = state.read().unwrap().post.galleries.clone();
        if let Some(gallery_images) = gallery_images {
            let gallery_images = gallery_images.into_iter().map(|v| async {
                let image_bytes = {
                    reddit_api
                        .client
                        .get(v)
                        .send()
                        .await
                        .unwrap()
                        .bytes()
                        .await
                        .unwrap()
                };
                let image_source = image::load_from_memory(&image_bytes).unwrap();
                picker.new_resize_protocol(image_source)
            });
            let gallery_images = join_all(gallery_images).await;
            state.write().unwrap().galleries = Some((0, gallery_images));

            app_event_sender.send(AppEvent::Draw).await.unwrap();
        }
        debug!("load_gallery_images end");
    }

    async fn load_comments(
        state: Arc<RwLock<PostDetailState>>,
        app_event_sender: Sender<AppEvent>,
        sub: &str,
        post_id: &str,
        reddit_api: Arc<RedditApi>,
    ) {
        debug!("load_comments start");
        let comments = reddit_api.get_post_comment(sub, post_id).await;
        {
            let comments = comments
                .as_listing()
                .children
                .into_iter()
                .filter_map(|d| d.as_comment_opt().map(|v| Comment::from(v)))
                .collect();
            state.write().unwrap().comments = comments;
        }
        app_event_sender.send(AppEvent::Draw).await.unwrap();
        debug!("load_comments end");
    }

    fn reset(&self) {
        let mut state = self.state.write().unwrap();
        state.post = Post::default();
        state.preview_image = None;
        state.comments.clear();
        if let Some((_, mut galleries)) = state.galleries.take() {
            galleries.clear();
        };
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
                    if let Some(load_handle) = self.state.write().unwrap().load_handle.take() {
                        load_handle.abort();
                    }
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
                '[' => {
                    if let Some((index, images)) = self.state.write().unwrap().galleries.as_mut() {
                        if *index == 0 {
                            *index = images.len() - 1;
                        } else {
                            *index -= 1;
                        }
                    };
                    self.app_event_sender.send(AppEvent::Draw).await?;
                }
                ']' => {
                    if let Some((index, images)) = self.state.write().unwrap().galleries.as_mut() {
                        *index += 1;
                        if *index >= images.len() {
                            *index = 0;
                        }
                    };
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

        let [root_block_inner_no_scrollbar, _] =
            Layout::horizontal([Constraint::Fill(1), Constraint::Length(1)])
                .areas(root_block_inner);

        let mut content_height = 0;

        let title_wrap = textwrap::wrap(&title, root_block_inner_no_scrollbar.width as usize - 1);
        let title_lines = title_wrap
            .into_iter()
            .map(|i| Line::from(i))
            .collect::<Vec<Line>>();
        content_height += title_lines.len() as u16;

        let preview_image_size =
            if let Some(preview_image) = &self.state.read().unwrap().preview_image {
                let [preview_image_area] = Layout::vertical([Constraint::Percentage(50)])
                    .areas(root_block_inner_no_scrollbar);
                preview_image.size_for(Resize::Scale(None), preview_image_area)
            } else {
                Rect::ZERO
            };
        content_height += preview_image_size.height;

        let gallery_image_size =
            if let Some((index, images)) = &self.state.read().unwrap().galleries {
                let gallery_image = &images[*index];
                let [gallery_image_area] = Layout::vertical([Constraint::Percentage(50)])
                    .areas(root_block_inner_no_scrollbar);
                let gallery_image_size =
                    gallery_image.size_for(Resize::Scale(None), gallery_image_area);
                Rect::new(
                    gallery_image_size.x,
                    gallery_image_size.y,
                    gallery_image_size.width,
                    gallery_image_size.height + 1,
                ) // + 1 for image info
            } else {
                Rect::ZERO
            };
        content_height += gallery_image_size.height;

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
            Layout::horizontal([Constraint::Fill(1), Constraint::Length(1)]).areas(scrollview_area);
        let scrollview_buf = scrollview.buf_mut();

        let [
            title_area,
            _,
            preview_image_area,
            _,
            gallery_image_area,
            _,
            body_area,
            _,
            comments_area,
        ] = Layout::vertical([
            Constraint::Length(title_lines.len() as u16),
            Constraint::Length(1),
            Constraint::Length(preview_image_size.height),
            Constraint::Length(if preview_image_size.height > 0 { 1 } else { 0 }),
            Constraint::Length(gallery_image_size.height),
            Constraint::Length(if gallery_image_size.height > 0 { 1 } else { 0 }),
            Constraint::Length(body_height),
            Constraint::Length(if body_height > 0 { 1 } else { 0 }),
            Constraint::Length(comments_height),
        ])
        .areas(scrollview_area);

        Paragraph::new(title_lines)
            .add_modifier(Modifier::BOLD)
            .render(title_area, scrollview_buf);

        let mut state = self.state.write().unwrap();
        if let Some(image) = &mut state.preview_image {
            let [image_center] = Layout::horizontal([Constraint::Length(preview_image_size.width)])
                .flex(Flex::Center)
                .areas(preview_image_area);
            let image_widget = StatefulImage::new().resize(Resize::Scale(None));
            image_widget.render(image_center, scrollview_buf, image);
        }

        if let Some((index, images)) = state.galleries.as_mut() {
            let [gallery_image_area, gallery_info_area] =
                Layout::vertical([Constraint::Fill(1), Constraint::Length(1)])
                    .areas(gallery_image_area);

            let [image_center] = Layout::horizontal([Constraint::Length(gallery_image_size.width)])
                .flex(Flex::Center)
                .areas(gallery_image_area);
            let image_widget = StatefulImage::new().resize(Resize::Scale(None));
            let image = &mut images[*index];
            image_widget.render(image_center, scrollview_buf, image);

            let info_text = format!("{}/{}", *index + 1, images.len());
            let [info_center] = Layout::horizontal([Constraint::Length(info_text.len() as u16)])
                .flex(Flex::Center)
                .areas(gallery_info_area);
            Paragraph::new(info_text).render(info_center, scrollview_buf);
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
