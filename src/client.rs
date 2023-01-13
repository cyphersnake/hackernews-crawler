use clap::Parser;

use futures::stream::StreamExt;
use hackernews_crawler::{
    core::UserPostRequest,
    hackernews_proxy_proto::{post_service_client::PostServiceClient, TopPostRequest},
};
use tonic::transport::Channel;

#[derive(clap::Parser, Debug)]
struct Args {
    #[arg(short, long, default_value = "http://0.0.0.0:7777")]
    address: String,
    #[command(subcommand)]
    action: Action,
}

#[allow(clippy::enum_variant_names)]
#[derive(clap::Subcommand, Debug)]
enum Action {
    TopPosts,
    UserPosts { user: String },
    UserTopPosts { user: String },
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    tracing::info!("Run with args {args:?}");

    let channel = Channel::builder(
        args.address
            .parse()
            .expect("Failed to parse address from args"),
    )
    .connect()
    .await
    .unwrap();

    let mut client = PostServiceClient::new(channel);

    let mut stream = match args.action {
        Action::TopPosts => {
            client
                .get_top_posts(tonic::Request::new(TopPostRequest {}))
                .await
        }
        Action::UserPosts { user } => {
            client
                .get_user_posts(tonic::Request::new(UserPostRequest::All { user }.into()))
                .await
        }
        Action::UserTopPosts { user } => {
            client
                .get_user_posts(tonic::Request::new(
                    UserPostRequest::WasAtFirstPage { user }.into(),
                ))
                .await
        }
    }
    .expect("Failed to get posts stream from server")
    .into_inner();

    while let Some(post) = stream.next().await {
        println!(
            "{:?}",
            <Result<hackernews_crawler::core::Post, _>>::from(
                post.expect("wrong post provided from server")
            )
            .unwrap()
        );
    }
}
