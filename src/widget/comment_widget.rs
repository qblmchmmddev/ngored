use ratatui::layout::{Constraint, Layout, Rect};

use crate::model::comment::Comment;

pub struct CommentWidget {
    depth: u16,
    body_texts: Vec<String>,
    is_selected: bool,
    author: String,
    score: i64,
}

impl CommentWidget {
    pub fn new(depth: u16, comment: Comment, is_selected: bool, area: Rect) -> Self {
        let [_, area] =
            Layout::horizontal([Constraint::Length(depth * 2), Constraint::Fill(1)]).areas(area);
        let text_wrap = textwrap::wrap(
            &comment.body,
            textwrap::Options::new(area.width as usize - 4),
        );
        Self {
            depth: depth,
            body_texts: text_wrap.into_iter().map(|v| v.into_owned()).collect(),
            is_selected: is_selected,
            author: comment.author.clone(),
            score: comment.score,
        }
    }
}
