use async_trait::async_trait;
use futures::stream::BoxStream;

use hackernews_crawler::core::{Post, UserPostRequest};

#[async_trait]
pub trait GetCurrentTopPosts {
    type Error;
    async fn get_current_top_posts<'l>(
        &'l self,
    ) -> Result<BoxStream<'l, Result<Post, Self::Error>>, Self::Error>;
}

#[async_trait]
pub trait GetUserPosts {
    type Error;

    async fn get_user_posts<'l>(
        &'l self,
        filter: UserPostRequest,
    ) -> Result<BoxStream<'l, Result<Post, Self::Error>>, Self::Error>;
}

#[async_trait]
pub trait InsertPost {
    type Error;

    async fn insert_post<'l>(&'l self, post: Post, is_first_page: bool) -> Result<(), Error>;
}

pub mod sqlite {
    use async_trait::async_trait;
    use futures::stream::BoxStream;
    pub use sqlx::sqlite::SqlitePool;

    use super::*;

    #[async_trait]
    impl GetCurrentTopPosts for SqlitePool {
        type Error = sqlx::Error;

        async fn get_current_top_posts<'l>(
            &'l self,
        ) -> Result<BoxStream<'l, Result<Post, Self::Error>>, Self::Error> {
            Ok(Box::pin(
                sqlx::query_as::<_, Post>(r#"
                    SELECT "posts".*
                    FROM "posts"
                    INNER JOIN 
                        "first_page_posts" AS "fpp" ON "posts"."post_id" = "fpp"."post_id" 
                        AND "fpp"."snapshot_moment" = (SELECT MAX("snapshot_moment") FROM "first_page_posts")
                "#).fetch(self),
            ))
        }
    }
    #[async_trait]
    impl GetUserPosts for SqlitePool {
        type Error = sqlx::Error;

        async fn get_user_posts<'l>(
            &'l self,
            filter: UserPostRequest,
        ) -> Result<BoxStream<'l, Result<Post, Self::Error>>, Self::Error> {
            Ok(Box::pin(
                    sqlx::query_as::<_, Post>(
                    r#"SELECT *
                        FROM "posts"
                        WHERE "author" = ?1
                          AND CASE ?2
                                  WHEN 'WasAtFirstPage' THEN "post_id" IN (SELECT "post_id" FROM "first_page_posts")
                                  WHEN 'All' THEN TRUE
                                  ELSE FALSE
                          END
                        ORDER BY "publication_moment"
                        "#,
                    )
                    .bind(filter.get_user().to_string())
                    .bind(<&'static str>::from(filter))
                .fetch(self)
            ))
        }
    }

    #[async_trait]
    impl InsertPost for SqlitePool {
        type Error = sqlx::Error;

        async fn insert_post<'l>(
            &'l self,
            post: Post,
            is_first_page: bool,
        ) -> Result<(), Self::Error> {
            sqlx::query!(
                r#"
                    INSERT INTO
                        "posts_view" ("post_id", "title", "author", "url", "link", "publication_moment", "last_snapshot_moment", "was_at_first_page")
                    VALUES
                        (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8);
                "#,
                post.post_id,
                post.title,
                post.author,
                post.url,
                post.link,
                post.publication_moment,
                post.last_snapshot_moment,
                is_first_page,
            )
            .execute(self)
            .await?;

            Ok(())
        }
    }

    #[cfg(test)]
    mod test {
        use futures::StreamExt;

        use super::*;

        fn get_rnd_post() -> Post {
            use rand::Rng;
            let mut rnd = rand::thread_rng();

            Post {
                post_id: rnd.gen(),
                title: "test".to_owned(),
                author: "test".to_owned(),
                url: "test".to_owned(),
                link: None,
                publication_moment: chrono::Local::now().naive_utc(),
                last_snapshot_moment: chrono::Local::now().naive_utc(),
            }
        }

        async fn get_storage() -> SqlitePool {
            let storage = SqlitePool::connect(":memory:").await.unwrap();

            sqlx::migrate!()
                .run(&mut storage.acquire().await.unwrap())
                .await
                .unwrap();

            storage
        }

        #[tokio::test]
        async fn test_consistency() {
            let storage = get_storage().await;

            let post = Post {
                author: "test_consistency".to_owned(),
                publication_moment: chrono::Local::now().naive_utc(),
                ..get_rnd_post()
            };

            let fp_post = Post {
                author: "test_consistency".to_owned(),
                publication_moment: chrono::Local::now().naive_utc(),
                ..get_rnd_post()
            };

            storage.insert_post(post.clone(), false).await.unwrap();
            storage.insert_post(fp_post.clone(), true).await.unwrap();

            let posts = storage
                .get_user_posts(UserPostRequest::All {
                    user: post.author.clone(),
                })
                .await
                .unwrap()
                .map(Result::unwrap)
                .collect::<Vec<_>>()
                .await;
            assert_eq!(posts, vec![post, fp_post.clone()]);

            let posts = storage
                .get_user_posts(UserPostRequest::WasAtFirstPage {
                    user: fp_post.author.clone(),
                })
                .await
                .unwrap()
                .map(Result::unwrap)
                .collect::<Vec<_>>()
                .await;
            assert_eq!(posts, vec![fp_post]);
        }

        #[tokio::test]
        async fn test_first_page() {
            let storage = get_storage().await;

            let last_snapshot_moment = chrono::Local::now().naive_utc();
            for post in (0..100).map(|post_id| Post {
                post_id,
                last_snapshot_moment,
                ..get_rnd_post()
            }) {
                storage
                    .insert_post(post.clone(), post.post_id < 50)
                    .await
                    .unwrap();
            }

            {
                let top_posts_ids = storage
                    .get_current_top_posts()
                    .await
                    .unwrap()
                    .map(Result::unwrap)
                    .map(|post| post.post_id)
                    .collect::<Vec<_>>()
                    .await;
                assert_eq!(
                    top_posts_ids,
                    (0..50).collect::<Vec<_>>(),
                    "failed to validate top page after first snapshot"
                );
            }

            let last_snapshot_moment = chrono::Local::now().naive_utc();
            for post in (100..200).map(|post_id| Post {
                post_id,
                last_snapshot_moment,
                ..get_rnd_post()
            }) {
                storage
                    .insert_post(post.clone(), post.post_id >= 150)
                    .await
                    .unwrap();
            }

            {
                let top_posts_ids = storage
                    .get_current_top_posts()
                    .await
                    .unwrap()
                    .map(Result::unwrap)
                    .map(|post| post.post_id)
                    .collect::<Vec<_>>()
                    .await;
                assert_eq!(
                    top_posts_ids,
                    (150..200).collect::<Vec<_>>(),
                    "failed to validate top page after second snapshot"
                );
            }
        }
    }
}

pub type Storage = sqlite::SqlitePool;
pub type Error = sqlx::Error;
