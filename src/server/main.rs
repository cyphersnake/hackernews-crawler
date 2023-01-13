// You should implement a web crawler for Hacker News. Crawl the first 10 pages of Hacker News and store enough information to be able to respond to the following through some APIs:
// 1.    List of top posts.
// 2.    List of posts sent by a user.
// 3.    List of posts of a user that was on the first page at some point.
//
// DO NOT spend too much time on this question (max 4-5 hours). This question is open ended. What we are curious to see is your use of:
// 1.    Database use
// 2.    API design
// 3.    Concurrency
// 4.    Testing
// 5.    Containerisation and ease of deployment
//
// It is not necessary to have a finished project, but it should be at a state where we can reliably get signals for above and to know that you are experienced in topics that will be needed during the job.
//
// For example, there's no need to have significant coverage, but we expect to see sort of mocking during some test.
//
// Requirements:
// 1. Use Rust.
// 2. Use a relational database.
/// Module with external api
mod api;

/// Module with scrapper for hackernews website
mod hackernews_scrapper;
mod posts_storage;

use std::{net::SocketAddr, sync::Arc, time::Duration};

use confique::Config;
use futures::{future::BoxFuture, StreamExt};
use posts_storage::{InsertPost, Storage};

#[derive(Debug, Config)]
struct Configuration {
    #[config(env = "GRPC_SERVER_ADDRESS", default = "0.0.0.0:7777")]
    bind_address: SocketAddr,
    #[config(env = "DATABASE_URL", default = "sqlite:posts.db")]
    sqlite_connect_str: String,
    #[config(env = "SCRAPPER_TIMEOUT_MILLIS", default = 1500)]
    scrapper_timeout_millis: u64,
    #[config(env = "SNAPSHOT_TIMEOUT_SECS", default = 60)]
    snapshot_timeout_secs: u64,
}

struct App {
    posts_storage: Arc<Storage>,
    scrapper: hackernews_scrapper::HackernewsScraper,
    scapper_timeout: Duration,
    snapshot_timeout: Duration,
    server: BoxFuture<'static, Result<(), tonic::transport::Error>>,
}

#[derive(thiserror::Error, Debug)]
enum Error {
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error(transparent)]
    Transport(#[from] tonic::transport::Error),
    #[error(transparent)]
    Runtime(#[from] tokio::task::JoinError),
}

impl App {
    async fn new(
        addr: SocketAddr,
        storage_connect_str: &str,
        scapper_timeout: Duration,
        snapshot_timeout: Duration,
    ) -> Result<Self, Error> {
        let posts_storage = Arc::new(
            posts_storage::Storage::connect(storage_connect_str)
                .await
                .expect("Failed to connect database"),
        );

        sqlx::migrate!()
            .run(&mut posts_storage.acquire().await?)
            .await
            .unwrap();

        Ok(Self {
            posts_storage: posts_storage.clone(),
            scrapper: hackernews_scrapper::HackernewsScraper::default(),
            scapper_timeout,
            snapshot_timeout,
            server: Box::pin(
                tonic::transport::Server::builder()
                    .accept_http1(true)
                    .add_service(
                        hackernews_crawler::proto::post_service_server::PostServiceServer::new(
                            api::Server { posts_storage },
                        ),
                    )
                    .serve(addr),
            ),
        })
    }

    pub async fn run(self) -> Result<(), Error> {
        let server_task = tokio::spawn(self.server);

        loop {
            if server_task.is_finished() {
                match server_task.await? {
                    Ok(()) => unreachable!("This grpc-task never stops"),
                    Err(err) => {
                        return Err(err.into());
                    }
                }
            }

            let mut collector = self.scrapper.new_collector(self.scapper_timeout);

            while let Some(output) = collector.next().await {
                if let Ok((page, post)) = output {
                    self.posts_storage
                        .insert_post(post, page == 1)
                        .await
                        .unwrap();
                }
            }

            tokio::time::sleep(self.snapshot_timeout).await;
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt().init();

    let config = Configuration::builder().file("config.toml").env().load()?;

    let app = App::new(
        config.bind_address,
        &config.sqlite_connect_str,
        Duration::from_millis(config.scrapper_timeout_millis),
        Duration::from_secs(config.snapshot_timeout_secs),
    )
    .await?;

    Ok(app.run().await?)
}
