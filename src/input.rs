//! Keyboard and touch → Direction.

use crate::game::Direction;
use leptos::prelude::*;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use web_sys::{KeyboardEvent, TouchEvent};

const SWIPE_MIN_PX: f64 = 30.0;

pub fn direction_from_key(key: &str) -> Option<Direction> {
    match key {
        "ArrowUp" | "w" | "W" => Some(Direction::Up),
        "ArrowDown" | "s" | "S" => Some(Direction::Down),
        "ArrowLeft" | "a" | "A" => Some(Direction::Left),
        "ArrowRight" | "d" | "D" => Some(Direction::Right),
        _ => None,
    }
}

pub fn direction_from_swipe(dx: f64, dy: f64) -> Option<Direction> {
    if dx.abs() < SWIPE_MIN_PX && dy.abs() < SWIPE_MIN_PX {
        return None;
    }
    if dx.abs() > dy.abs() {
        if dx > 0.0 {
            Some(Direction::Right)
        } else {
            Some(Direction::Left)
        }
    } else if dy > 0.0 {
        Some(Direction::Down)
    } else {
        Some(Direction::Up)
    }
}

/// Attach window keydown listener for the lifetime of the app.
pub fn use_keyboard(on_dir: Callback<Direction>) {
    Effect::new(move |_| {
        let handler = Closure::<dyn FnMut(KeyboardEvent)>::new(move |ev: KeyboardEvent| {
            // Don't steal keys while typing a subreddit.
            if let Some(t) = ev.target() {
                if let Some(el) = t.dyn_ref::<web_sys::HtmlElement>() {
                    let tag = el.tag_name();
                    if tag == "INPUT" || tag == "TEXTAREA" || tag == "SELECT" {
                        return;
                    }
                }
            }
            if let Some(dir) = direction_from_key(&ev.key()) {
                ev.prevent_default();
                on_dir.run(dir);
            }
        });

        if let Some(window) = web_sys::window() {
            let _ = window
                .add_event_listener_with_callback("keydown", handler.as_ref().unchecked_ref());
            // Keep listener for SPA lifetime.
            handler.forget();
        }
    });
}

#[derive(Clone, Copy, Default)]
pub struct TouchTracker {
    pub start_x: f64,
    pub start_y: f64,
}

pub fn touch_start_coords(ev: &TouchEvent) -> Option<(f64, f64)> {
    let touch = ev.touches().get(0)?;
    Some((touch.client_x() as f64, touch.client_y() as f64))
}

pub fn touch_end_delta(ev: &TouchEvent, start: TouchTracker) -> Option<(f64, f64)> {
    let touch = ev.changed_touches().get(0)?;
    let x = touch.client_x() as f64;
    let y = touch.client_y() as f64;
    Some((x - start.start_x, y - start.start_y))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys() {
        assert_eq!(direction_from_key("ArrowLeft"), Some(Direction::Left));
        assert_eq!(direction_from_key("w"), Some(Direction::Up));
        assert_eq!(direction_from_key("x"), None);
    }

    #[test]
    fn swipes() {
        assert_eq!(direction_from_swipe(40.0, 5.0), Some(Direction::Right));
        assert_eq!(direction_from_swipe(-40.0, 5.0), Some(Direction::Left));
        assert_eq!(direction_from_swipe(5.0, 40.0), Some(Direction::Down));
        assert_eq!(direction_from_swipe(5.0, -40.0), Some(Direction::Up));
        assert_eq!(direction_from_swipe(10.0, 10.0), None);
    }
}
