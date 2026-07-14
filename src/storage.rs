//! Persistence via localStorage / sessionStorage.

#[cfg(target_arch = "wasm32")]
use crate::difficulty::{clamp_target, DEFAULT_TARGET};
#[cfg(not(target_arch = "wasm32"))]
use crate::difficulty::DEFAULT_TARGET;
use crate::subreddits::SubredditPool;

#[cfg(target_arch = "wasm32")]
const BEST_KEY: &str = "rust-game-2048-best";
#[cfg(target_arch = "wasm32")]
const SUB_KEY: &str = "rust-game-2048-subreddit";
#[cfg(target_arch = "wasm32")]
const GOAL_KEY: &str = "rust-game-2048-goal";
#[cfg(target_arch = "wasm32")]
const SESSION_SEEN_KEY: &str = "rust-game-2048-session-seen";
#[cfg(target_arch = "wasm32")]
const POOL_KEY: &str = "rust-game-2048-sub-pool";

pub fn load_best() -> u32 {
    #[cfg(target_arch = "wasm32")]
    {
        web_sys::window()
            .and_then(|w| w.local_storage().ok().flatten())
            .and_then(|s| s.get_item(BEST_KEY).ok().flatten())
            .and_then(|v| v.parse().ok())
            .unwrap_or(0)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        0
    }
}

pub fn save_best(score: u32) {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(storage) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
            let _ = storage.set_item(BEST_KEY, &score.to_string());
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = score;
    }
}

pub fn load_subreddit() -> String {
    #[cfg(target_arch = "wasm32")]
    {
        web_sys::window()
            .and_then(|w| w.local_storage().ok().flatten())
            .and_then(|s| s.get_item(SUB_KEY).ok().flatten())
            .unwrap_or_default()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        String::new()
    }
}

pub fn save_subreddit(sub: &str) {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(storage) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
            let _ = storage.set_item(SUB_KEY, sub);
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = sub;
    }
}

pub fn load_subreddit_pool() -> SubredditPool {
    #[cfg(target_arch = "wasm32")]
    {
        web_sys::window()
            .and_then(|w| w.local_storage().ok().flatten())
            .and_then(|s| s.get_item(POOL_KEY).ok().flatten())
            .map(|v| SubredditPool::from_str(&v))
            .unwrap_or(SubredditPool::Sfw)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        SubredditPool::Sfw
    }
}

pub fn save_subreddit_pool(pool: SubredditPool) {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(storage) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
            let _ = storage.set_item(POOL_KEY, pool.as_str());
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = pool;
    }
}

pub fn load_goal() -> u32 {
    #[cfg(target_arch = "wasm32")]
    {
        let raw = web_sys::window()
            .and_then(|w| w.local_storage().ok().flatten())
            .and_then(|s| s.get_item(GOAL_KEY).ok().flatten())
            .and_then(|v| v.parse().ok())
            .unwrap_or(DEFAULT_TARGET);
        clamp_target(raw)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        DEFAULT_TARGET
    }
}

pub fn save_goal(goal: u32) {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(storage) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
            let _ = storage.set_item(GOAL_KEY, &goal.to_string());
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = goal;
    }
}

/// URLs already shown this browser session (sessionStorage — clears on tab close).
pub fn load_session_seen_urls() -> Vec<String> {
    #[cfg(target_arch = "wasm32")]
    {
        web_sys::window()
            .and_then(|w| w.session_storage().ok().flatten())
            .and_then(|s| s.get_item(SESSION_SEEN_KEY).ok().flatten())
            .and_then(|raw| serde_json::from_str(&raw).ok())
            .unwrap_or_default()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        Vec::new()
    }
}

/// Record every media URL from a post as used this session.
pub fn mark_session_seen_urls(urls: &[String]) {
    #[cfg(target_arch = "wasm32")]
    {
        if urls.is_empty() {
            return;
        }
        let mut list = load_session_seen_urls();
        for u in urls {
            list.retain(|x| x != u);
            list.insert(0, u.clone());
        }
        // Cap so storage stays reasonable within a long session
        list.truncate(500);
        if let Some(storage) = web_sys::window().and_then(|w| w.session_storage().ok().flatten()) {
            if let Ok(raw) = serde_json::to_string(&list) {
                let _ = storage.set_item(SESSION_SEEN_KEY, &raw);
            }
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = urls;
    }
}

// Back-compat names used by app during transition
pub fn load_recent_image_urls() -> Vec<String> {
    load_session_seen_urls()
}

pub fn push_recent_image_url(url: &str) {
    mark_session_seen_urls(&[url.to_string()]);
}

pub fn push_recent_media_urls(urls: impl IntoIterator<Item = String>) {
    let v: Vec<String> = urls.into_iter().collect();
    mark_session_seen_urls(&v);
}
