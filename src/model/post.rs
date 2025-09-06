use crate::reddit_api::PostData;

#[derive(Debug, Default, Clone)]
pub struct Post {
    pub author: String,
    pub body: String,
    pub crosspost_parent: Vec<Post>,
    pub id: String,
    pub num_comments: u64,
    pub preview_image_url: Option<String>,
    pub score: i64,
    pub subreddit: String,
    pub title: String,
    pub url: String,
}

impl From<PostData> for Post {
    fn from(value: PostData) -> Self {
        Post {
            id: value.id,
            subreddit: value.subreddit,
            author: value.author,
            title: value.title,
            body: value.selftext,
            url: value.url,
            num_comments: value.num_comments,
            score: value.score,
            crosspost_parent: value
                .crosspost_parent_list
                .into_iter()
                .map(Post::from)
                .collect(),
            preview_image_url: value.preview.and_then(|i| {
                i.images
                    .first()
                    .and_then(|i| i.resolutions.last().map(|i| i.url.clone()))
            }),
        }
    }
}
