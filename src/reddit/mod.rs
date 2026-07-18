//! Fetch random top media (images, galleries, videos) from a subreddit.
//!
//! Split for maintainability: types, name normalize, JSON parse, network/probe.

mod normalize;
mod net;
mod parse;
mod types;

pub use normalize::normalize_subreddit;
pub use net::{
    abort_active_fetches, filter_still_available_media, load_media_batch, load_random_image,
    media_seen_in_session, warm_media_cache, LoadTier,
};
#[allow(unused_imports)] // public API for tests / future call sites
pub use parse::extract_images;
pub use types::{MediaItem, MediaKind, RedditMedia};
#[allow(unused_imports)]
pub use types::RedditError;
