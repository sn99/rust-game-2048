use crate::components::{
    BoardView, DifficultyBar, Header, ImagePanel, Lightbox, Overlay, RevealBackground,
    SubredditBar,
};
use crate::difficulty::clamp_target;
use crate::game::{Board, Direction};
use crate::input::{
    direction_from_swipe, touch_end_delta, touch_start_coords, use_keyboard, TouchTracker,
};
use crate::progress::reveal_progress;
use crate::reddit::{load_random_image, warm_media_cache, RedditMedia};
use crate::storage::{
    load_best, load_goal, load_session_seen_urls, load_subreddit, push_recent_media_urls, save_best,
    save_goal, save_subreddit,
};
use leptos::prelude::*;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use web_sys::TouchEvent;

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
    let image = RwSignal::new(None::<RedditMedia>);
    /// Preloaded next media for instant "New image".
    let preloaded = RwSignal::new(None::<(RedditMedia, &'static str)>);
    let preload_busy = RwSignal::new(false);
    let slide_index = RwSignal::new(0usize);
    let load_status = RwSignal::new(String::new());
    let loading = RwSignal::new(false);
    let lightbox_open = RwSignal::new(false);
    let lightbox_sharp = RwSignal::new(false);

    let score = Signal::derive(move || board.get().score());
    let status = Signal::derive(move || board.get().status());
    let tiles = Signal::derive(move || board.get().tiles().to_vec());
    let max_tile = Signal::derive(move || board.get().max_tile());
    let win_tile = Signal::derive(move || goal.get());
    let media_items = Signal::derive(move || {
        image
            .get()
            .map(|i| i.items.clone())
            .unwrap_or_default()
    });
    let image_title =
        Signal::derive(move || image.get().map(|i| i.title.clone()).unwrap_or_default());
    let image_permalink = Signal::derive(move || image.get().map(|i| i.permalink.clone()));
    let has_image = Signal::derive(move || image.get().is_some());
    let post_unlocked =
        Signal::derive(move || max_tile.get() >= win_tile.get() && win_tile.get() > 0);
    let reveal_pct = Signal::derive(move || {
        (reveal_progress(max_tile.get(), goal.get()) * 100.0).round() as u32
    });
    let current_item = Signal::derive(move || {
        let list = media_items.get();
        if list.is_empty() {
            return None;
        }
        let i = slide_index.get().min(list.len() - 1);
        Some(list[i].clone())
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
        if animating.get_untracked() || lightbox_open.get_untracked() {
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

    let apply_loaded_media = move |img: RedditMedia, window: &'static str| {
        save_subreddit(&img.subreddit);
        subreddit.set(img.subreddit.clone());
        push_recent_media_urls(img.items.iter().map(|i| i.url.clone()));
        warm_media_cache(&img);

        let kind_bit = if img.has_video() && img.is_multi() {
            format!(" · {} clips/photos", img.items.len())
        } else if img.has_video() {
            " · video".into()
        } else if img.is_multi() {
            format!(" · {} photos", img.items.len())
        } else {
            String::new()
        };
        let title_bit = if img.title.is_empty() {
            String::new()
        } else {
            let t = if img.title.len() > 50 {
                format!("{}…", &img.title[..47])
            } else {
                img.title.clone()
            };
            format!(" — {t}")
        };
        let ready = if preloaded.get_untracked().is_some() {
            ""
        } else {
            ""
        };
        let _ = ready;
        load_status.set(format!(
            "r/{} · {}{}{}",
            img.subreddit, window, kind_bit, title_bit
        ));
        slide_index.set(0);
        image.set(Some(img));
        lightbox_sharp.set(false);
        animating.set(false);
        let g = goal.get_untracked();
        board.update(|b| {
            let _ = rng.try_update_value(|r| b.reset_with_goal(r, g));
        });
    };

    let start_preload = move || {
        if preload_busy.get_untracked() {
            return;
        }
        let raw = subreddit.get_untracked();
        if raw.trim().is_empty() {
            return;
        }
        preload_busy.set(true);
        spawn_local(async move {
            let mut avoid = load_session_seen_urls();
            if let Some(cur) = image.get_untracked() {
                avoid.push(cur.primary_url().to_string());
            }
            if let Some((p, _)) = preloaded.get_untracked() {
                avoid.push(p.primary_url().to_string());
            }
            match load_random_image(&raw, &avoid).await {
                Ok((img, window)) => {
                    warm_media_cache(&img);
                    preloaded.set(Some((img, window)));
                    // Hint that next is ready (don't overwrite error states noisily)
                    let status = load_status.get_untracked();
                    if !status.is_empty() && !status.contains("next ready") {
                        load_status.set(format!("{status} · next ready"));
                    }
                }
                Err(_) => {
                    preloaded.set(None);
                }
            }
            preload_busy.set(false);
        });
    };

    let on_load_image = Callback::new(move |_: ()| {
        if loading.get_untracked() {
            return;
        }

        // Instant path: use preloaded +1
        if let Some((img, window)) = preloaded.get_untracked() {
            preloaded.set(None);
            apply_loaded_media(img, window);
            start_preload();
            return;
        }

        let raw = subreddit.get_untracked();
        loading.set(true);
        load_status.set("Searching top week → day → month → year → all-time…".into());
        spawn_local(async move {
            let avoid = load_session_seen_urls();
            match load_random_image(&raw, &avoid).await {
                Ok((img, window)) => {
                    apply_loaded_media(img, window);
                    start_preload();
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
        preloaded.set(None);
        slide_index.set(0);
        lightbox_open.set(false);
        load_status.set(String::new());
    });

    let on_open_full = Callback::new(move |_: ()| {
        lightbox_sharp.set(false);
        lightbox_open.set(true);
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
        <RevealBackground item=current_item max_tile=max_tile win_tile=win_tile />
        <Lightbox
            open=lightbox_open
            items=media_items
            image_title=image_title
            slide_index=slide_index
            max_tile=max_tile
            win_tile=win_tile
            sharp=lightbox_sharp
        />
        <main class="app">
            <Header
                score=score
                best=best.into()
                win_tile=win_tile
                on_new_game=new_game
            />
            <DifficultyBar target=win_tile on_select=on_select_goal />

            <div class="play-layout">
                <section class="play-game">
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
                </section>

                <aside class="play-media">
                    <SubredditBar
                        subreddit=subreddit
                        status=load_status.into()
                        loading=loading.into()
                        on_load=on_load_image
                        has_image=has_image
                    />
                    <ImagePanel
                        items=media_items
                        image_title=image_title
                        image_permalink=image_permalink
                        slide_index=slide_index
                        post_unlocked=post_unlocked
                        max_tile=max_tile
                        win_tile=win_tile
                        reveal_pct=reveal_pct
                        on_open_full=on_open_full
                        on_clear=on_clear_image
                    />
                </aside>
            </div>

            <p class="credit desktop-hide-credit">
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
