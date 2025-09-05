use crate::reddit_api::PostData;

#[derive(Debug, Default, Clone)]
pub struct Post {
    pub id: String,
    pub subreddit: String,
    pub author: String,
    pub title: String,
    pub body: String,
    pub url: String,
    pub num_comments: u64,
    pub score: i64,
    pub crosspost_parent: Vec<Post>,
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
        }
    }
}
