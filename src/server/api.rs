use std::fmt::Debug;
use std::sync::Arc;

use futures::{Stream, StreamExt};
use tokio::sync::mpsc::UnboundedSender;
use tokio_stream::wrappers::UnboundedReceiverStream;
use tonic::Status;

use crate::posts_storage::{GetCurrentTopPosts, GetUserPosts};
use hackernews_crawler::{hackernews_core, hackernews_proxy_proto as proto};

pub struct Server<S: GetCurrentTopPosts + GetUserPosts> {
    pub posts_storage: Arc<S>,
}

async fn handle_posts_stream<E>(
    stream: impl Stream<Item = Result<hackernews_core::Post, E>>,
    sender: UnboundedSender<Result<proto::Post, Status>>,
) where
    E: Debug,
{
    stream
        .map(|result_with_post| match result_with_post {
            Ok(post) => Ok(proto::Post::from(post)),
            Err(err) => Err(Status::internal(format!(
                "error while deserliaze post: {err:?}" // TODO Hide from user
            ))),
        })
        .for_each(|result_with_post| async {
            if let Err(err) = sender.send(result_with_post) {
                tracing::error!("internal error while send err-response to get_top_posts: {err:?}");
            }
        })
        .await;
}

#[tonic::async_trait]
impl<S: GetCurrentTopPosts + GetUserPosts> proto::post_service_server::PostService for Server<S>
where
    S: 'static + Send + Sync,
    <S as GetUserPosts>::Error: ToString + Debug, // TODO: Mapping to Status
    <S as GetCurrentTopPosts>::Error: ToString + Debug, // TODO: Mappinc to Status
{
    type GetTopPostsStream = UnboundedReceiverStream<Result<proto::Post, Status>>;
    type GetUserPostsStream = UnboundedReceiverStream<Result<proto::Post, Status>>;

    async fn get_top_posts(
        &self,
        _request: tonic::Request<proto::TopPostRequest>,
    ) -> Result<tonic::Response<Self::GetTopPostsStream>, Status> {
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();

        let posts_storage = self.posts_storage.clone();
        // We cannot return the original stream, because it has a link
        // to the sqlx pool but cannot own it. I did not find offhand
        // a way to "cheat" fetch call inside sqlx, so I created a bidirectional
        // channel, it would have been more time, most likely would have made it easier
        let _task = tokio::task::spawn(async move {
            let stream = match posts_storage.get_current_top_posts().await {
                Ok(stream) => stream,
                Err(err) => {
                    if let Err(err) = sender.send(Err(Status::internal(err.to_string()))) {
                        tracing::error!(
                            "internal error while send err-response to get_top_posts: {err:?}"
                        );
                    }
                    return;
                }
            };

            handle_posts_stream(stream, sender).await;
        });

        // A more correct way is to return a wrapper over this stream to
        // also store and stop the tokio task not through an error when
        // the receiver is killed, however, let's leave this as a potential
        // improvement
        Ok(tonic::Response::new(UnboundedReceiverStream::new(receiver)))
    }

    async fn get_user_posts(
        &self,
        request: tonic::Request<proto::UserPostRequest>,
    ) -> Result<tonic::Response<Self::GetUserPostsStream>, tonic::Status> {
        let request = Option::<hackernews_core::UserPostRequest>::from(request.into_inner())
            .ok_or_else(|| Status::invalid_argument("Please provide request detail"))?;

        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();

        let posts_storage = self.posts_storage.clone();
        // We cannot return the original stream, because it has a link
        // to the sqlx pool but cannot own it. I did not find offhand
        // a way to "cheat" fetch inside sqlx, so I created a bidirectional
        // channel, it would have been more time, most likely would have made it easier
        let _task = tokio::task::spawn(async move {
            let stream = match posts_storage.get_user_posts(request).await {
                Ok(stream) => stream,
                Err(err) => {
                    if let Err(err) = sender.send(Err(Status::internal(err.to_string()))) {
                        tracing::error!(
                            "internal error while send err-response to get_user_posts: {err:?}"
                        );
                    }
                    return;
                }
            };

            handle_posts_stream(stream, sender).await;
        });

        // A more correct way is to return a wrapper over this stream to
        // also store and stop the tokio task not through an error when
        // the receiver is killed, however, let's leave this as a potential
        // improvement
        Ok(tonic::Response::new(UnboundedReceiverStream::new(receiver)))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use futures::stream::BoxStream;
    use tonic::codegen::Service;

    use hackernews_crawler::hackernews_core::{Post, UserPostRequest};

    use super::*;

    #[derive(Debug, Default)]
    struct StorageMock {
        pub users_posts: HashMap<String, Post>,
        pub top_posts: Vec<Vec<Post>>,
    }

    impl StorageMock {
        // TODO Migrate to RAII
        fn assert_ready(&self) {
            assert!(self.users_posts.is_empty());
            assert!(self.top_posts.is_empty());
        }
    }

    #[async_trait::async_trait]
    impl GetUserPosts for StorageMock {
        type Error = sqlx::Error;

        async fn get_user_posts<'l>(
            &'l self,
            _filter: UserPostRequest,
        ) -> Result<BoxStream<'l, Result<Post, Self::Error>>, Self::Error> {
            todo!("validate the correctness of the request and return user posts")
        }
    }

    #[async_trait::async_trait]
    impl GetCurrentTopPosts for StorageMock {
        type Error = sqlx::Error;

        async fn get_current_top_posts<'l>(
            &'l self,
        ) -> Result<BoxStream<'l, Result<Post, Self::Error>>, Self::Error> {
            todo!("validate the correctness of the request and return current top posts")
        }
    }

    #[test]
    fn test_get_top_posts() {
        let mock = Arc::new(StorageMock::default());
        use hackernews_crawler::proto::post_service_server::PostServiceServer;
        PostServiceServer::new(Server {
            posts_storage: mock.clone(),
        })
        .call(tonic::codegen::http::Request::<_>::new(
            "TODO, Mock Request".to_owned(),
        ));
        mock.assert_ready();
    }

    #[test]
    fn test_get_user_posts() {
        let mock = Arc::new(StorageMock::default());
        use hackernews_crawler::proto::post_service_server::PostServiceServer;
        PostServiceServer::new(Server {
            posts_storage: mock.clone(),
        })
        .call(tonic::codegen::http::Request::<_>::new(
            "TODO, Mock Request".to_owned(),
        ));
        mock.assert_ready();
    }
}
