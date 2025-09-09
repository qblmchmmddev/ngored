use std::collections::HashMap;

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

    pub async fn get_posts(&self, sub: &str) -> Data {
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

    pub async fn get_post_comment(&self, sub: &str, post_id: &str) -> Data {
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
        serde_json::from_value(res[1].clone()).unwrap()
    }
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", content = "data")]
pub enum Data {
    #[serde(rename = "t1")]
    Comment(CommentData),
    #[serde(rename = "t3")]
    Post(PostData),
    Listing(ListingData),
    #[serde(rename = "more")]
    More(MoreData),
}

impl Data {
    fn variant_str(&self) -> &'static str {
        match self {
            Data::Comment(..) => "Comment",
            Data::Post(..) => "Post",
            Data::Listing(..) => "Listing",
            Data::More(..) => "More",
        }
    }

    pub fn as_post(self) -> PostData {
        if let Data::Post(data) = self {
            data
        } else {
            panic!("{} is not Post", self.variant_str())
        }
    }

    pub fn as_listing(self) -> ListingData {
        if let Data::Listing(data) = self {
            data
        } else {
            panic!("{} is not Listing", self.variant_str())
        }
    }

    pub fn as_comment(self) -> CommentData {
        if let Data::Comment(data) = self {
            data
        } else {
            panic!("{} is not Comment", self.variant_str())
        }
    }
    pub fn as_comment_opt(self) -> Option<CommentData> {
        if let Data::Comment(data) = self {
            Some(data)
        } else {
            None
        }
    }
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

fn deserialize_replies<'de, D>(deserializer: D) -> Result<Option<Box<Data>>, D::Error>
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

#[derive(Debug, Deserialize)]
pub struct MoreData {
    pub count: u64,
    pub children: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct ListingData {
    pub children: Vec<Data>,
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
    pub media_metadata: Option<MediaMetadata>,
    pub gallery_data: Option<GalleryData>,
    pub created_utc: f64,
    pub media: Option<Media>,
}
#[derive(Debug, Deserialize)]
pub struct Media {
    pub reddit_video: Option<RedditVideo>,
}

#[derive(Debug, Deserialize)]
pub struct RedditVideo {
    pub hls_url: String,
}

#[derive(Debug, Deserialize)]
pub struct MediaMetadata {
    #[serde(flatten)]
    pub items: HashMap<String, MediaItem>,
}

#[derive(Debug, Deserialize)]
pub struct MediaItem {
    pub status: String,
    pub e: String, // probably better as enum if only "Image"
    pub m: String, // MIME type
    pub p: Vec<MediaPreview>,
    // pub s: MediaPreview,
    pub id: String,
}

#[derive(Debug, Deserialize)]
pub struct MediaPreview {
    pub y: u32,
    pub x: u32,
    pub u: String,
}
#[derive(Debug, Deserialize)]
pub struct GalleryData {
    pub items: Vec<GalleryItem>,
}

#[derive(Debug, Deserialize)]
pub struct GalleryItem {
    pub media_id: String,
}

#[derive(Debug, Deserialize)]
pub struct CommentData {
    pub body: String,
    pub author: String,
    pub score: i64,
    pub created_utc: f64,
    #[serde(default, deserialize_with = "deserialize_replies")]
    pub replies: Option<Box<Data>>,
}

// impl<'de> Deserialize<'de> for ListingData<CommentData> {
//     fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
//     where
//         D: Deserializer<'de>,
//     {
//         let value: Value = Value::deserialize(deserializer)?;

//         let children = value
//             .get("children")
//             .and_then(|i| i.as_array())
//             .ok_or_else(|| serde::de::Error::missing_field("children"))?;

//         let mut comments = children
//             .iter()
//             .filter_map(|val| {
//                 if val.get("kind").and_then(|v| v.as_str()) == Some("t1") {
//                     serde_json::from_value(val.clone()).ok()
//                 } else {
//                     None
//                 }
//             })
//             .collect();

//         Ok(ListingData { children: comments })
//     }
// }
