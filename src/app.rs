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
    abort_active_fetches, load_media_batch, load_random_image, media_seen_in_session,
    normalize_subreddit, warm_media_cache, RedditMedia,
};
use crate::subreddits::warm_community_caches;
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
    /// Queue of ready next posts (target depth 3) for instant Next / Next game.
    let preload_queue = RwSignal::new(Vec::<(RedditMedia, &'static str)>::new());
    let preload_busy = RwSignal::new(false);
    /// Bumped to cancel in-flight prefetch workers.
    let preload_gen = RwSignal::new(0u32);
    /// Bumped to cancel in-flight foreground fetches (user switched sub mid-load).
    let load_gen = RwSignal::new(0u32);
    const PRELOAD_TARGET: usize = 3;
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

    /// Abort in-flight foreground + background fetches (user switched away).
    let cancel_in_flight = move || {
        load_gen.update(|g| *g = g.wrapping_add(1));
        preload_gen.update(|g| *g = g.wrapping_add(1));
        preload_busy.set(false);
        loading.set(false);
        // Stop browser HTTP work (AbortController) so Network tab clears abandoned calls.
        abort_active_fetches();
    };

    /// One batched API wave fills missing queue slots (not 1 request per post).
    let fill_preload_queue = move || {
        // (3) Never prefetch when the queue already has enough.
        if loading.get_untracked() || preload_busy.get_untracked() {
            return;
        }
        let raw = subreddit.get_untracked();
        if raw.trim().is_empty() {
            return;
        }
        let have = preload_queue.with_untracked(|q| q.len());
        if have >= PRELOAD_TARGET {
            return;
        }
        let need = PRELOAD_TARGET - have;
        if need == 0 {
            return;
        }
        let gen = preload_gen.get_untracked();
        preload_busy.set(true);

        spawn_local(async move {
            if preload_gen.get_untracked() != gen || loading.get_untracked() {
                preload_busy.set(false);
                return;
            }

            let mut avoid = load_session_seen_urls();
            if let Some(cur) = image.get_untracked() {
                for it in &cur.items {
                    avoid.push(it.url.clone());
                }
            }
            preload_queue.with_untracked(|q| {
                for (p, _) in q {
                    for it in &p.items {
                        avoid.push(it.url.clone());
                    }
                }
            });

            // Single batch path: ~1 listing + 1 id-reverify for up to `need` posts.
            if let Ok(batch) = load_media_batch(&raw, &avoid, need).await {
                if preload_gen.get_untracked() != gen {
                    preload_busy.set(false);
                    return;
                }
                // Still the same sub the user wants?
                let still = subreddit.get_untracked();
                if normalize_subreddit(&still).as_deref() != normalize_subreddit(&raw).as_deref()
                    && !still.eq_ignore_ascii_case(&raw)
                {
                    preload_busy.set(false);
                    return;
                }
                for (live, window) in batch {
                    let dup = preload_queue.with_untracked(|q| {
                        q.iter()
                            .any(|(m, _)| m.id == live.id || m.primary_url() == live.primary_url())
                    });
                    if dup || media_seen_in_session(&live, &load_session_seen_urls()) {
                        continue;
                    }
                    warm_media_cache(&live);
                    preload_queue.update(|q| {
                        if q.len() < PRELOAD_TARGET {
                            q.push((live, window));
                        }
                    });
                }
            }
            preload_busy.set(false);
        });
    };

    /// Load next media. Supersedes any in-flight fetch (user can switch mid-load).
    let load_media = move || {
        let raw = subreddit.get_untracked();
        let want_sub = normalize_subreddit(&raw).unwrap_or_else(|| raw.trim().to_string());
        let avoid = load_session_seen_urls();

        // Instant path: pop from ready queue (same sub).
        let mut queue = preload_queue.get_untracked();
        let mut taken = None;
        queue.retain(|(img, window)| {
            if taken.is_some() {
                return true;
            }
            let pre_sub =
                normalize_subreddit(&img.subreddit).unwrap_or_else(|| img.subreddit.clone());
            if !want_sub.is_empty()
                && pre_sub.eq_ignore_ascii_case(&want_sub)
                && !media_seen_in_session(img, &avoid)
            {
                taken = Some((img.clone(), *window));
                false
            } else {
                true
            }
        });
        // Drop wrong-sub leftovers when user switched communities.
        queue.retain(|(img, _)| {
            let pre_sub =
                normalize_subreddit(&img.subreddit).unwrap_or_else(|| img.subreddit.clone());
            want_sub.is_empty() || pre_sub.eq_ignore_ascii_case(&want_sub)
        });
        preload_queue.set(queue);

        if let Some((img, window)) = taken {
            // Cancel anything still downloading an old sub.
            cancel_in_flight();
            apply_loaded_media(img, window, true);
            fill_preload_queue();
            return;
        }

        // Supersede previous foreground + background work for the sub we're skipping.
        let gen = load_gen.get_untracked().wrapping_add(1);
        load_gen.set(gen);
        preload_gen.update(|g| *g = g.wrapping_add(1));
        preload_busy.set(false);
        preload_queue.set(Vec::new());

        loading.set(true);
        load_status.set(String::new());
        let raw_owned = raw.clone();
        spawn_local(async move {
            let avoid = load_session_seen_urls();
            // Fetch current + extras for the queue in one efficient batch when possible.
            let result = load_media_batch(&raw_owned, &avoid, 1 + PRELOAD_TARGET).await;

            // Abandoned: user typed another sub / hit SFW / Next on something else.
            if load_gen.get_untracked() != gen {
                return;
            }

            match result {
                Ok(mut batch) if !batch.is_empty() => {
                    let (first, window) = batch.remove(0);
                    apply_loaded_media(first, window, true);
                    for (live, w) in batch {
                        warm_media_cache(&live);
                        preload_queue.update(|q| {
                            if q.len() < PRELOAD_TARGET {
                                q.push((live, w));
                            }
                        });
                    }
                }
                Ok(_) => {
                    match load_random_image(&raw_owned, &avoid).await {
                        Ok((img, window)) => {
                            if load_gen.get_untracked() != gen {
                                return;
                            }
                            apply_loaded_media(img, window, true);
                        }
                        Err(e) => {
                            if load_gen.get_untracked() == gen {
                                load_status.set(e.to_string());
                            }
                        }
                    }
                }
                Err(e) => {
                    if load_gen.get_untracked() == gen {
                        load_status.set(e.to_string());
                    }
                }
            }

            if load_gen.get_untracked() != gen {
                return;
            }
            loading.set(false);
            // Only prefetch if the queue still has room (batch often already filled it).
            if image.get_untracked().is_some()
                && preload_queue.with_untracked(|q| q.len()) < PRELOAD_TARGET
            {
                fill_preload_queue();
            }
        });
    };

    /// Play / Next / Next game: new board + next post for the current sub.
    let on_play = Callback::new(move |_: ()| {
        load_media();
    });

    /// User is editing the sub field or skipping — abandon the fetch in progress.
    let on_sub_edit = Callback::new(move |_: ()| {
        cancel_in_flight();
        preload_queue.set(Vec::new());
        load_status.set(String::new());
    });

    // Warm SFW + NSFW community decks once so random picks are instant.
    Effect::new(move |_| {
        spawn_local(async {
            let _ = warm_community_caches().await;
        });
    });

    let on_clear_image = Callback::new(move |_: ()| {
        image.set(None);
        preload_queue.set(Vec::new());
        cancel_in_flight();
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
                on_sub_edit=on_sub_edit
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
                            on_next_game=on_play
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
