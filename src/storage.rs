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
