# Design: rust-game-2048

**Date:** 2026-07-15  
**Status:** Approved  
**Goal:** A faithful classic 2048 browser game in Rust (Leptos CSR), hostable on GitHub Pages via GitHub Actions.

## Summary

Build a single-crate web game that matches [play2048.co](https://play2048.co) gameplay and presentation: 4×4 grid, slide/merge, score + best (localStorage), win at 2048 with option to continue, game over, keyboard + swipe, and classic tile styling with motion.

## Goals

- Faithful classic 2048 rules and UX (not a minimal demo).
- Pure game logic testable without a browser.
- Full UI in Rust via **Leptos** (client-side rendering only).
- Static deploy to **GitHub Pages** with **GitHub Actions** (Trunk).

## Non-goals

- Server-side rendering / hydration
- Multiplayer, accounts, leaderboards
- Undo / redo
- Tutorial mode / onboarding overlay
- Native desktop/mobile shells
- Audio

## Architecture

**Approach:** Layered single crate — pure `game` module + Leptos components + CSS.

```
rust-game-2048/
├── Cargo.toml
├── Trunk.toml
├── index.html
├── style.css
├── src/
│   ├── main.rs              # mount Leptos app
│   ├── app.rs               # root layout, signals, wiring
│   ├── game/
│   │   ├── mod.rs
│   │   ├── board.rs         # 4×4 grid, score, status
│   │   ├── move_logic.rs    # slide/merge per direction
│   │   └── rng.rs           # spawn 2 (90%) / 4 (10%)
│   ├── components/
│   │   ├── board.rs         # grid + tiles
│   │   ├── header.rs        # title, score, best, New Game
│   │   └── overlay.rs       # You win! / Game over
│   └── input.rs             # keyboard + touch swipe
├── .github/workflows/deploy.yml
└── README.md
```

### Data flow

1. User input (arrow keys / WASD / swipe) → `Direction`.
2. Pure `Board::try_move(dir)` → new board, score delta, whether anything moved.
3. If moved: spawn random tile; update score and best; re-evaluate win/lose.
4. Leptos signals update → re-render tiles and overlays.
5. Best score persisted in `localStorage` (key e.g. `rust-game-2048-best`).

### Why this shape

- Unit tests cover rules without WASM.
- CSS owns classic look and animations; components stay thin.
- CSR + Trunk output is a static `dist/` suitable for GitHub Pages.

## Game rules (classic)

| Rule | Behavior |
|------|----------|
| Grid | 4×4 cells; empty or power-of-two tile value |
| Start | Two random tiles (2 with 90%, 4 with 10%) |
| Move | All tiles slide fully in the chosen direction |
| Merge | Equal tiles merge once per move; value doubles; score += new value |
| Chain | No multi-merge in one move (e.g. `[2,2,2,2]` → `[4,4,0,0]` not `[8,...]`) |
| After move | If anything slid or merged, spawn one random tile on an empty cell |
| No-op | If nothing changed, do not spawn and do not change score |
| Win | First time a **2048** tile appears → win overlay; user may **Keep going** |
| Continue | After keep going, play continues; no second win overlay for higher tiles |
| Lose | No empty cells and no adjacent equal pairs → game over |
| New Game | Reset board and score; best score remains |
| Best | Max score ever; stored in `localStorage` |

### Pure logic API (sketch)

```rust
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Direction { Up, Down, Left, Right }

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum GameStatus { Playing, Won, Over }

pub struct Board {
    cells: [[u32; 4]; 4],
    score: u32,
    status: GameStatus,
    won_once: bool, // true after first 2048; suppresses re-win overlay
}

impl Board {
    pub fn new<R: Rng>(rng: &mut R) -> Self;
    pub fn try_move<R: Rng>(&mut self, dir: Direction, rng: &mut R) -> bool;
    pub fn score(&self) -> u32;
    pub fn status(&self) -> GameStatus;
    pub fn cells(&self) -> &[[u32; 4]; 4];
    pub fn reset<R: Rng>(&mut self, rng: &mut R);
}
```

`try_move` returns whether the board changed. RNG is injected for deterministic tests.

## UI / UX

### Layout (classic-inspired)

- Centered column: title **2048**, score box, best box, **New Game** button.
- Rounded 4×4 board with gaps; tiles colored by value (classic palette for 2…2048+).
- Short explanation under the board (join tiles, get to 2048).
- Responsive; playable on phone (touch) and desktop (keyboard).

### Input

| Source | Mapping |
|--------|---------|
| Keyboard | Arrow keys, WASD |
| Touch | Swipe ≥ ~30px; dominant axis wins |
| Buttons | New Game; Keep going / Try again on overlays |

Ignore input while an overlay requires a choice, except New Game.

### Overlays

- **You win!** — Keep going | Try again  
- **Game over!** — Try again  

### Animation

- CSS transitions for position (slide) and pop on merge/spawn.
- Prefer position/transform updates driven by post-move board state (good enough for classic feel).
- No requirement for pixel-perfect frame parity with play2048.co.

### Accessibility

- Focusable New Game and overlay actions.
- Visible focus styles.
- Board region labeled for assistive tech where practical.

## Testing

| Layer | Coverage |
|-------|----------|
| Unit (`game`) | Empty slide; single merge; no double-merge; score; spawn only when moved; win detection; game over detection; continue after win |
| Manual | Keyboard, swipe, localStorage best, GH Pages load |

Run unit tests with `cargo test` (native target). No mandatory browser E2E in v1.

## Build and deploy

### Tooling

- Rust stable + `wasm32-unknown-unknown`
- [Trunk](https://trunkrs.dev/) for bundling
- Leptos with CSR features only

### Local

```bash
trunk serve          # dev
trunk build --release
cargo test
```

### GitHub Pages

- Workflow on push to `main` (and optionally `workflow_dispatch`).
- Install Rust + wasm target + Trunk.
- `trunk build --release` (public URL base configured for project pages if needed, e.g. `/rust-game-2048/`).
- Upload `dist/` artifact; deploy with `actions/deploy-pages`.
- Repo Settings → Pages → Source: GitHub Actions.

### README

Document: play URL pattern, local setup, build, test, Pages enablement, license note that 2048 is a known game concept (implementation original).

## Dependencies (planned)

- `leptos` (CSR)
- `wasm-bindgen` / `web-sys` (keyboard, touch, localStorage) as needed by Leptos or thin wrappers
- `getrandom` or `fastrand` with WASM support for tile spawn
- `trunk` (dev/CI tool, not a crate dep)

Exact versions chosen at implementation time for current Leptos stable.

## Success criteria

1. Playable classic 2048 in the browser after Trunk build.
2. Score, best (persisted), New Game, win continue, game over behave as specified.
3. Keyboard and swipe work.
4. Unit tests cover core move/merge/win/lose cases.
5. Push to `main` deploys a working site on GitHub Pages.

## Implementation order (high level)

1. Scaffold crate + Trunk + empty Leptos shell.
2. Implement and unit-test pure `game` module.
3. Wire signals + board/header UI + CSS.
4. Input (keyboard, swipe) + overlays + localStorage.
5. Polish animations and responsive layout.
6. GitHub Actions deploy + README.

## Open decisions resolved

| Topic | Decision |
|-------|----------|
| Fidelity | Faithful classic clone |
| UI stack | Leptos CSR |
| Structure | Layered single crate |
| Deploy | GitHub Actions → Pages |
| Project name | `rust-game-2048` |
| Tutorial | Out of scope |
