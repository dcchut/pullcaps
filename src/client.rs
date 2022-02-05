use crate::models::{AsAttrs, Comment, Post};
use crate::Filter;
use async_stream::stream;
use futures::stream::{self, Stream, StreamExt};
use governor::{Quota, RateLimiter};
use once_cell::sync::OnceCell;
use reqwest::IntoUrl;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::num::NonZeroU32;
use std::pin::Pin;

type PSRateLimiter = RateLimiter<
    governor::state::NotKeyed,
    governor::state::InMemoryState,
    governor::clock::DefaultClock,
    governor::middleware::NoOpMiddleware,
>;

const BATCH_SIZE: u8 = 25;
static PS_CLIENT: OnceCell<PSRateLimiter> = OnceCell::new();

/// A global rate limiter to ensure we don't blow our API limits
async fn rate_limiter(client: &reqwest::Client) -> &'static PSRateLimiter {
    // We need to make an async request when we first construct our rate limiter
    // to determine the rate limit itself.  Note that the way this is currently
    // done is bit dodgy - if multiple clients are constructed in close succession
    // we could end up performing this request multiple times.  That seems like
    // a weird use-case to me, so for now this is fine.
    if let Some(limiter) = PS_CLIENT.get() {
        return limiter;
    }

    #[derive(Deserialize)]
    struct Metadata {
        server_ratelimit_per_minute: NonZeroU32,
    }

    let mut quota = Quota::per_minute(NonZeroU32::new(120).unwrap());
    if let Ok(response) = client.get("https://api.pushshift.io/meta").send().await {
        if let Ok(Metadata {
            server_ratelimit_per_minute,
        }) = response.json().await
        {
            quota = Quota::per_minute(server_ratelimit_per_minute);
        }
    }

    PS_CLIENT.get_or_init(|| RateLimiter::direct(quota))
}

/// An opinionated asynchronous `Client` to make requests to the PushShift API.
///
/// This client is built on top of a [`reqwest::Client`], so as per that documentation
/// is is advised you create a single one and **reuse** it.  In adition, `Client` is both
/// [`Send`] and [`Sync`] so you don't need to wrap it to reuse it.
#[derive(Clone)]
pub struct Client {
    client: reqwest::Client,
    limiter: &'static PSRateLimiter,
}

impl Client {
    /// Creates a new client for the PushShift API.
    ///
    /// # Note
    /// Requests to the PushShift API are rate limited using a global rate limiter.
    /// The first time a client is constructed a request is made to PushShift to
    /// determine the global rate limit.
    ///
    /// # Example
    /// ```rust,no_run
    /// # #[tokio::main]
    /// # async fn main() {
    /// use pullcaps::Client;
    ///
    /// let client = Client::new().await;
    /// # }
    /// ```
    pub async fn new() -> Self {
        Self::with_client(reqwest::Client::new()).await
    }

    /// Creates a new client for the PushShift API with the given backing [`reqwest::Client`].
    ///
    /// # Example
    /// ```rust,no_run
    /// # #[tokio::main]
    /// # async fn main() {
    /// use pullcaps::Client;
    ///
    /// let reqwest_client = reqwest::Client::new();
    ///
    /// // Both clients share the same underlying pool.
    /// let client1 = Client::with_client(reqwest_client.clone());
    /// let client2 = Client::with_client(reqwest_client);
    /// # }
    /// ```
    pub async fn with_client(client: reqwest::Client) -> Self {
        let limiter = rate_limiter(&client).await;
        Self { client, limiter }
    }

    /// Returns a [`Stream`] of [`Comment`]'s matching the given query filter.
    ///
    /// [`Stream`]: futures::Stream
    /// # Example
    /// ```rust
    /// use futures::StreamExt;
    /// use pullcaps::{Client, Filter};
    ///
    /// /// Prints out all comments for the given `user`.
    /// async fn print_user_comments(user: String) {
    ///     let client = Client::new().await;
    ///
    ///     let mut comments = client.get_comments(Filter::new().author(user));
    ///
    ///     while let Some(comment) = comments.next().await {
    ///         println!("test: {}", comment.body);
    ///     }
    /// }
    /// ```
    pub fn get_comments(&self, filter: Filter) -> Pin<Box<dyn Stream<Item = Comment> + '_>> {
        Box::pin(
            self.paginated("https://api.pushshift.io/reddit/comment/search/", filter)
                .flat_map(stream::iter),
        )
    }

    /// Returns a [`Stream`] of [`Post`]'s matching the given query filter.
    ///
    /// [`Stream`]: futures::Stream
    /// # Example
    /// ```rust
    /// use futures::StreamExt;
    /// use pullcaps::{Client, Filter};
    ///
    /// /// Prints out all posts for the given `user`.
    /// async fn print_user_posts(user: String) {
    ///     let client = Client::new().await;
    ///
    ///     let mut posts = client.get_posts(Filter::new().author(user));
    ///
    ///     while let Some(post) = posts.next().await {
    ///         println!("comment page: {}", post.comment_url);
    ///     }
    /// }
    /// ```
    pub fn get_posts(&self, filter: Filter) -> Pin<Box<dyn Stream<Item = Post> + '_>> {
        Box::pin(
            self.paginated("https://api.pushshift.io/reddit/submission/search/", filter)
                .flat_map(stream::iter),
        )
    }

    /// Returns paginated items from the given URL together with the given query parameters.
    /// Any errors that occur during this process will be ignored.
    fn paginated<T, U>(&self, url: U, mut params: Filter) -> impl Stream<Item = Vec<T>> + '_
    where
        T: 'static + DeserializeOwned + AsAttrs,
        U: IntoUrl,
    {
        #[derive(Deserialize, Debug)]
        struct PushShiftResponse<T> {
            data: Vec<T>,
        }

        #[derive(Clone, Serialize)]
        struct PushShiftQueryParams<'a, Q> {
            #[serde(flatten)]
            inner: &'a Q,
            limit: u8,
        }

        let url = url.into_url().unwrap();

        stream! {
            loop {
                let inner_params = PushShiftQueryParams {
                    inner: &params,
                    limit: BATCH_SIZE,
                };

                // Ensure each query is gated behind the rate limiter
                self.limiter.until_ready().await;
                let response = self.client.get(url.clone()).query(&inner_params).send().await;

                if let Ok(response) = response {
                    if let Ok(parsed_response) = response.json::<PushShiftResponse<T>>().await {
                        if let Some(last_content) = parsed_response.data.last() {
                            params = params.before(last_content.attrs().date.clone());
                        } else {
                            break;
                        }

                        yield parsed_response.data;
                        continue;
                    }
                }

                // Something went wrong - to avoid requesting the same data repeatedly
                // we just bail out.
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_is_send_and_sync() {
        fn is_send_and_sync<T: Send + Sync>() {}
        is_send_and_sync::<Client>();
    }
}
