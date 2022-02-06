//! # pullcaps
//!
//! The `pullcaps` crate provides a convenient, opinionated, asynchronous client
//! for the [PushShift API](https://pushshift.io/).
//!
//! ## Getting all comments from a specific user.
//!
//! The following example shows a small script that gets all comments made
//! by a specific user.
//!
//! ```rust,no_run
//! use futures::StreamExt;
//! use pullcaps::{Client, Filter};
//!
//! # #[tokio::main]
//! # async fn main() {
//! let client = Client::new();
//! let filter = Filter::new().author("reddit");
//!
//! let mut comments = client.get_comments(filter).await;
//! while let Some(comment) = comments.next().await {
//!     println!("text: {}", comment.body);
//! }
//! # }
//! ```
//!
//! **NOTE**: If you plan to perform multiple requests, it is best to create a [`Client`]
//! and reuse it.
//!
//! ## Getting posts in a given subreddit
//!
//! The following example shows how to get posts from a given subreddit - in particular
//! we utilize [`futures::StreamExt::take`] to limit ourselves to the five most recent posts
//! in the subreddit.
//!
//! ```rust,no_run
//! use futures::StreamExt;
//! use pullcaps::{Client, Filter};
//!
//! # #[tokio::main]
//! # async fn main() {
//! let client = Client::new();
//! let filter = Filter::new().subreddit("askreddit");
//!
//! let mut posts = client.get_posts(filter).await.take(5);
//! while let Some(post) = posts.next().await {
//!     if let Some(text) = post.self_text {
//!         println!("text: {}", text);
//!     }
//! }
//! # }
//! ```

pub mod models;

mod client;
mod filter;

pub use client::Client;
pub use filter::{Filter, SortType};
