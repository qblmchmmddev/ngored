use crate::reddit_api::CommentData;

#[derive(Clone)]
pub struct Comment {
    pub body: String,
    pub author: String,
    pub score: i64,
    pub replies: Vec<Comment>,
}

impl From<CommentData> for Comment {
    fn from(value: CommentData) -> Self {
        Self {
            body: value.body,
            author: value.author,
            score: value.score,
            replies: value.replies.map_or(Vec::new(), |replies| {
                replies
                    .as_listing()
                    .children
                    .into_iter()
                    .filter_map(|comment_data| comment_data.as_comment_opt().map(|v| v.into()))
                    .collect()
            }),
        }
    }
}

impl Comment {
    /// Flatten this comment tree into (depth, Comment)
    pub fn flatten(&self, depth: usize) -> Vec<(usize, Comment)> {
        let mut out = Vec::new();

        // push self
        out.push((depth, self.clone()));

        // recursively flatten replies
        for reply in &self.replies {
            out.extend(reply.flatten(depth + 1));
        }

        out
    }
}
