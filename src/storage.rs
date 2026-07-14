//! Persistence via localStorage.

#[cfg(target_arch = "wasm32")]
use crate::difficulty::{clamp_target, DEFAULT_TARGET};
#[cfg(not(target_arch = "wasm32"))]
use crate::difficulty::DEFAULT_TARGET;

#[cfg(target_arch = "wasm32")]
const BEST_KEY: &str = "rust-game-2048-best";
#[cfg(target_arch = "wasm32")]
const SUB_KEY: &str = "rust-game-2048-subreddit";
#[cfg(target_arch = "wasm32")]
const GOAL_KEY: &str = "rust-game-2048-goal";

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

#[cfg(target_arch = "wasm32")]
const RECENT_IMAGES_KEY: &str = "rust-game-2048-recent-images";
#[cfg(target_arch = "wasm32")]
const RECENT_IMAGES_MAX: usize = 40;

/// Recently shown image URLs (avoid repeats).
pub fn load_recent_image_urls() -> Vec<String> {
    #[cfg(target_arch = "wasm32")]
    {
        web_sys::window()
            .and_then(|w| w.local_storage().ok().flatten())
            .and_then(|s| s.get_item(RECENT_IMAGES_KEY).ok().flatten())
            .and_then(|raw| serde_json::from_str(&raw).ok())
            .unwrap_or_default()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        Vec::new()
    }
}

pub fn push_recent_image_url(url: &str) {
    #[cfg(target_arch = "wasm32")]
    {
        let mut list = load_recent_image_urls();
        list.retain(|u| u != url);
        list.insert(0, url.to_string());
        list.truncate(RECENT_IMAGES_MAX);
        if let Some(storage) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
            if let Ok(raw) = serde_json::to_string(&list) {
                let _ = storage.set_item(RECENT_IMAGES_KEY, &raw);
            }
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = url;
    }
}
