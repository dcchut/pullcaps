use chrono::serde::ts_seconds_option;
use chrono::{DateTime, Utc};
use serde::Serialize;

/// Used to filter a particular query down in some way.
#[derive(Clone, Default, Serialize)]
pub struct Filter {
    pub author: Option<String>,
    pub subreddit: Option<String>,

    #[serde(with = "ts_seconds_option")]
    pub before: Option<DateTime<Utc>>,

    #[serde(with = "ts_seconds_option")]
    pub after: Option<DateTime<Utc>>,

    pub sort_type: SortType,

    #[serde(skip)]
    pub limit: Option<i64>,
}

impl Filter {
    pub fn new() -> Self {
        Self {
            author: None,
            subreddit: None,
            before: None,
            after: None,
            sort_type: SortType::default(),
            limit: None,
        }
    }

    #[must_use]
    pub fn author<S: Into<String>>(mut self, author: S) -> Self {
        self.author = Some(author.into());
        self
    }

    #[must_use]
    pub fn subreddit<S: Into<String>>(mut self, subreddit: S) -> Self {
        self.subreddit = Some(subreddit.into());
        self
    }

    #[must_use]
    pub fn before(mut self, before: DateTime<Utc>) -> Self {
        self.before = Some(before);
        self
    }

    #[must_use]
    pub fn after(mut self, after: DateTime<Utc>) -> Self {
        self.after = Some(after);
        self
    }

    #[must_use]
    pub fn sort_type(mut self, sort_type: SortType) -> Self {
        self.sort_type = sort_type;
        self
    }

    #[must_use]
    pub fn limit(mut self, limit: i64) -> Self {
        self.limit = Some(limit);
        self
    }
}

/// Indicates how a particular query should be sorted.
#[non_exhaustive]
#[derive(Clone, Debug, Serialize)]
pub enum SortType {
    /// Sort by creation date.
    #[serde(rename = "created_utc")]
    CreatedDate,
    /// Sort by score.
    #[serde(rename = "score")]
    Score,
    /// Sort by number of comments.
    #[serde(rename = "num_comments")]
    NumComments,
}

impl SortType {
    pub fn new() -> Self {
        Self::CreatedDate
    }
}

impl Default for SortType {
    fn default() -> Self {
        Self::new()
    }
}
