use super::move_logic::slide_line;

/// Move direction on the 4×4 board.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

/// High-level game status for overlays.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameStatus {
    Playing,
    /// Just reached 2048 for the first time; show win overlay.
    Won,
    /// No legal moves remain.
    Over,
}

/// Classic 4×4 2048 board.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Board {
    /// `cells[row][col]`, 0 = empty.
    cells: [[u32; 4]; 4],
    score: u32,
    status: GameStatus,
    /// True after the first 2048 tile appears (suppresses re-win overlay).
    won_once: bool,
}

impl Board {
    /// New game with two random tiles.
    pub fn new(rng: &mut fastrand::Rng) -> Self {
        let mut board = Self {
            cells: [[0; 4]; 4],
            score: 0,
            status: GameStatus::Playing,
            won_once: false,
        };
        board.spawn_tile(rng);
        board.spawn_tile(rng);
        board
    }

    /// Empty board (for tests). No random tiles.
    #[cfg(test)]
    pub fn empty() -> Self {
        Self {
            cells: [[0; 4]; 4],
            score: 0,
            status: GameStatus::Playing,
            won_once: false,
        }
    }

    #[cfg(test)]
    pub fn from_cells(cells: [[u32; 4]; 4]) -> Self {
        let mut board = Self {
            cells,
            score: 0,
            status: GameStatus::Playing,
            won_once: false,
        };
        board.refresh_status_after_setup();
        board
    }

    pub fn cells(&self) -> &[[u32; 4]; 4] {
        &self.cells
    }

    pub fn score(&self) -> u32 {
        self.score
    }

    pub fn status(&self) -> GameStatus {
        self.status
    }

    #[cfg(test)]
    pub fn won_once(&self) -> bool {
        self.won_once
    }

    /// After win overlay, continue playing without re-showing win.
    pub fn continue_after_win(&mut self) {
        if self.status == GameStatus::Won {
            self.status = GameStatus::Playing;
            self.won_once = true;
            if !self.has_moves() {
                self.status = GameStatus::Over;
            }
        }
    }

    /// Reset for New Game (caller keeps best score separately).
    pub fn reset(&mut self, rng: &mut fastrand::Rng) {
        *self = Self::new(rng);
    }

    /// Attempt a move. Returns whether the board changed.
    /// Spawns a tile only if something moved. Updates win/lose status.
    pub fn try_move(&mut self, dir: Direction, rng: &mut fastrand::Rng) -> bool {
        if self.status == GameStatus::Over || self.status == GameStatus::Won {
            // Block moves while win overlay is up; Over is terminal until reset.
            return false;
        }

        let (next, gained) = apply_move(&self.cells, dir);
        if next == self.cells {
            return false;
        }

        self.cells = next;
        self.score += gained;
        self.spawn_tile(rng);
        self.update_status_after_move();
        true
    }


    fn spawn_tile(&mut self, rng: &mut fastrand::Rng) {
        let empties: Vec<(usize, usize)> = (0..4)
            .flat_map(|r| (0..4).map(move |c| (r, c)))
            .filter(|&(r, c)| self.cells[r][c] == 0)
            .collect();
        if empties.is_empty() {
            return;
        }
        let (r, c) = empties[rng.usize(..empties.len())];
        // 90% → 2, 10% → 4
        self.cells[r][c] = if rng.f32() < 0.9 { 2 } else { 4 };
    }

    fn update_status_after_move(&mut self) {
        let has_2048 = self.cells.iter().flatten().any(|&v| v >= 2048);
        if has_2048 && !self.won_once {
            self.status = GameStatus::Won;
            self.won_once = true;
            return;
        }
        if !self.has_moves() {
            self.status = GameStatus::Over;
        } else {
            self.status = GameStatus::Playing;
        }
    }

    #[cfg(test)]
    fn refresh_status_after_setup(&mut self) {
        let has_2048 = self.cells.iter().flatten().any(|&v| v >= 2048);
        if has_2048 {
            self.won_once = true;
        }
        if !self.has_moves() {
            self.status = GameStatus::Over;
        }
    }

    fn has_moves(&self) -> bool {
        // Empty cell
        if self.cells.iter().flatten().any(|&v| v == 0) {
            return true;
        }
        // Horizontal merge
        for r in 0..4 {
            for c in 0..3 {
                if self.cells[r][c] == self.cells[r][c + 1] {
                    return true;
                }
            }
        }
        // Vertical merge
        for c in 0..4 {
            for r in 0..3 {
                if self.cells[r][c] == self.cells[r + 1][c] {
                    return true;
                }
            }
        }
        false
    }
}

fn apply_move(cells: &[[u32; 4]; 4], dir: Direction) -> ([[u32; 4]; 4], u32) {
    let mut out = [[0u32; 4]; 4];
    let mut score = 0u32;

    match dir {
        Direction::Left => {
            for r in 0..4 {
                let (line, s) = slide_line(cells[r]);
                out[r] = line;
                score += s;
            }
        }
        Direction::Right => {
            for r in 0..4 {
                let mut rev = cells[r];
                rev.reverse();
                let (mut line, s) = slide_line(rev);
                line.reverse();
                out[r] = line;
                score += s;
            }
        }
        Direction::Up => {
            for c in 0..4 {
                let col = [cells[0][c], cells[1][c], cells[2][c], cells[3][c]];
                let (line, s) = slide_line(col);
                for r in 0..4 {
                    out[r][c] = line[r];
                }
                score += s;
            }
        }
        Direction::Down => {
            for c in 0..4 {
                let col = [cells[3][c], cells[2][c], cells[1][c], cells[0][c]];
                let (line, s) = slide_line(col);
                for r in 0..4 {
                    out[3 - r][c] = line[r];
                }
                score += s;
            }
        }
    }

    (out, score)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rng() -> fastrand::Rng {
        fastrand::Rng::with_seed(42)
    }

    #[test]
    fn new_has_two_tiles() {
        let b = Board::new(&mut rng());
        let n = b.cells().iter().flatten().filter(|&&v| v != 0).count();
        assert_eq!(n, 2);
        assert_eq!(b.score(), 0);
        assert_eq!(b.status(), GameStatus::Playing);
    }

    #[test]
    fn slide_left_merges_and_scores() {
        let mut b = Board::from_cells([
            [2, 2, 0, 0],
            [0, 0, 0, 0],
            [0, 0, 0, 0],
            [0, 0, 0, 0],
        ]);
        // Use a board full of known state; after move one spawn appears.
        let moved = b.try_move(Direction::Left, &mut rng());
        assert!(moved);
        assert_eq!(b.cells()[0][0], 4);
        assert_eq!(b.score(), 4);
        // Exactly one extra non-zero from spawn (plus the 4)
        let nonzero = b.cells().iter().flatten().filter(|&&v| v != 0).count();
        assert_eq!(nonzero, 2);
    }

    #[test]
    fn no_move_no_spawn() {
        let cells = [
            [2, 4, 8, 16],
            [0, 0, 0, 0],
            [0, 0, 0, 0],
            [0, 0, 0, 0],
        ];
        let mut b = Board::from_cells(cells);
        let moved = b.try_move(Direction::Left, &mut rng());
        assert!(!moved);
        assert_eq!(b.cells(), &cells);
        assert_eq!(b.score(), 0);
    }

    #[test]
    fn no_double_merge_in_row() {
        let mut b = Board::from_cells([
            [2, 2, 2, 2],
            [0, 0, 0, 0],
            [0, 0, 0, 0],
            [0, 0, 0, 0],
        ]);
        b.try_move(Direction::Left, &mut rng());
        assert_eq!(b.cells()[0][0], 4);
        assert_eq!(b.cells()[0][1], 4);
        assert_eq!(b.score(), 8);
    }

    #[test]
    fn win_on_2048() {
        let mut b = Board::from_cells([
            [1024, 1024, 0, 0],
            [0, 0, 0, 0],
            [0, 0, 0, 0],
            [0, 0, 0, 0],
        ]);
        b.try_move(Direction::Left, &mut rng());
        assert_eq!(b.cells()[0][0], 2048);
        assert_eq!(b.status(), GameStatus::Won);
        // Moves blocked while Won
        assert!(!b.try_move(Direction::Left, &mut rng()));
        b.continue_after_win();
        assert_eq!(b.status(), GameStatus::Playing);
        // No second win overlay for higher tiles
        assert!(b.won_once());
    }

    #[test]
    fn game_over_when_stuck() {
        let b = Board::from_cells([
            [2, 4, 2, 4],
            [4, 2, 4, 2],
            [2, 4, 2, 4],
            [4, 2, 4, 2],
        ]);
        assert_eq!(b.status(), GameStatus::Over);
    }

    #[test]
    fn has_moves_when_adjacent_equal() {
        let b = Board::from_cells([
            [2, 2, 4, 8],
            [16, 32, 64, 128],
            [256, 512, 1024, 8],
            [16, 32, 64, 128],
        ]);
        assert_eq!(b.status(), GameStatus::Playing);
    }

    #[test]
    fn move_up_and_down() {
        let mut b = Board::from_cells([
            [0, 0, 0, 0],
            [0, 0, 0, 0],
            [0, 0, 0, 0],
            [2, 0, 0, 0],
        ]);
        assert!(b.try_move(Direction::Up, &mut rng()));
        assert_eq!(b.cells()[0][0], 2);

        let mut b = Board::from_cells([
            [2, 0, 0, 0],
            [0, 0, 0, 0],
            [0, 0, 0, 0],
            [0, 0, 0, 0],
        ]);
        assert!(b.try_move(Direction::Down, &mut rng()));
        assert_eq!(b.cells()[3][0], 2);
    }

    #[test]
    fn reset_clears_score() {
        let mut b = Board::from_cells([
            [2, 2, 0, 0],
            [0, 0, 0, 0],
            [0, 0, 0, 0],
            [0, 0, 0, 0],
        ]);
        b.try_move(Direction::Left, &mut rng());
        assert!(b.score() > 0);
        b.reset(&mut rng());
        assert_eq!(b.score(), 0);
        assert_eq!(b.status(), GameStatus::Playing);
    }
}
