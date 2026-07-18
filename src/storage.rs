//! Persistence via localStorage / sessionStorage (GitHub Pages, no server).

#[cfg(target_arch = "wasm32")]
use crate::difficulty::{clamp_target, DEFAULT_TARGET};
#[cfg(not(target_arch = "wasm32"))]
use crate::difficulty::DEFAULT_TARGET;
use crate::game::BoardSnapshot;
use crate::reddit::RedditMedia;
use crate::subreddits::SubredditPool;
use serde::{Deserialize, Serialize};

const BEST_KEY: &str = "rust-game-2048-best";
const SUB_KEY: &str = "rust-game-2048-subreddit";
const GOAL_KEY: &str = "rust-game-2048-goal";
const SESSION_SEEN_KEY: &str = "rust-game-2048-session-seen";
const POOL_KEY: &str = "rust-game-2048-sub-pool";
const GAME_SESSION_KEY: &str = "rust-game-2048-game-session-v1";
const GALLERY_KEY: &str = "rust-game-2048-gallery-v1";
const GOOD_SUBS_KEY: &str = "rust-game-2048-good-subs-v1";

/// Cap concurrent-ish media history in the session gallery.
pub const GALLERY_MAX: usize = 24;
/// Cap durable "good sub" cache across visits.
pub const GOOD_SUBS_MAX: usize = 40;

/// Mid-game snapshot restored after refresh (sessionStorage — same tab).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GameSession {
    pub board: BoardSnapshot,
    pub reveal_from: u32,
    pub reveal_to: u32,
    pub media: Option<RedditMedia>,
    pub slide_index: usize,
    pub goal: u32,
}

/// Unlocked post in the session gallery.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GalleryEntry {
    pub media: RedditMedia,
    pub goal: u32,
    pub max_tile: u32,
}

fn ls_get(key: &str) -> Option<String> {
    #[cfg(target_arch = "wasm32")]
    {
        web_sys::window()
            .and_then(|w| w.local_storage().ok().flatten())
            .and_then(|s| s.get_item(key).ok().flatten())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = key;
        None
    }
}

fn ls_set(key: &str, val: &str) {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(storage) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
            let _ = storage.set_item(key, val);
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (key, val);
    }
}

fn ss_get(key: &str) -> Option<String> {
    #[cfg(target_arch = "wasm32")]
    {
        web_sys::window()
            .and_then(|w| w.session_storage().ok().flatten())
            .and_then(|s| s.get_item(key).ok().flatten())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = key;
        None
    }
}

fn ss_set(key: &str, val: &str) {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(storage) = web_sys::window().and_then(|w| w.session_storage().ok().flatten()) {
            let _ = storage.set_item(key, val);
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (key, val);
    }
}

fn ss_remove(key: &str) {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(storage) = web_sys::window().and_then(|w| w.session_storage().ok().flatten()) {
            let _ = storage.remove_item(key);
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = key;
    }
}

pub fn load_best() -> u32 {
    ls_get(BEST_KEY)
        .and_then(|v| v.parse().ok())
        .unwrap_or(0)
}

pub fn save_best(score: u32) {
    ls_set(BEST_KEY, &score.to_string());
}

pub fn load_subreddit() -> String {
    ls_get(SUB_KEY).unwrap_or_default()
}

pub fn save_subreddit(sub: &str) {
    ls_set(SUB_KEY, sub);
}

pub fn load_subreddit_pool() -> SubredditPool {
    ls_get(POOL_KEY)
        .map(|v| SubredditPool::from_str(&v))
        .unwrap_or(SubredditPool::Sfw)
}

pub fn save_subreddit_pool(pool: SubredditPool) {
    ls_set(POOL_KEY, pool.as_str());
}

pub fn load_goal() -> u32 {
    let raw = ls_get(GOAL_KEY)
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_TARGET);
    #[cfg(target_arch = "wasm32")]
    {
        clamp_target(raw)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = raw;
        DEFAULT_TARGET
    }
}

pub fn save_goal(goal: u32) {
    ls_set(GOAL_KEY, &goal.to_string());
}

/// URLs already shown this browser session (sessionStorage — clears on tab close).
pub fn load_session_seen_urls() -> Vec<String> {
    ss_get(SESSION_SEEN_KEY)
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

/// Record every media URL from a post as used this session.
pub fn mark_session_seen_urls(urls: &[String]) {
    if urls.is_empty() {
        return;
    }
    let mut list = load_session_seen_urls();
    for u in urls {
        list.retain(|x| x != u);
        list.insert(0, u.clone());
    }
    list.truncate(500);
    if let Ok(raw) = serde_json::to_string(&list) {
        ss_set(SESSION_SEEN_KEY, &raw);
    }
}

pub fn push_recent_media_urls(urls: impl IntoIterator<Item = String>) {
    let v: Vec<String> = urls.into_iter().collect();
    mark_session_seen_urls(&v);
}

// --- Session restore (item 7) ---

pub fn load_game_session() -> Option<GameSession> {
    let raw = ss_get(GAME_SESSION_KEY)?;
    serde_json::from_str(&raw).ok()
}

pub fn save_game_session(session: &GameSession) {
    if let Ok(raw) = serde_json::to_string(session) {
        ss_set(GAME_SESSION_KEY, &raw);
    }
}

pub fn clear_game_session() {
    ss_remove(GAME_SESSION_KEY);
}

// --- Gallery of unlocked posts (item 17) ---

pub fn load_gallery() -> Vec<GalleryEntry> {
    ss_get(GALLERY_KEY)
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

pub fn push_gallery_entry(entry: GalleryEntry) {
    let mut list = load_gallery();
    let already = list.iter().any(|e| {
        (!entry.media.id.is_empty() && e.media.id == entry.media.id)
            || e.media.primary_url() == entry.media.primary_url()
    });
    // Already unlocked — keep its position (do not move on re-view / click).
    if already {
        return;
    }
    list.insert(0, entry);
    list.truncate(GALLERY_MAX);
    save_gallery(&list);
}

/// Replace the whole gallery (e.g. after pruning deleted posts). Preserves caller order.
pub fn save_gallery(entries: &[GalleryEntry]) {
    let mut list = entries.to_vec();
    list.truncate(GALLERY_MAX);
    if let Ok(raw) = serde_json::to_string(&list) {
        ss_set(GALLERY_KEY, &raw);
    }
}

// --- Durable good-subs cache (item 9) ---

pub fn load_good_subs() -> Vec<String> {
    ls_get(GOOD_SUBS_KEY)
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

/// Remember a sub that successfully delivered media (cross-session).
pub fn remember_good_sub(name: &str) {
    let name = name.trim();
    if name.is_empty() {
        return;
    }
    let key = name.to_ascii_lowercase();
    let mut list = load_good_subs();
    list.retain(|s| s.to_ascii_lowercase() != key);
    list.insert(0, name.to_string());
    list.truncate(GOOD_SUBS_MAX);
    if let Ok(raw) = serde_json::to_string(&list) {
        ls_set(GOOD_SUBS_KEY, &raw);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::{BoardSnapshot, GameStatus, Tile};

    #[test]
    fn game_session_roundtrip_json() {
        let session = GameSession {
            board: BoardSnapshot {
                tiles: vec![Tile {
                    id: 1,
                    value: 2,
                    row: 0,
                    col: 0,
                    is_new: false,
                    is_merged: false,
                }],
                next_id: 2,
                score: 0,
                status: GameStatus::Playing,
                win_tile: 256,
                won_once: false,
            },
            reveal_from: 2,
            reveal_to: 256,
            media: None,
            slide_index: 0,
            goal: 256,
        };
        let raw = serde_json::to_string(&session).unwrap();
        let back: GameSession = serde_json::from_str(&raw).unwrap();
        assert_eq!(back.goal, 256);
        assert_eq!(back.board.tiles.len(), 1);
    }
}
