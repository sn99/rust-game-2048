use crate::components::{BoardView, Header, Overlay};
use crate::game::{Board, Direction};
use crate::input::{
    direction_from_swipe, touch_end_delta, touch_start_coords, use_keyboard, TouchTracker,
};
use crate::storage::{load_best, save_best};
use leptos::prelude::*;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use web_sys::TouchEvent;

/// Match classic 2048 slide duration before spawning the next tile.
const SLIDE_MS: i32 = 100;

#[component]
pub fn App() -> impl IntoView {
    let rng = StoredValue::new(fastrand::Rng::new());
    let board = RwSignal::new(rng.try_update_value(|r| Board::new(r)).expect("rng"));
    let best = RwSignal::new(load_best());
    let touch = RwSignal::new(TouchTracker::default());
    let animating = RwSignal::new(false);

    let score = Signal::derive(move || board.get().score());
    let status = Signal::derive(move || board.get().status());
    let tiles = Signal::derive(move || board.get().tiles().to_vec());

    let finish_move = move || {
        board.update(|b| {
            let _ = rng.try_update_value(|r| b.spawn_after_move(r));
            let s = b.score();
            best.update(|best_score| {
                if s > *best_score {
                    *best_score = s;
                    save_best(s);
                }
            });
        });
        animating.set(false);
    };

    let apply_move = Callback::new(move |dir: Direction| {
        if animating.get_untracked() {
            return;
        }

        let moved = board.try_update(|b| b.try_move(dir)).unwrap_or(false);
        if !moved {
            return;
        }

        // Score already includes merges; update best early so UI feels snappy.
        let s = board.with_untracked(|b| b.score());
        best.update(|best_score| {
            if s > *best_score {
                *best_score = s;
                save_best(s);
            }
        });

        animating.set(true);
        // Spawn after slide so appear anim doesn't start mid-slide.
        set_timeout(SLIDE_MS, finish_move);
    });

    let new_game = Callback::new(move |_: ()| {
        animating.set(false);
        board.update(|b| {
            let _ = rng.try_update_value(|r| b.reset(r));
        });
    });

    let keep_going = Callback::new(move |_: ()| {
        board.update(|b| b.continue_after_win());
    });

    use_keyboard(apply_move);

    let on_touch_start = move |ev: TouchEvent| {
        if let Some((x, y)) = touch_start_coords(&ev) {
            touch.set(TouchTracker {
                start_x: x,
                start_y: y,
            });
        }
    };

    let on_touch_end = move |ev: TouchEvent| {
        let start = touch.get();
        if let Some((dx, dy)) = touch_end_delta(&ev, start) {
            if let Some(dir) = direction_from_swipe(dx, dy) {
                apply_move.run(dir);
            }
        }
    };

    let on_touch_move = move |ev: TouchEvent| {
        ev.prevent_default();
    };

    view! {
        <main class="app">
            <Header score=score best=best.into() on_new_game=new_game />
            <div
                class="board-wrap"
                on:touchstart=on_touch_start
                on:touchend=on_touch_end
                on:touchmove=on_touch_move
            >
                <BoardView tiles=tiles />
                <Overlay status=status on_keep_going=keep_going on_try_again=new_game />
            </div>
            <p class="how-to">
                <strong>"How to play: "</strong>
                "Use your "
                <strong>"arrow keys"</strong>
                " or "
                <strong>"swipe"</strong>
                " to move the tiles. When two tiles with the same number touch, they "
                <strong>"merge into one!"</strong>
            </p>
            <p class="credit">
                "Built with Rust + Leptos · Inspired by "
                <a href="https://play2048.co/" target="_blank" rel="noopener noreferrer">
                    "2048 by Gabriele Cirulli"
                </a>
            </p>
        </main>
    }
}

fn set_timeout(ms: i32, f: impl FnOnce() + 'static) {
    let Some(window) = web_sys::window() else {
        f();
        return;
    };
    let cb = Closure::once(Box::new(f) as Box<dyn FnOnce()>);
    let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
        cb.as_ref().unchecked_ref(),
        ms,
    );
    // Keep closure alive until the timer fires.
    cb.forget();
}
