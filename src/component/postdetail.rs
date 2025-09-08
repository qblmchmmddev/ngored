use std::{
    ops::Deref,
    sync::{Arc, RwLock},
};

use chrono::Utc;
use chrono_humanize::HumanTime;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind};
use futures::future::join_all;
use log::debug;
use ratatui::{
    layout::{Constraint, Flex, Layout, Rect, Size},
    style::{Modifier, Stylize},
    text::Line,
    widgets::{Block, BorderType, Borders, Paragraph, StatefulWidget, Widget},
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
    medias: Option<(usize, Vec<StatefulProtocol>)>,
    crosspost_parents_medias: Option<Vec<(usize, Vec<StatefulProtocol>)>>,
    loading_comment: bool,
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
            medias: None,
            crosspost_parents_medias: None,
            loading_comment: false,
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
                    Self::load_preivew_image(
                        state.clone(),
                        app_event_sender.clone(),
                        reddit_api.clone(),
                        picker.clone(),
                    ),
                    Self::load_crosspost_parent_medias(
                        state.clone(),
                        app_event_sender.clone(),
                        reddit_api.clone(),
                        picker.clone(),
                    ),
                    Self::load_gallery_images(
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
    }

    async fn load_crosspost_parent_medias(
        state: Arc<RwLock<PostDetailState>>,
        app_event_sender: Sender<AppEvent>,
        reddit_api: Arc<RedditApi>,
        picker: Arc<Picker>,
    ) {
        let crosspost_parents = state.read().unwrap().post.crosspost_parent.clone();
        let crosspost_parents_medias = crosspost_parents.into_iter().filter_map(|mut v| {
            if let Some(gallery_images) = v.galleries.take() {
                let reddit_api = reddit_api.clone();
                let picker = picker.clone();

                let gallery_images = gallery_images.into_iter().map(move |v| {
                    let reddit_api = reddit_api.clone();
                    let picker = picker.clone();
                    async move {
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
                        Some(picker.new_resize_protocol(image_source))
                    }
                });
                Some(async move {
                    join_all(gallery_images)
                        .await
                        .into_iter()
                        .flatten()
                        .collect::<Vec<_>>()
                })
            } else {
                None
            }
        });

        let crosspost_parents_medias = join_all(crosspost_parents_medias)
            .await
            .into_iter()
            .map(|v| (0, v))
            .collect();

        state.write().unwrap().crosspost_parents_medias = Some(crosspost_parents_medias);

        app_event_sender.send(AppEvent::Draw).await.unwrap();
    }

    async fn load_gallery_images(
        state: Arc<RwLock<PostDetailState>>,
        app_event_sender: Sender<AppEvent>,
        reddit_api: Arc<RedditApi>,
        picker: Arc<Picker>,
    ) {
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
            state.write().unwrap().medias = Some((0, gallery_images));

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
        state.write().unwrap().loading_comment = true;
        app_event_sender.send(AppEvent::Draw).await.unwrap();

        let comments = reddit_api.get_post_comment(sub, post_id).await;

        let comments = comments
            .as_listing()
            .children
            .into_iter()
            .filter_map(|d| d.as_comment_opt().map(|v| Comment::from(v)))
            .collect();
        {
            let mut state = state.write().unwrap();
            state.loading_comment = false;
            state.comments = comments;
        }

        app_event_sender.send(AppEvent::Draw).await.unwrap();
    }

    fn reset(&self) {
        let mut state = self.state.write().unwrap();
        state.post = Post::default();
        state.preview_image = None;
        state.comments.clear();
        state.loading_comment = false;
        if let Some((_, mut galleries)) = state.medias.take() {
            galleries.clear();
        };
        if let Some(mut crosspost_parents_medias) = state.crosspost_parents_medias.take() {
            crosspost_parents_medias
                .iter_mut()
                .for_each(|(_, v)| v.clear());
            crosspost_parents_medias.clear();
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
                'o' => {
                    let state = self.state.read().unwrap();
                    open::that(format!(
                        "https://www.reddit.com/r/{}/comments/{}",
                        state.post.subreddit, state.post.id
                    ))
                    .unwrap();
                }
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
                    let mut state = self.state.write().unwrap();
                    if let Some((index, images)) = state.medias.as_mut() {
                        if *index == 0 {
                            *index = images.len() - 1;
                        } else {
                            *index -= 1;
                        }
                    };
                    if let Some(crosspost_parents_medias) = state.crosspost_parents_medias.as_mut()
                    {
                        crosspost_parents_medias
                            .iter_mut()
                            .for_each(|(index, images)| {
                                if *index == 0 {
                                    *index = images.len() - 1;
                                } else {
                                    *index -= 1;
                                }
                            });
                    }
                    self.app_event_sender.send(AppEvent::Draw).await?;
                }
                ']' => {
                    let mut state = self.state.write().unwrap();
                    if let Some((index, images)) = state.medias.as_mut() {
                        *index += 1;
                        if *index >= images.len() {
                            *index = 0;
                        }
                    };
                    if let Some(crosspost_parents_medias) = state.crosspost_parents_medias.as_mut()
                    {
                        crosspost_parents_medias
                            .iter_mut()
                            .for_each(|(index, images)| {
                                *index += 1;
                                if *index >= images.len() {
                                    *index = 0;
                                }
                            });
                    }
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
        let (sub, created, author, title, score, num_comments, body, comments, loading_comment) = {
            let state = self.state.read().unwrap();
            (
                state.post.subreddit.clone(),
                state.post.created_at.clone(),
                state.post.author.clone(),
                state.post.title.clone(),
                state.post.score,
                state.post.num_comments,
                state.post.body.clone(),
                state.comments.clone(),
                state.loading_comment,
            )
        };
        let is_body_empty = body.is_empty();

        let root_block = Block::bordered().border_type(BorderType::Rounded).title(
            format!(
                "r/{} • u/{} • {}",
                sub,
                author,
                HumanTime::from(created - Utc::now())
            )
            .italic(),
        );
        let root_block_inner = root_block.inner(root_area);
        root_block.render(root_area, root_buf);

        let [root_block_inner_no_scrollbar, _] =
            Layout::horizontal([Constraint::Fill(1), Constraint::Length(2)])
                .areas(root_block_inner);

        let mut content_height = 0;

        let title_wrap = textwrap::wrap(&title, root_block_inner_no_scrollbar.width as usize);
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

        let crosspost_parents_medias_sizes = if let Some(crosspost_parents_medias) =
            &self.state.read().unwrap().crosspost_parents_medias
        {
            crosspost_parents_medias
                .iter()
                .map(|(index, images)| {
                    let media_image = &images[*index];
                    let [media_image_area] = Layout::vertical([Constraint::Percentage(50)])
                        .areas(root_block_inner_no_scrollbar);
                    let media_image_size =
                        media_image.size_for(Resize::Scale(None), media_image_area);
                    Rect::new(
                        media_image_size.x,
                        media_image_size.y,
                        media_image_size.width,
                        media_image_size.height + 1,
                    ) // + 1 for image index info
                })
                .collect::<Vec<_>>()
        } else {
            Vec::default()
        };
        let crosspost_parents_height = crosspost_parents_medias_sizes
            .iter()
            .fold(0, |a, b| a + b.height);
        content_height += crosspost_parents_height;

        let media_image_size = if let Some((index, images)) = &self.state.read().unwrap().medias {
            let media_image = &images[*index];
            let [media_image_area] =
                Layout::vertical([Constraint::Percentage(50)]).areas(root_block_inner_no_scrollbar);
            let media_image_size = media_image.size_for(Resize::Scale(None), media_image_area);
            Rect::new(
                media_image_size.x,
                media_image_size.y,
                media_image_size.width,
                media_image_size.height + 1,
            ) // + 1 for image index info
        } else {
            Rect::ZERO
        };
        content_height += media_image_size.height;

        let body_wrap = if is_body_empty {
            Vec::default()
        } else {
            textwrap::wrap(&body, root_block_inner_no_scrollbar.width as usize)
        };
        let body_lines = body_wrap
            .into_iter()
            .map(|i| Line::from(i))
            .collect::<Vec<Line>>();
        let body_height = body_lines.len() as u16;
        content_height += body_height;

        let (comment_widgets, comment_height) = if loading_comment {
            let comment_height = 1;
            content_height += comment_height;
            (None, comment_height)
        } else {
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
            (Some(comment_widgets), comments_height)
        };

        let mut scrollview =
            ScrollView::new(Size::new(root_block_inner.width, content_height as u16 + 2))
                .horizontal_scrollbar_visibility(ScrollbarVisibility::Never);
        let scrollview_area = scrollview.area();
        let [scrollview_area, _for_scrollbar] =
            Layout::horizontal([Constraint::Fill(1), Constraint::Length(1)]).areas(scrollview_area);
        let scrollview_buf = scrollview.buf_mut();

        content_height += 1; // for post info

        let [
            title_area,
            preview_image_area,
            crosspost_parents_area,
            gallery_image_area,
            body_area,
            info_area,
            comments_area,
        ] = Layout::vertical([
            Constraint::Length(title_lines.len() as u16),
            Constraint::Length(preview_image_size.height),
            Constraint::Length(crosspost_parents_height),
            Constraint::Length(media_image_size.height),
            Constraint::Length(body_height),
            Constraint::Length(1),
            Constraint::Length(comment_height),
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

        if let Some(crosspost_parents_medias) = &mut state.crosspost_parents_medias {
            let mut crosspost_parents_area = crosspost_parents_area;
            crosspost_parents_medias.iter_mut().enumerate().for_each(
                |(index, crosspost_parent_medias)| {
                    let size = crosspost_parents_medias_sizes[index];
                    let (index, images) = crosspost_parent_medias;

                    let [crosspost_parent_area, crosspost_info_area, remaining_area] =
                        Layout::vertical([
                            Constraint::Length(size.height - 1),
                            Constraint::Length(1),
                            Constraint::Fill(1),
                        ])
                        .areas(crosspost_parents_area);
                    crosspost_parents_area = remaining_area;

                    let [image_center] = Layout::horizontal([Constraint::Length(size.width)])
                        .flex(Flex::Center)
                        .areas(crosspost_parent_area);
                    let image_widget = StatefulImage::new().resize(Resize::Scale(None));
                    let image = &mut images[*index];
                    image_widget.render(image_center, scrollview_buf, image);

                    let info_text = format!("{}/{}", *index + 1, images.len());
                    let [info_center] =
                        Layout::horizontal([Constraint::Length(info_text.len() as u16)])
                            .flex(Flex::Center)
                            .areas(crosspost_info_area);
                    Paragraph::new(info_text).render(info_center, scrollview_buf);
                },
            );
        }

        if let Some((index, images)) = state.medias.as_mut() {
            let [gallery_image_area, gallery_info_area] =
                Layout::vertical([Constraint::Fill(1), Constraint::Length(1)])
                    .areas(gallery_image_area);

            let [image_center] = Layout::horizontal([Constraint::Length(media_image_size.width)])
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

        Block::new()
            .borders(Borders::BOTTOM)
            .title_bottom(format!("👍🏻{} • 💬{}", score, num_comments))
            .render(info_area, scrollview_buf);

        if loading_comment {
            let loading_comment_text = "Loading comment...";
            let [center] =
                Layout::horizontal([Constraint::Length(loading_comment_text.len() as u16)])
                    .flex(Flex::Center)
                    .areas(comments_area);
            Paragraph::new(loading_comment_text).render(center, scrollview_buf);
        } else if let Some(comment_widgets) = comment_widgets {
            let mut comments_area = comments_area;
            comment_widgets.into_iter().for_each(|i| {
                let [comment_area, remaining_comments_area] =
                    Layout::vertical([Constraint::Length(i.height() as u16), Constraint::Fill(1)])
                        .areas(comments_area);
                i.render(comment_area, scrollview_buf);
                comments_area = remaining_comments_area;
            });
        }

        scrollview.render(root_block_inner, root_buf, &mut state.scroll_state);
    }
}
