use futures::StreamExt;
use pullcaps::{Client, Filter, SortType};

#[tokio::main]
async fn main() {
    let client = Client::new();

    let mut posts = client
        .get_posts(
            Filter::new()
                .subreddit("askreddit")
                .sort_type(SortType::Score),
        )
        .await
        .take(5);

    while let Some(post) = posts.next().await {
        println!("url: {}", post.content_url);
    }
}
