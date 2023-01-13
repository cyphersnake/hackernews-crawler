pub use reqwest::Url;
pub type DateTime = chrono::NaiveDateTime;
pub type PostId = i64;

#[derive(Debug, sqlx::FromRow, Clone, PartialEq, Eq)]
pub struct Post {
    pub post_id: PostId,
    pub title: String,
    pub author: String,
    pub url: String,
    pub link: Option<String>,
    pub publication_moment: DateTime,
    pub last_snapshot_moment: DateTime,
}

#[derive(strum::IntoStaticStr)]
pub enum UserPostRequest {
    All { user: String },
    WasAtFirstPage { user: String },
}
impl UserPostRequest {
    pub fn get_user(&self) -> &str {
        match self {
            UserPostRequest::All { user } => user,
            UserPostRequest::WasAtFirstPage { user } => user,
        }
    }
}
