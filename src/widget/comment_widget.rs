use chrono::{DateTime, Utc};
use chrono_humanize::HumanTime;
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Stylize},
    text::Line,
    widgets::{Block, BorderType, Borders, Paragraph, Widget},
};

use crate::model::comment::Comment;

pub struct CommentWidget {
    depth: u16,
    body_texts: Vec<String>,
    is_selected: bool,
    author: String,
    score: i64,
    created: DateTime<Utc>,
}

impl CommentWidget {
    pub fn new(depth: u16, comment: Comment, is_selected: bool, container_width: u16) -> Self {
        let width = container_width - depth * 2;
        let text_wrap = textwrap::wrap(&comment.body, textwrap::Options::new(width as usize));
        Self {
            depth: depth,
            body_texts: text_wrap.into_iter().map(|v| v.into_owned()).collect(),
            is_selected: is_selected,
            author: comment.author.clone(),
            score: comment.score,
            created: comment.created_at,
        }
    }

    pub fn height(&self) -> usize {
        self.body_texts.len() + 2
    }
}

impl Widget for CommentWidget {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        let [_, area] =
            Layout::horizontal([Constraint::Length(self.depth * 2), Constraint::Fill(1)])
                .areas(area);
        let lines: Vec<Line> = self.body_texts.into_iter().map(|t| Line::from(t)).collect();
        let mut item = Paragraph::new(lines).block(
            Block::new()
                .borders(Borders::LEFT | Borders::BOTTOM)
                .border_type(BorderType::Rounded)
                // .title(self.author.bold())
                .title(Line::from(vec![
                    self.author.bold(),
                    format!(" • {}", HumanTime::from(self.created - Utc::now())).italic(),
                ]))
                .title_bottom(format!("👍🏻{}", self.score)),
        );
        if self.is_selected {
            item = item.fg(Color::Green);
        }
        item.render(area, buf);
    }
}
