# pullcaps

[![crates.io](https://img.shields.io/crates/v/pullcaps.svg)](https://crates.io/crates/pullcaps)
[![Documentation](https://docs.rs/pullcaps/badge.svg)](https://docs.rs/pullcaps)
[![MIT/Apache-2 licensed](https://img.shields.io/crates/l/pullcaps.svg)](./LICENSE-APACHE)
[![CI](https://github.com/dcchut/pullcaps/workflows/CI/badge.svg)](https://github.com/dcchut/pullcaps/actions?query=workflow%3ACI)

A convenient, opinionated, asynchronous client for the [PushShift API](https://pushshift.io).

# Example

This library is built on top of [Tokio](https://tokio.rs/), and currently produces streams
from the [futures](https://rust-lang.github.io/futures-rs/) crate.  An example `Cargo.toml` could be:

```toml
[dependencies]
futures = { version = "0.3" }
pullcaps = { version = "0.1" }
tokio = { version = "1", features = ["full"] }
```

A small example is then:

```rust
use pullcaps::{Client, Filter};
use futures::StreamExt;

#[tokio::main]
async fn main() {
  let client = Client::new().await;
    
  // Get the five most recent posts in /r/askreddit
  let mut posts = client.get_posts(Filter::new().subreddit("askreddit")).take(5);
  while let Some(post) = posts.next().await {
    println!("url: {}", post.content_url);
  }
}
```

For additional examples see the [documentation](https://docs.rs/pullcaps).

### License
Licensed under either of
* Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)
