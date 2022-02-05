use chrono::serde::ts_seconds;
use chrono::{DateTime, Utc};
use serde::Deserialize;

pub(crate) trait AsAttrs {
    fn attrs(&self) -> &Attrs;
}

#[derive(Clone, Debug, Deserialize)]
pub enum Content {
    Comment(Comment),
    Post(Post),
}

impl Content {
    pub fn attrs(&self) -> &Attrs {
        match self {
            Content::Comment(comment) => &comment.attrs,
            Content::Post(post) => &post.attrs,
        }
    }

    pub fn as_comment(&self) -> Option<&Comment> {
        match self {
            Content::Comment(comment) => Some(comment),
            _ => None,
        }
    }

    pub fn as_post(&self) -> Option<&Post> {
        match self {
            Content::Post(post) => Some(post),
            _ => None,
        }
    }
}

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

#[derive(Clone, Debug, Deserialize)]
pub struct Comment {
    #[serde(flatten)]
    pub author: Author,

    #[serde(flatten)]
    pub subreddit: SubReddit,

    #[serde(flatten)]
    pub attrs: Attrs,

    // CommentContent or something?
    pub body: String,
    pub parent_id: String,
}

impl AsAttrs for Comment {
    fn attrs(&self) -> &Attrs {
        &self.attrs
    }
}

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

#[derive(Clone, Debug, Deserialize)]
pub struct Author {
    #[serde(rename = "author_fullname")]
    pub id: Option<String>,
    #[serde(rename = "author")]
    pub name: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct SubReddit {
    #[serde(rename = "subreddit_id")]
    pub id: String,
    #[serde(rename = "subreddit")]
    pub name: String,
}
