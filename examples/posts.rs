use futures::StreamExt;
use pullcaps::{Client, Filter};

#[tokio::main]
async fn main() {
    let client = Client::new().await;

    let mut posts = client
        .get_posts(Filter::new().subreddit("askreddit"))
        .take(5);
    while let Some(post) = posts.next().await {
        println!("url: {}", post.content_url);
    }
}
