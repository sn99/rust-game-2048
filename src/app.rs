use crate::components::{
    BoardView, Chrome, ImagePanel, Lightbox, Overlay, RevealBackground,
};
use crate::difficulty::clamp_target;
use crate::game::{Board, Direction};
use crate::input::{
    direction_from_swipe, touch_end_delta, touch_start_coords, use_keyboard, TouchTracker,
};
use crate::progress::reveal_progress_range;
use crate::reddit::{
    filter_live_media, load_random_image, media_seen_in_session, normalize_subreddit,
    post_is_still_public, warm_media_cache, RedditMedia,
};
use crate::storage::{
    load_best, load_goal, load_session_seen_urls, load_subreddit, load_subreddit_pool,
    push_recent_media_urls, save_best, save_goal, save_subreddit,
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
    /// Unblur range: 0% at reveal_from, 100% at reveal_to.
    let reveal_from = RwSignal::new(2u32);
    let reveal_to = RwSignal::new(initial_goal);

    let subreddit = RwSignal::new(load_subreddit());
    let subreddit_pool = RwSignal::new(load_subreddit_pool());
    let image = RwSignal::new(None::<RedditMedia>);
    /// Preloaded next media for instant "New image".
    let preloaded = RwSignal::new(None::<(RedditMedia, &'static str)>);
    let preload_busy = RwSignal::new(false);
    /// Bumped to cancel a pending delayed prefetch.
    let preload_gen = RwSignal::new(0u32);
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
    let progress = Signal::derive(move || {
        reveal_progress_range(max_tile.get(), reveal_from.get(), reveal_to.get())
    });
    let reveal_pct = Signal::derive(move || (progress.get() * 100.0).round() as u32);
    let reveal_hint = Signal::derive(move || {
        format!(
            "Clears at {} · {}% revealed",
            reveal_to.get(),
            reveal_pct.get()
        )
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
        if animating.get_untracked()
            || lightbox_open.get_untracked()
            || loading.get_untracked()
        {
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
        reveal_from.set(2);
        reveal_to.set(t);
    });

    // Filled after on_load_image is defined — see bottom wiring.

    let apply_loaded_media = move |img: RedditMedia, window: &'static str, reset_board: bool| {
        save_subreddit(&img.subreddit);
        subreddit.set(img.subreddit.clone());
        push_recent_media_urls(img.items.iter().map(|i| i.url.clone()));
        warm_media_cache(&img);

        // Clear chrome errors; post title lives only in the reveal panel.
        let _ = window;
        load_status.set(String::new());
        slide_index.set(0);
        image.set(Some(img));
        lightbox_sharp.set(false);
        animating.set(false);

        let g = goal.get_untracked();
        if reset_board {
            board.update(|b| {
                let _ = rng.try_update_value(|r| b.reset_with_goal(r, g));
            });
            reveal_from.set(2);
            reveal_to.set(g);
        } else {
            // Keep board; new post unblurs over the next doubling of the highest tile.
            let cur = board.with_untracked(|b| b.max_tile().max(2));
            reveal_from.set(cur);
            reveal_to.set(cur.saturating_mul(2).max(g));
        }
    };

    /// Prefetch next post immediately in the background (no artificial delay).
    let start_preload = move || {
        if preloaded.get_untracked().is_some() || preload_busy.get_untracked() {
            return;
        }
        let raw = subreddit.get_untracked();
        if raw.trim().is_empty() {
            return;
        }
        let gen = preload_gen.get_untracked().wrapping_add(1);
        preload_gen.set(gen);
        preload_busy.set(true);

        spawn_local(async move {
            if preload_gen.get_untracked() != gen {
                preload_busy.set(false);
                return;
            }
            if preloaded.get_untracked().is_some() || loading.get_untracked() {
                preload_busy.set(false);
                return;
            }
            let mut avoid = load_session_seen_urls();
            if let Some(cur) = image.get_untracked() {
                for it in &cur.items {
                    avoid.push(it.url.clone());
                }
            }
            match load_random_image(&raw, &avoid).await {
                Ok((img, window)) => {
                    if preload_gen.get_untracked() != gen {
                        preload_busy.set(false);
                        return;
                    }
                    warm_media_cache(&img);
                    preloaded.set(Some((img, window)));
                }
                Err(_) => {
                    // Silent background failure (e.g. 429).
                }
            }
            preload_busy.set(false);
        });
    };

    /// Load next media. `reset_board`: true for New image; false for Keep going.
    let load_media = move |reset_board: bool| {
        if loading.get_untracked() {
            return;
        }

        let raw = subreddit.get_untracked();
        let want_sub = normalize_subreddit(&raw).unwrap_or_else(|| raw.trim().to_string());
        loading.set(true);
        // Keep prior success status visible; overlay already says "Loading media…".
        // Do not set long intermediate strings like week→day→month search paths.

        spawn_local(async move {
            let mut avoid = load_session_seen_urls();

            if let Some((img, window)) = preloaded.get_untracked() {
                // Only use a prefetch that matches the sub the user currently wants.
                let pre_sub = normalize_subreddit(&img.subreddit)
                    .unwrap_or_else(|| img.subreddit.clone());
                if !want_sub.is_empty()
                    && pre_sub.eq_ignore_ascii_case(&want_sub)
                    && !media_seen_in_session(&img, &avoid)
                {
                    preloaded.set(None);
                    // Re-check post page still public (prefetch may be minutes old).
                    if post_is_still_public(&img.id).await {
                        if let Some(live) = filter_live_media(img).await {
                            if !media_seen_in_session(&live, &avoid)
                                && post_is_still_public(&live.id).await
                            {
                                apply_loaded_media(live, window, reset_board);
                                loading.set(false);
                                start_preload();
                                return;
                            }
                        }
                    }
                } else {
                    // Stale prefetch for another sub — drop it.
                    preloaded.set(None);
                }
            }

            avoid = load_session_seen_urls();
            match load_random_image(&raw, &avoid).await {
                Ok((img, window)) => {
                    apply_loaded_media(img, window, reset_board);
                    start_preload();
                }
                Err(e) => {
                    load_status.set(e.to_string());
                }
            }
            loading.set(false);
        });
    };

    /// Play / Next: always reset board + load media for the current sub.
    let on_play = Callback::new(move |_: ()| {
        load_media(true);
    });

    let keep_going = Callback::new(move |_: ()| {
        board.update(|b| b.continue_after_win());
        // New post starts blurred; unblurs as you build toward the next double.
        // Stays on the same subreddit (does not run Random).
        if !subreddit.get_untracked().trim().is_empty() || image.get_untracked().is_some() {
            load_media(false);
        }
    });

    /// Reset the board only — keep the same subreddit and the same media.
    let try_again = Callback::new(move |_: ()| {
        animating.set(false);
        let g = goal.get_untracked();
        board.update(|b| {
            let _ = rng.try_update_value(|r| b.reset_with_goal(r, g));
        });
        // Re-blur current image from the start of this run.
        reveal_from.set(2);
        reveal_to.set(g);
        lightbox_sharp.set(false);
        // Do NOT fetch a new post or change the subreddit.
    });

    let on_clear_image = Callback::new(move |_: ()| {
        image.set(None);
        preloaded.set(None);
        preload_gen.update(|g| *g = g.wrapping_add(1));
        preload_busy.set(false);
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
        if loading.get_untracked() {
            return;
        }
        if let Some((x, y)) = touch_start_coords(&ev) {
            touch.set(TouchTracker {
                start_x: x,
                start_y: y,
            });
        }
    };

    let on_touch_end = move |ev: TouchEvent| {
        if loading.get_untracked() {
            return;
        }
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
        <RevealBackground item=current_item progress=progress />
        <Lightbox
            open=lightbox_open
            items=media_items
            image_title=image_title
            slide_index=slide_index
            progress=progress
            sharp=lightbox_sharp
        />
        <main class="app">
            <Chrome
                score=score
                best=best.into()
                win_tile=win_tile
                target=win_tile
                on_select=on_select_goal
                subreddit=subreddit
                pool=subreddit_pool
                status=load_status.into()
                loading=loading.into()
                on_play=on_play
                has_image=has_image
            />

            <div class="play-layout">
                <section class="play-game">
                    <div
                        class=move || {
                            if loading.get() {
                                "board-wrap board-wrap-locked"
                            } else {
                                "board-wrap"
                            }
                        }
                        on:touchstart=on_touch_start
                        on:touchend=on_touch_end
                        on:touchmove=on_touch_move
                    >
                        <BoardView tiles=tiles />
                        <Overlay
                            status=status
                            win_tile=win_tile
                            on_keep_going=keep_going
                            on_try_again=try_again
                        />
                        <Show when=move || loading.get()>
                            <div class="board-loading" role="status" aria-live="polite">
                                <div class="board-loading-card">
                                    <div class="board-loading-spinner"></div>
                                    <p>"Loading media…"</p>
                                    <p class="board-loading-sub">"Play unlocks when ready"</p>
                                </div>
                            </div>
                        </Show>
                    </div>
                </section>

                <aside class="play-media">
                    <ImagePanel
                        items=media_items
                        image_title=image_title
                        image_permalink=image_permalink
                        slide_index=slide_index
                        post_unlocked=post_unlocked
                        progress=progress
                        reveal_pct=reveal_pct
                        reveal_hint=reveal_hint
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
