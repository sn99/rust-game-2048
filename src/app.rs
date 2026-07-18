use crate::components::{
    BoardView, Chrome, Gallery, ImagePanel, Lightbox, Overlay, RevealBackground,
};
use crate::difficulty::clamp_target;
use crate::game::{Board, Direction};
use crate::input::{
    direction_from_swipe, touch_end_delta, touch_start_coords, use_keyboard, TouchTracker,
};
use crate::progress::reveal_progress_range;
use crate::reddit::{
    abort_active_fetches, load_media_batch, load_random_image, media_seen_in_session,
    normalize_subreddit, warm_media_cache, LoadTier, RedditMedia,
};
use crate::storage::{
    clear_game_session, load_best, load_gallery, load_game_session, load_goal,
    load_session_seen_urls, load_subreddit, load_subreddit_pool, push_gallery_entry,
    push_recent_media_urls, remember_good_sub, save_best, save_game_session, save_goal,
    save_subreddit, GalleryEntry, GameSession,
};
use crate::subreddits::warm_community_caches;
use leptos::prelude::*;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use web_sys::TouchEvent;

const SLIDE_MS: i32 = 100;
/// Prefetch depth — lower = less concurrent network work.
const PRELOAD_TARGET: usize = 2;

#[component]
pub fn App() -> impl IntoView {
    let rng = StoredValue::new(fastrand::Rng::new());
    let initial_goal = load_goal();

    // --- Session restore (refresh in the same tab) ---
    let restored = load_game_session().and_then(|s| {
        let board = Board::from_snapshot(s.board.clone())?;
        Some((
            board,
            s.reveal_from,
            s.reveal_to,
            s.media,
            s.slide_index,
        ))
    });
    let (initial_board, initial_reveal_from, initial_reveal_to, initial_media, initial_slide) =
        if let Some(parts) = restored {
            parts
        } else {
            let b = rng
                .try_update_value(|r| Board::new(r, initial_goal))
                .expect("rng");
            (b, 2u32, initial_goal, None, 0usize)
        };

    let board = RwSignal::new(initial_board);
    let best = RwSignal::new(load_best());
    let touch = RwSignal::new(TouchTracker::default());
    let animating = RwSignal::new(false);
    let goal = RwSignal::new(initial_goal);
    let reveal_from = RwSignal::new(initial_reveal_from);
    let reveal_to = RwSignal::new(initial_reveal_to);

    let subreddit = RwSignal::new(load_subreddit());
    let subreddit_pool = RwSignal::new(load_subreddit_pool());
    let image = RwSignal::new(initial_media);
    let preload_queue = RwSignal::new(Vec::<(RedditMedia, &'static str)>::new());
    let preload_busy = RwSignal::new(false);
    let preload_gen = RwSignal::new(0u32);
    let load_gen = RwSignal::new(0u32);
    // Quality-refine generation — cancelled when user moves on (Play/Next/sub change).
    let quality_gen = RwSignal::new(0u32);
    let slide_index = RwSignal::new(initial_slide);
    let load_status = RwSignal::new(String::new());
    let loading = RwSignal::new(false);
    let lightbox_open = RwSignal::new(false);
    let lightbox_sharp = RwSignal::new(false);
    let gallery = RwSignal::new(load_gallery());
    // Track which media ids we already added to the gallery this unlock.
    let gallery_armed = RwSignal::new(None::<String>);

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
    let gallery_sig = Signal::derive(move || gallery.get());

    let persist_session = move || {
        let Some(snap) = board.with_untracked(|b| Some(b.to_snapshot())) else {
            return;
        };
        let session = GameSession {
            board: snap,
            reveal_from: reveal_from.get_untracked(),
            reveal_to: reveal_to.get_untracked(),
            media: image.get_untracked(),
            slide_index: slide_index.get_untracked(),
            goal: goal.get_untracked(),
        };
        save_game_session(&session);
    };

    let maybe_record_gallery = move || {
        if !post_unlocked.get_untracked() {
            return;
        }
        let Some(media) = image.get_untracked() else {
            return;
        };
        let id = if media.id.is_empty() {
            media.primary_url().to_string()
        } else {
            media.id.clone()
        };
        if gallery_armed.get_untracked().as_deref() == Some(id.as_str()) {
            return;
        }
        gallery_armed.set(Some(id));
        let entry = GalleryEntry {
            media,
            goal: goal.get_untracked(),
            max_tile: max_tile.get_untracked(),
        };
        push_gallery_entry(entry.clone());
        gallery.set(load_gallery());
    };

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
        maybe_record_gallery();
        persist_session();
    };

    let apply_move = Callback::new(move |dir: Direction| {
        let first_load_lock = loading.get_untracked() && image.get_untracked().is_none();
        if animating.get_untracked() || lightbox_open.get_untracked() || first_load_lock {
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
        gallery_armed.set(None);
        persist_session();
    });

    let apply_loaded_media = move |img: RedditMedia, window: &'static str, reset_board: bool| {
        save_subreddit(&img.subreddit);
        remember_good_sub(&img.subreddit);
        subreddit.set(img.subreddit.clone());
        push_recent_media_urls(img.items.iter().map(|i| i.url.clone()));
        warm_media_cache(&img);

        let _ = window;
        load_status.set(String::new());
        slide_index.set(0);
        image.set(Some(img));
        lightbox_sharp.set(false);
        animating.set(false);
        gallery_armed.set(None);

        let g = goal.get_untracked();
        if reset_board {
            board.update(|b| {
                let _ = rng.try_update_value(|r| b.reset_with_goal(r, g));
            });
            reveal_from.set(2);
            reveal_to.set(g);
        } else {
            let cur = board.with_untracked(|b| b.max_tile().max(2));
            reveal_from.set(cur);
            reveal_to.set(cur.saturating_mul(2).max(g));
        }
        persist_session();
    };

    let cancel_in_flight = move || {
        load_gen.update(|g| *g = g.wrapping_add(1));
        preload_gen.update(|g| *g = g.wrapping_add(1));
        quality_gen.update(|g| *g = g.wrapping_add(1));
        preload_busy.set(false);
        loading.set(false);
        abort_active_fetches();
    };

    fn same_sub(a: &str, b: &str) -> bool {
        match (normalize_subreddit(a), normalize_subreddit(b)) {
            (Some(x), Some(y)) => x.eq_ignore_ascii_case(&y),
            _ => a.trim().eq_ignore_ascii_case(b.trim()),
        }
    }

    let fill_preload_queue = move || {
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

            if let Ok(batch) = load_media_batch(&raw, &avoid, need, LoadTier::Fast).await {
                if preload_gen.get_untracked() != gen {
                    preload_busy.set(false);
                    return;
                }
                let still = subreddit.get_untracked();
                if !same_sub(&still, &raw) {
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

    // Quality refine — cancelled when quality_gen or preload_gen bumps (user moved on).
    let refine_queue_quality = move || {
        let raw = subreddit.get_untracked();
        if raw.trim().is_empty() {
            return;
        }
        let pgen = preload_gen.get_untracked();
        let qgen = quality_gen.get_untracked().wrapping_add(1);
        quality_gen.set(qgen);
        spawn_local(async move {
            let mut avoid = load_session_seen_urls();
            if let Some(cur) = image.get_untracked() {
                for it in &cur.items {
                    avoid.push(it.url.clone());
                }
            }
            let Ok(batch) =
                load_media_batch(&raw, &avoid, PRELOAD_TARGET, LoadTier::Quality).await
            else {
                return;
            };
            // Drop result if user switched sub, started a new load, or superseded refine.
            if quality_gen.get_untracked() != qgen || preload_gen.get_untracked() != pgen {
                return;
            }
            let still = subreddit.get_untracked();
            if !same_sub(&still, &raw) {
                return;
            }
            if batch.is_empty() {
                return;
            }
            for (m, _) in &batch {
                warm_media_cache(m);
            }
            preload_queue.set(batch);
        });
    };

    let load_media = move || {
        let mut raw = subreddit.get_untracked();
        if raw.trim().is_empty() {
            raw = "pics".to_string();
            subreddit.set(raw.clone());
            save_subreddit(&raw);
        }
        let want_sub = normalize_subreddit(&raw).unwrap_or_else(|| raw.trim().to_string());
        let avoid = load_session_seen_urls();

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
        queue.retain(|(img, _)| {
            let pre_sub =
                normalize_subreddit(&img.subreddit).unwrap_or_else(|| img.subreddit.clone());
            want_sub.is_empty() || pre_sub.eq_ignore_ascii_case(&want_sub)
        });
        preload_queue.set(queue);

        if let Some((img, window)) = taken {
            apply_loaded_media(img, window, true);
            load_gen.update(|g| *g = g.wrapping_add(1));
            quality_gen.update(|g| *g = g.wrapping_add(1));
            loading.set(false);
            fill_preload_queue();
            refine_queue_quality();
            return;
        }

        let gen = load_gen.get_untracked().wrapping_add(1);
        load_gen.set(gen);
        preload_gen.update(|g| *g = g.wrapping_add(1));
        quality_gen.update(|g| *g = g.wrapping_add(1));
        preload_busy.set(false);
        preload_queue.set(Vec::new());
        abort_active_fetches();

        let first_load = image.get_untracked().is_none();
        if first_load {
            loading.set(true);
        }
        load_status.set(String::new());
        let raw_owned = raw.clone();
        spawn_local(async move {
            let avoid = load_session_seen_urls();
            let result = load_media_batch(
                &raw_owned,
                &avoid,
                1 + PRELOAD_TARGET,
                LoadTier::Fast,
            )
            .await;

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
                Ok(_) | Err(_) => match load_random_image(&raw_owned, &avoid).await {
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
                },
            }

            if load_gen.get_untracked() != gen {
                return;
            }
            loading.set(false);
            if image.get_untracked().is_some() {
                if preload_queue.with_untracked(|q| q.len()) < PRELOAD_TARGET {
                    fill_preload_queue();
                }
                refine_queue_quality();
            }
        });
    };

    let on_play = Callback::new(move |_: ()| {
        load_media();
    });

    let on_sub_edit = Callback::new(move |_: ()| {
        cancel_in_flight();
        preload_queue.set(Vec::new());
        load_status.set(String::new());
    });

    Effect::new(move |_| {
        spawn_local(async {
            let _ = warm_community_caches().await;
        });
        if !subreddit.get_untracked().trim().is_empty() {
            fill_preload_queue();
        }
        // If we restored an already-unlocked post, seed gallery arming.
        if post_unlocked.get_untracked() {
            maybe_record_gallery();
        }
    });

    let on_clear_image = Callback::new(move |_: ()| {
        image.set(None);
        preload_queue.set(Vec::new());
        cancel_in_flight();
        slide_index.set(0);
        lightbox_open.set(false);
        load_status.set(String::new());
        clear_game_session();
        gallery_armed.set(None);
    });

    let on_open_full = Callback::new(move |_: ()| {
        lightbox_sharp.set(false);
        lightbox_open.set(true);
    });

    // Gallery: swap media only — keep board & reveal progress.
    let on_gallery_select = Callback::new(move |entry: GalleryEntry| {
        save_subreddit(&entry.media.subreddit);
        subreddit.set(entry.media.subreddit.clone());
        warm_media_cache(&entry.media);
        slide_index.set(0);
        image.set(Some(entry.media));
        lightbox_sharp.set(false);
        gallery_armed.set(None);
        // If already past goal, re-arm so we don't re-insert; mark armed after check.
        if post_unlocked.get_untracked() {
            maybe_record_gallery();
        }
        persist_session();
    });

    use_keyboard(apply_move);

    let play_locked = Signal::derive(move || loading.get() && image.get().is_none());

    let on_touch_start = move |ev: TouchEvent| {
        if play_locked.get_untracked() {
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
        if play_locked.get_untracked() {
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
                            if play_locked.get() {
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
                        <Show when=move || play_locked.get()>
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
                    <Show when=move || !has_image.get() && !loading.get()>
                        <section class="panel image-panel image-panel-empty" aria-label="How to start">
                            <h2 class="panel-title">"Reveal"</h2>
                            <p class="image-panel-meta">
                                "Pick a goal, enter a subreddit (or tap SFW / NSFW), then Play. The image unblurs as you approach the goal tile."
                            </p>
                        </section>
                    </Show>
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
                    <Gallery entries=gallery_sig on_select=on_gallery_select />
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
