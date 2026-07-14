//! Pure 2048 game logic (no DOM / WASM).

mod board;
mod move_logic;

pub use board::{Board, Direction, GameStatus};
