use futures::StreamExt;
use pullcaps::{Client, Filter};

#[tokio::main]
async fn main() {
    let client = Client::new();

    let mut comments = client
        .get_comments(Filter::new().author("reddit"))
        .await
        .take(5);
    while let Some(comment) = comments.next().await {
        println!("text: {}", comment.body);
    }
}
