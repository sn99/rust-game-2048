use crate::components::{BoardView, Header, Overlay};
use crate::game::{Board, Direction};
use crate::input::{
    direction_from_swipe, touch_end_delta, touch_start_coords, use_keyboard, TouchTracker,
};
use crate::storage::{load_best, save_best};
use leptos::prelude::*;
use web_sys::TouchEvent;

#[component]
pub fn App() -> impl IntoView {
    let rng = StoredValue::new(fastrand::Rng::new());
    let board = RwSignal::new(rng.try_update_value(|r| Board::new(r)).expect("rng"));
    let best = RwSignal::new(load_best());
    let touch = RwSignal::new(TouchTracker::default());

    let score = Signal::derive(move || board.get().score());
    let status = Signal::derive(move || board.get().status());
    let cells = Signal::derive(move || *board.get().cells());

    let apply_move = Callback::new(move |dir: Direction| {
        board.update(|b| {
            let moved = rng
                .try_update_value(|r| b.try_move(dir, r))
                .unwrap_or(false);
            if moved {
                let s = b.score();
                best.update(|best_score| {
                    if s > *best_score {
                        *best_score = s;
                        save_best(s);
                    }
                });
            }
        });
    });

    let new_game = Callback::new(move |_: ()| {
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
                <BoardView cells=cells />
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
