//! Reddit media types.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MediaKind {
    Image,
    Video,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MediaItem {
    pub url: String,
    pub kind: MediaKind,
    /// Poster frame for videos (optional).
    pub poster: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RedditMedia {
    pub items: Vec<MediaItem>,
    pub title: String,
    pub permalink: String,
    pub subreddit: String,
    /// Reddit base36 post id (e.g. `1uuqpvt`), used to re-verify the post is still public.
    pub id: String,
    /// Score at fetch time (for ranking “top” within a window).
    pub score: i64,
}

impl RedditMedia {
    pub fn primary_url(&self) -> &str {
        self.items.first().map(|m| m.url.as_str()).unwrap_or("")
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RedditError {
    InvalidSubreddit,
    Network(String),
    NoImages,
    Parse(String),
}

impl std::fmt::Display for RedditError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RedditError::InvalidSubreddit => {
                write!(f, "Enter a subreddit name or reddit.com/r/… URL")
            }
            RedditError::Network(s) => {
                if s.contains("429") {
                    write!(f, "{s}")
                } else if s.contains("403") {
                    write!(
                        f,
                        "Could not load media (blocked). Wait a moment and try Load again."
                    )
                } else {
                    write!(f, "Could not load media ({s})")
                }
            }
            RedditError::NoImages => {
                write!(
                    f,
                    "No unused public image/video left — try another sub or reload the page"
                )
            }
            RedditError::Parse(s) => write!(f, "Unexpected API response ({s})"),
        }
    }
}
