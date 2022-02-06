use crate::models::{AsAttrs, Comment, Post};
use crate::{Filter, SortType};
use async_stream::stream;
use chrono::{DateTime, Duration, Utc};
use futures::stream::{self, select_all, Stream, StreamExt};
use governor::{Quota, RateLimiter};
use once_cell::sync::OnceCell;
use reqwest::{IntoUrl, Url};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::num::NonZeroU32;
use std::ops::Div;
use std::pin::Pin;

type PSRateLimiter = RateLimiter<
    governor::state::NotKeyed,
    governor::state::InMemoryState,
    governor::clock::DefaultClock,
    governor::middleware::NoOpMiddleware,
>;

const BATCH_SIZE: i64 = 50;
const DESIRED_BUCKET_VOLUME: i64 = 25;

/// A global rate limiter, used to limit PS API queries to 1 per second.
fn rate_limiter() -> &'static PSRateLimiter {
    static PS_RATE_LIMITER: OnceCell<PSRateLimiter> = OnceCell::new();
    PS_RATE_LIMITER
        .get_or_init(|| RateLimiter::direct(Quota::per_second(NonZeroU32::new(1).unwrap())))
}

#[derive(Deserialize, Debug)]
struct PushShiftMetadata {
    total_results: i64,
}

#[derive(Deserialize, Debug)]
struct PushShiftResponse<T> {
    data: Vec<T>,
    metadata: Option<PushShiftMetadata>,
}

#[derive(Clone, Serialize)]
struct PushShiftQueryParams<'a> {
    #[serde(flatten)]
    inner: &'a Filter,
    sort: Option<&'static str>,
    limit: i64,
    metadata: bool,
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
    /// ```rust
    /// use pullcaps::Client;
    ///
    /// let client = Client::new();
    /// ```
    pub fn new() -> Self {
        Self::with_client(reqwest::Client::new())
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
    pub fn with_client(client: reqwest::Client) -> Self {
        Self {
            client,
            limiter: rate_limiter(),
        }
    }

    /// Returns a [`Stream`] of [`Comment`]'s matching the given query filter.
    ///
    /// [`Stream`]: futures::Stream
    ///
    /// # Ordering
    ///
    /// This method chunks the search space up into many different buckets which
    /// are independently queried - as such there is no guarantee of the order
    /// that results are returned.
    ///
    /// # Example
    /// ```rust,no_run
    /// # async fn example() {
    /// use futures::StreamExt;
    /// use pullcaps::{Client, Filter};
    ///
    /// let client = Client::new();
    ///
    /// let mut comments = client.get_comments(Filter::new().author("reddit")).await;
    ///
    /// while let Some(comment) = comments.next().await {
    ///     println!("test: {}", comment.body);
    /// }
    /// # }
    /// ```
    pub async fn get_comments(&self, filter: Filter) -> Pin<Box<dyn Stream<Item = Comment> + '_>> {
        let url = Url::parse("https://api.pushshift.io/reddit/comment/search/").unwrap();
        self._stream(url, filter).await
    }

    /// Returns a [`Stream`] of [`Post`]'s matching the given query filter.
    ///
    /// [`Stream`]: futures::Stream
    ///
    /// # Ordering
    ///
    /// This method chunks the search space up into many different buckets which
    /// are independently queried - as such there is no guarantee of the order
    /// that results are returned.
    ///
    /// # Example
    /// ```rust,no_run
    /// # async fn example() {
    /// use futures::StreamExt;
    /// use pullcaps::{Client, Filter, SortType};
    ///
    /// let client = Client::new();
    ///
    /// // Gets the 25 highest score posts in /r/aww
    /// let mut posts = client
    ///     .get_posts(Filter::new().subreddit("aww").sort_type(SortType::Score))
    ///     .await
    ///     .take(25);
    ///
    /// while let Some(post) = posts.next().await {
    ///     println!("comment page: {}", post.comment_url);
    /// }
    /// # }
    /// ```
    pub async fn get_posts(&self, filter: Filter) -> Pin<Box<dyn Stream<Item = Post> + '_>> {
        let url = Url::parse("https://api.pushshift.io/reddit/submission/search/").unwrap();
        self._stream(url, filter).await
    }

    /// Creates a [`Stream`], either chunked or unchunked depending on the context.
    async fn _stream<T: 'static + DeserializeOwned + AsAttrs>(
        &self,
        url: Url,
        filter: Filter,
    ) -> Pin<Box<dyn Stream<Item = T> + '_>> {
        if matches!(filter.sort_type, SortType::CreatedDate) {
            // TODO: for now we only implement chunked requests for filters
            //       that sort by date; we'd need a similar sort of logic
            //       to chunk requests based on the other attributes.
            if let Some((total, oldest, newest)) =
                self.get_date_bounds::<Post>(url.clone(), &filter).await
            {
                return Box::pin(
                    select_all(chunked(total, oldest, newest).map(|(l, r)| {
                        Box::pin(self.paginated(url.clone(), filter.clone().before(r).after(l)))
                    }))
                    .flat_map(stream::iter),
                );
            }
        }

        Box::pin(self.paginated(url, filter).flat_map(stream::iter))
    }

    /// Performs a single request to the PushShift API, returning the deserialized result.
    async fn _get<T: DeserializeOwned>(
        &self,
        url: Url,
        params: PushShiftQueryParams<'_>,
    ) -> Option<PushShiftResponse<T>> {
        self.limiter.until_ready().await;
        let response = self.client.get(url).query(&params).send().await;

        if let Ok(response) = response {
            if let Ok(parsed_response) = response.json::<PushShiftResponse<T>>().await {
                return Some(parsed_response);
            }
        }

        None
    }

    /// Determines the oldest and most recent dates of items corresponding to this query,
    /// together with the total number of matching items.
    async fn get_date_bounds<'a, T: DeserializeOwned + AsAttrs>(
        &self,
        url: Url,
        params: &Filter,
    ) -> Option<(i64, DateTime<Utc>, DateTime<Utc>)> {
        let newest: PushShiftResponse<T> = self
            ._get(
                url.clone(),
                PushShiftQueryParams {
                    inner: params,
                    sort: Some("desc"),
                    limit: 1,
                    metadata: true,
                },
            )
            .await?;

        // Only want to do this for queries with lots of results.
        let total_results = if let Some(metadata) = &newest.metadata {
            metadata.total_results
        } else {
            return None;
        };

        if total_results <= BATCH_SIZE {
            return None;
        }

        let oldest: PushShiftResponse<T> = self
            ._get(
                url,
                PushShiftQueryParams {
                    inner: params,
                    sort: Some("asc"),
                    limit: 1,
                    metadata: false,
                },
            )
            .await?;
        Some((
            total_results,
            oldest.data[0].attrs().date,
            newest.data[0].attrs().date,
        ))
    }

    /// Returns paginated items from the given URL together with the given query parameters.
    /// Any errors that occur during this process will be ignored.
    fn paginated<T, U>(&self, url: U, mut params: Filter) -> impl Stream<Item = Vec<T>> + '_
    where
        T: 'static + DeserializeOwned + AsAttrs,
        U: IntoUrl,
    {
        let url = url.into_url().unwrap();

        stream! {
            loop {
                let inner_params = PushShiftQueryParams {
                    inner: &params,
                    sort: None,
                    limit: BATCH_SIZE,
                    metadata: false,
                };

                if let Some(parsed_response) = self._get::<T>(url.clone(), inner_params).await {
                    if let Some(last_content) = parsed_response.data.last() {
                        params = params.before(last_content.attrs().date.clone());
                    } else {
                        break;
                    }

                    // If we got less than the batch size of results then there's
                    // not going to be any more results in the next query.
                    let should_break = parsed_response.data.len() < BATCH_SIZE as usize;

                    yield parsed_response.data;

                    if should_break {
                        break;
                    } else {
                        continue;
                    }
                }

                break;
            }
        }
    }
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}

fn chunked(
    total: i64,
    oldest: DateTime<Utc>,
    newest: DateTime<Utc>,
) -> impl Iterator<Item = (DateTime<Utc>, DateTime<Utc>)> {
    // We make the (somewhat suspicious) assumption that posts are evenly distributed
    // through time.  Chunk the problem down into buckets where, assuming posts _are_
    // evenly distributed, we expect around 50 posts. We also put up upper bound
    // of 200 chunks to avoid creating an enormous amount of streams.
    let buckets = (total / DESIRED_BUCKET_VOLUME).min(200);
    let bucket_width = (newest - oldest).div((buckets + 1) as i32).num_seconds();

    (0..=buckets).map(move |c| {
        let l = if c == 0 {
            oldest
        } else {
            oldest + Duration::seconds((c * bucket_width) + 1)
        };
        let r = if c == buckets {
            newest
        } else {
            oldest + Duration::seconds((c + 1) * bucket_width)
        };
        (l, r)
    })
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
