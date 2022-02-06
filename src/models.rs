//! The data model underlying the PushShift API.
use chrono::serde::ts_seconds;
use chrono::{DateTime, Utc};
use serde::Deserialize;

pub(crate) trait AsAttrs {
    fn attrs(&self) -> &Attrs;
}

/// Common attributes between  [`Post`]'s and [`Comment`]'s.
#[derive(Clone, Debug, Deserialize)]
pub struct Attrs {
    /// A unique ID identify the content.
    pub id: String,

    /// The score of this content.
    pub score: i32,

    /// A permalink to this content.
    pub permalink: Option<String>,

    /// The date at which this content was created.
    #[serde(rename = "created_utc", with = "ts_seconds")]
    pub date: DateTime<Utc>,
}

/// A single comment on a reddit [`Post`].
#[derive(Clone, Debug, Deserialize)]
pub struct Comment {
    #[serde(flatten)]
    pub author: Author,

    #[serde(flatten)]
    pub subreddit: SubReddit,

    #[serde(flatten)]
    pub attrs: Attrs,

    pub body: String,
    pub parent_id: String,
}

impl AsAttrs for Comment {
    fn attrs(&self) -> &Attrs {
        &self.attrs
    }
}

/// A single reddit post.
#[derive(Clone, Debug, Deserialize)]
pub struct Post {
    #[serde(flatten)]
    pub author: Author,

    #[serde(flatten)]
    pub subreddit: SubReddit,

    #[serde(flatten)]
    pub attrs: Attrs,

    /// URL of the linked content.
    #[serde(rename = "url")]
    pub content_url: String,

    /// URL to the comment page for this post.
    #[serde(rename = "full_link")]
    pub comment_url: String,

    /// The text of this post, if a self-post.
    #[serde(rename = "selftext")]
    pub self_text: Option<String>,
}

impl AsAttrs for Post {
    fn attrs(&self) -> &Attrs {
        &self.attrs
    }
}

/// The author of a [`Post`] or [`Comment`].
#[derive(Clone, Debug, Deserialize)]
pub struct Author {
    #[serde(rename = "author_fullname")]
    pub id: Option<String>,
    #[serde(rename = "author")]
    pub name: String,
}

/// The subreddit associated to a [`Post`] or [`Comment`]
#[derive(Clone, Debug, Deserialize)]
pub struct SubReddit {
    #[serde(rename = "subreddit_id")]
    pub id: String,
    #[serde(rename = "subreddit")]
    pub name: String,
}
