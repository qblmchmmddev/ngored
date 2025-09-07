use reqwest::Client;
use serde::{Deserialize, Deserializer};
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct RedditApi {
    pub client: Client,
}

impl RedditApi {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.6 Safari/605.1.15")
    .build().unwrap();

        Self { client }
    }

    pub async fn get_posts(&self, sub: &str) -> Content<ListingData<PostData>> {
        self.client
            .get(format!("https://www.reddit.com/r/{}/best.json", sub))
            .query(&[("raw_json", "1")])
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap()
    }

    pub async fn get_post_comment(
        &self,
        sub: &str,
        post_id: &str,
    ) -> Content<ListingData<CommentData>> {
        let res: Vec<serde_json::Value> = self
            .client
            .get(format!("https://www.reddit.com/r/{}/{}.json", sub, post_id))
            .query(&[("raw_json", "1")])
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        let comment_data: Content<ListingData<CommentData>> =
            serde_json::from_value(res[1].clone()).unwrap();
        comment_data
    }
}

#[derive(Debug, Deserialize)]
pub struct Content<T> {
    pub data: T,
}

#[derive(Debug, Deserialize)]
pub struct ListingData<T> {
    pub children: Vec<Content<T>>,
}

#[derive(Debug, Deserialize)]
pub struct PostData {
    pub id: String,
    pub subreddit: String,
    pub author: String,
    pub title: String,
    pub selftext: String,
    pub url: String,
    pub num_comments: u64,
    pub score: i64,
    #[serde(default = "Vec::default")]
    pub crosspost_parent_list: Vec<PostData>,
    pub preview: Option<Preview>,
}

#[derive(Debug, Deserialize)]
pub struct Preview {
    pub images: Vec<Image>,
}

#[derive(Debug, Deserialize)]
pub struct Image {
    pub resolutions: Vec<ImageResolution>,
}

#[derive(Debug, Deserialize)]
pub struct ImageResolution {
    pub url: String,
    pub width: u16,
    pub height: u16,
}

#[derive(Debug, Deserialize)]
pub struct CommentData {
    pub body: String,
    pub author: String,
    pub score: i64,
    #[serde(default, deserialize_with = "deserialize_replies")]
    pub replies: Option<Content<ListingData<CommentData>>>,
}

fn deserialize_replies<'de, D>(
    deserializer: D,
) -> Result<Option<Content<ListingData<CommentData>>>, D::Error>
where
    D: Deserializer<'de>,
{
    let val: Value = Deserialize::deserialize(deserializer)?;
    if val.is_string() {
        // replies == ""
        Ok(None)
    } else if val.is_object() {
        // replies == { kind: "Listing", data: ... }
        serde_json::from_value(val)
            .map(Some)
            .map_err(serde::de::Error::custom)
    } else {
        Ok(None)
    }
}
