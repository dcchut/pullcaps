use chrono::serde::ts_seconds_option;
use chrono::{DateTime, Utc};
use serde::Serialize;

/// Used to filter a particular query down in some way.
#[derive(Clone, Default, Serialize)]
pub struct Filter {
    author: Option<String>,
    subreddit: Option<String>,

    #[serde(with = "ts_seconds_option")]
    before: Option<DateTime<Utc>>,
}

impl Filter {
    pub fn new() -> Self {
        Self {
            author: None,
            subreddit: None,
            before: None,
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
}
