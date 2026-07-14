use crate::components::{
    BoardView, DifficultyBar, Header, Overlay, RevealBackground, SubredditBar,
};
use crate::difficulty::clamp_target;
use crate::game::{Board, Direction};
use crate::input::{
    direction_from_swipe, touch_end_delta, touch_start_coords, use_keyboard, TouchTracker,
};
use crate::progress::reveal_progress;
use crate::reddit::{load_random_image, RedditImage};
use crate::storage::{
    load_best, load_goal, load_subreddit, save_best, save_goal, save_subreddit,
};
use leptos::prelude::*;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use web_sys::TouchEvent;

/// Match classic 2048 slide duration before spawning the next tile.
const SLIDE_MS: i32 = 100;

#[component]
pub fn App() -> impl IntoView {
    let rng = StoredValue::new(fastrand::Rng::new());
    let initial_goal = load_goal();
    let board = RwSignal::new(
        rng.try_update_value(|r| Board::new(r, initial_goal))
            .expect("rng"),
    );
    let best = RwSignal::new(load_best());
    let touch = RwSignal::new(TouchTracker::default());
    let animating = RwSignal::new(false);
    let goal = RwSignal::new(initial_goal);

    let subreddit = RwSignal::new(load_subreddit());
    let image = RwSignal::new(None::<RedditImage>);
    let load_status = RwSignal::new(String::new());
    let loading = RwSignal::new(false);

    let score = Signal::derive(move || board.get().score());
    let status = Signal::derive(move || board.get().status());
    let tiles = Signal::derive(move || board.get().tiles().to_vec());
    let max_tile = Signal::derive(move || board.get().max_tile());
    let win_tile = Signal::derive(move || goal.get());
    let image_url = Signal::derive(move || image.get().map(|i| i.url));
    let has_image = Signal::derive(move || image.get().is_some());
    let reveal_pct = Signal::derive(move || {
        (reveal_progress(max_tile.get(), goal.get()) * 100.0).round() as u32
    });

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

        let s = board.with_untracked(|b| b.score());
        best.update(|best_score| {
            if s > *best_score {
                *best_score = s;
                save_best(s);
            }
        });

        animating.set(true);
        set_timeout(SLIDE_MS, finish_move);
    });

    let new_game = Callback::new(move |_: ()| {
        animating.set(false);
        let g = goal.get_untracked();
        board.update(|b| {
            let _ = rng.try_update_value(|r| b.reset_with_goal(r, g));
        });
    });

    let on_select_goal = Callback::new(move |t: u32| {
        let t = clamp_target(t);
        if t == goal.get_untracked() {
            return;
        }
        goal.set(t);
        save_goal(t);
        animating.set(false);
        board.update(|b| {
            let _ = rng.try_update_value(|r| b.reset_with_goal(r, t));
        });
    });

    let keep_going = Callback::new(move |_: ()| {
        board.update(|b| b.continue_after_win());
    });

    let on_load_image = Callback::new(move |_: ()| {
        if loading.get_untracked() {
            return;
        }
        let raw = subreddit.get_untracked();
        loading.set(true);
        load_status.set("Fetching top posts…".into());
        spawn_local(async move {
            match load_random_image(&raw).await {
                Ok(img) => {
                    save_subreddit(&img.subreddit);
                    subreddit.set(img.subreddit.clone());
                    let msg = if img.title.is_empty() {
                        format!("Loaded from r/{}", img.subreddit)
                    } else {
                        let title = if img.title.len() > 80 {
                            format!("{}…", &img.title[..77])
                        } else {
                            img.title.clone()
                        };
                        format!("r/{} — {}", img.subreddit, title)
                    };
                    image.set(Some(img));
                    load_status.set(msg);
                }
                Err(e) => {
                    load_status.set(e.to_string());
                }
            }
            loading.set(false);
        });
    });

    let on_clear_image = Callback::new(move |_: ()| {
        image.set(None);
        load_status.set(String::new());
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
        <RevealBackground image_url=image_url max_tile=max_tile win_tile=win_tile />
        <main class="app">
            <Header
                score=score
                best=best.into()
                win_tile=win_tile
                on_new_game=new_game
            />
            <DifficultyBar target=win_tile on_select=on_select_goal />
            <SubredditBar
                subreddit=subreddit
                status=load_status.into()
                loading=loading.into()
                on_load=on_load_image
                on_clear=on_clear_image
                has_image=has_image
                reveal_pct=reveal_pct
            />
            <div
                class="board-wrap"
                on:touchstart=on_touch_start
                on:touchend=on_touch_end
                on:touchmove=on_touch_move
            >
                <BoardView tiles=tiles />
                <Overlay
                    status=status
                    win_tile=win_tile
                    on_keep_going=keep_going
                    on_try_again=new_game
                />
            </div>
            <p class="how-to">
                <strong>"How to play: "</strong>
                "Use your "
                <strong>"arrow keys"</strong>
                " or "
                <strong>"swipe"</strong>
                " to move the tiles. Pick a "
                <strong>"goal"</strong>
                " below the title (64 is easiest, 4096 is hardest). "
                "Optional: load a subreddit image — it unblurs as you approach your goal."
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
    cb.forget();
}
