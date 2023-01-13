use crate::hackernews_core::{self, DateTime};

tonic::include_proto!("hackernews_proxy");

impl From<Timestamp> for DateTime {
    fn from(value: Timestamp) -> Self {
        DateTime::from_timestamp_opt(value.timestmap, 0)
            .expect("Need error handling, but will neglect this at this stage")
    }
}
impl From<DateTime> for Timestamp {
    fn from(value: DateTime) -> Self {
        Timestamp {
            timestmap: value.timestamp(),
        }
    }
}
impl From<StringWrapper> for String {
    fn from(value: StringWrapper) -> Self {
        value.str
    }
}
impl From<String> for StringWrapper {
    fn from(value: String) -> Self {
        StringWrapper { str: value }
    }
}

#[derive(Debug)]
pub enum Error {
    LostLink,
    WrongUrl(url::ParseError),
    LostSnapshotTime,
    LostPublicationTime,
}

impl From<hackernews_core::Post> for Post {
    fn from(value: hackernews_core::Post) -> Post {
        Post {
            post_id: value.post_id,
            title: value.title,
            author: value.author,
            link: value.link.map(Into::into),
            url: value.url,
            publication_moment: Some(value.publication_moment.into()),
            last_snapshot_moment: Some(value.last_snapshot_moment.into()),
        }
    }
}
impl From<Post> for Result<hackernews_core::Post, Error> {
    fn from(value: Post) -> Result<hackernews_core::Post, Error> {
        Ok(hackernews_core::Post {
            post_id: value.post_id,
            title: value.title,
            author: value.author,
            url: value.url,
            link: value.link.map(Into::into),
            publication_moment: value
                .publication_moment
                .ok_or(Error::LostPublicationTime)?
                .into(),
            last_snapshot_moment: value
                .last_snapshot_moment
                .ok_or(Error::LostSnapshotTime)?
                .into(),
        })
    }
}
impl From<hackernews_core::UserPostRequest> for UserPostRequest {
    fn from(value: hackernews_core::UserPostRequest) -> Self {
        match value {
            hackernews_core::UserPostRequest::All { user } => Self {
                user,
                filter: Some(user_post_request::Filter::All(Empty {})),
            },
            hackernews_core::UserPostRequest::WasAtFirstPage { user } => Self {
                user,
                filter: Some(user_post_request::Filter::WasAtFirstPage(Empty {})),
            },
        }
    }
}

impl From<UserPostRequest> for Option<hackernews_core::UserPostRequest> {
    fn from(value: UserPostRequest) -> Self {
        match value {
            UserPostRequest {
                user: _,
                filter: None,
            } => None,
            UserPostRequest {
                user,
                filter: Some(user_post_request::Filter::All(_)),
            } => Some(hackernews_core::UserPostRequest::All { user }),
            UserPostRequest {
                user,
                filter: Some(user_post_request::Filter::WasAtFirstPage(_)),
            } => Some(hackernews_core::UserPostRequest::WasAtFirstPage { user }),
        }
    }
}
