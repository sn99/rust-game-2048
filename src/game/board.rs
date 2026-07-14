use super::move_logic::{slide_tile_line, LineTile, SlidTile};

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
    /// Just reached the win target for the first time; show win overlay.
    Won,
    /// No legal moves remain.
    Over,
}

/// A single tile with stable identity for CSS transitions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tile {
    pub id: u64,
    pub value: u32,
    pub row: u8,
    pub col: u8,
    /// Freshly spawned — play appear animation (not slide).
    pub is_new: bool,
    /// Just created by a merge — play pop animation.
    pub is_merged: bool,
}

/// Classic 4×4 2048 board with identity-tracked tiles.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Board {
    tiles: Vec<Tile>,
    next_id: u64,
    score: u32,
    status: GameStatus,
    /// Win when a tile reaches this value (64, 128, …).
    win_tile: u32,
    /// True after the goal was reached once (suppresses re-win overlay).
    won_once: bool,
}

impl Board {
    /// New game with two random tiles. `win_tile` is the goal (e.g. 256, 2048).
    pub fn new(rng: &mut fastrand::Rng, win_tile: u32) -> Self {
        let mut board = Self {
            tiles: Vec::new(),
            next_id: 1,
            score: 0,
            status: GameStatus::Playing,
            win_tile: win_tile.max(2),
            won_once: false,
        };
        board.spawn_tile(rng, true);
        board.spawn_tile(rng, true);
        board
    }

    /// Empty board (for tests). No random tiles.
    #[cfg(test)]
    pub fn empty() -> Self {
        Self {
            tiles: Vec::new(),
            next_id: 1,
            score: 0,
            status: GameStatus::Playing,
            win_tile: 2048,
            won_once: false,
        }
    }

    #[cfg(test)]
    pub fn from_cells(cells: [[u32; 4]; 4]) -> Self {
        Self::from_cells_with_goal(cells, 2048)
    }

    #[cfg(test)]
    pub fn from_cells_with_goal(cells: [[u32; 4]; 4], win_tile: u32) -> Self {
        let mut board = Self {
            tiles: Vec::new(),
            next_id: 1,
            score: 0,
            status: GameStatus::Playing,
            win_tile: win_tile.max(2),
            won_once: false,
        };
        for r in 0..4 {
            for c in 0..4 {
                if cells[r][c] != 0 {
                    board.push_tile(r as u8, c as u8, cells[r][c], false, false);
                }
            }
        }
        board.refresh_status_after_setup();
        board
    }

    pub fn tiles(&self) -> &[Tile] {
        &self.tiles
    }

    /// Grid view for tests / debugging.
    pub fn cells(&self) -> [[u32; 4]; 4] {
        let mut cells = [[0u32; 4]; 4];
        for t in &self.tiles {
            cells[t.row as usize][t.col as usize] = t.value;
        }
        cells
    }

    pub fn score(&self) -> u32 {
        self.score
    }

    /// Highest tile value on the board (0 if empty).
    pub fn max_tile(&self) -> u32 {
        self.tiles.iter().map(|t| t.value).max().unwrap_or(0)
    }

    pub fn win_tile(&self) -> u32 {
        self.win_tile
    }

    /// Change goal without reshuffling (used when starting a new game at a new level).
    pub fn set_win_tile(&mut self, win_tile: u32) {
        self.win_tile = win_tile.max(2);
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
        let goal = self.win_tile;
        *self = Self::new(rng, goal);
    }

    /// New game at a chosen difficulty goal.
    pub fn reset_with_goal(&mut self, rng: &mut fastrand::Rng, win_tile: u32) {
        *self = Self::new(rng, win_tile);
    }

    /// Slide/merge only (no spawn). Returns whether anything changed.
    /// Call [`spawn_after_move`] after the slide animation (~100ms).
    pub fn try_move(&mut self, dir: Direction) -> bool {
        if self.status == GameStatus::Over || self.status == GameStatus::Won {
            return false;
        }

        // Clear animation flags so prior new/merge classes don't stick.
        for t in &mut self.tiles {
            t.is_new = false;
            t.is_merged = false;
        }

        let (next_tiles, gained, moved) = apply_tile_move(&self.tiles, dir);
        if !moved {
            return false;
        }

        self.tiles = next_tiles;
        self.score += gained;
        self.check_win_only();
        true
    }

    /// Spawn one random tile after a successful move, then re-evaluate lose.
    pub fn spawn_after_move(&mut self, rng: &mut fastrand::Rng) {
        self.spawn_tile(rng, true);
        self.finalize_status_after_spawn();
    }

    /// Full move used by tests (slide + immediate spawn).
    #[cfg(test)]
    pub fn try_move_with_spawn(&mut self, dir: Direction, rng: &mut fastrand::Rng) -> bool {
        if !self.try_move(dir) {
            return false;
        }
        self.spawn_after_move(rng);
        true
    }

    fn push_tile(&mut self, row: u8, col: u8, value: u32, is_new: bool, is_merged: bool) {
        let id = self.next_id;
        self.next_id += 1;
        self.tiles.push(Tile {
            id,
            value,
            row,
            col,
            is_new,
            is_merged,
        });
    }

    fn spawn_tile(&mut self, rng: &mut fastrand::Rng, is_new: bool) {
        let occupied: [[bool; 4]; 4] = {
            let mut o = [[false; 4]; 4];
            for t in &self.tiles {
                o[t.row as usize][t.col as usize] = true;
            }
            o
        };
        let empties: Vec<(u8, u8)> = (0..4u8)
            .flat_map(|r| (0..4u8).map(move |c| (r, c)))
            .filter(|&(r, c)| !occupied[r as usize][c as usize])
            .collect();
        if empties.is_empty() {
            return;
        }
        let (r, c) = empties[rng.usize(..empties.len())];
        let value = if rng.f32() < 0.9 { 2 } else { 4 };
        self.push_tile(r, c, value, is_new, false);
    }

    fn check_win_only(&mut self) {
        if self.max_tile() >= self.win_tile && !self.won_once {
            self.status = GameStatus::Won;
            self.won_once = true;
        }
    }

    fn finalize_status_after_spawn(&mut self) {
        // Keep win overlay until user continues.
        if self.status == GameStatus::Won {
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
        if self.max_tile() >= self.win_tile {
            self.won_once = true;
        }
        if !self.has_moves() {
            self.status = GameStatus::Over;
        }
    }

    fn has_moves(&self) -> bool {
        let cells = self.cells();
        if cells.iter().flatten().any(|&v| v == 0) {
            return true;
        }
        for r in 0..4 {
            for c in 0..3 {
                if cells[r][c] == cells[r][c + 1] {
                    return true;
                }
            }
        }
        for c in 0..4 {
            for r in 0..3 {
                if cells[r][c] == cells[r + 1][c] {
                    return true;
                }
            }
        }
        false
    }
}

fn grid_from_tiles(tiles: &[Tile]) -> [[Option<LineTile>; 4]; 4] {
    let mut grid = [[None; 4]; 4];
    for t in tiles {
        grid[t.row as usize][t.col as usize] = Some(LineTile {
            id: t.id,
            value: t.value,
        });
    }
    grid
}

fn tiles_from_grid(grid: [[Option<SlidTile>; 4]; 4]) -> Vec<Tile> {
    let mut out = Vec::new();
    for r in 0..4 {
        for c in 0..4 {
            if let Some(t) = grid[r][c] {
                out.push(Tile {
                    id: t.id,
                    value: t.value,
                    row: r as u8,
                    col: c as u8,
                    is_new: false,
                    is_merged: t.is_merged,
                });
            }
        }
    }
    out
}

fn apply_tile_move(tiles: &[Tile], dir: Direction) -> (Vec<Tile>, u32, bool) {
    let before = grid_from_tiles(tiles);
    let mut after = [[None; 4]; 4];
    let mut score = 0u32;

    match dir {
        Direction::Left => {
            for r in 0..4 {
                let (line, s) = slide_tile_line(before[r]);
                after[r] = line;
                score += s;
            }
        }
        Direction::Right => {
            for r in 0..4 {
                let mut rev = before[r];
                rev.reverse();
                let (mut line, s) = slide_tile_line(rev);
                line.reverse();
                after[r] = line;
                score += s;
            }
        }
        Direction::Up => {
            for c in 0..4 {
                let col = [before[0][c], before[1][c], before[2][c], before[3][c]];
                let (line, s) = slide_tile_line(col);
                for r in 0..4 {
                    after[r][c] = line[r];
                }
                score += s;
            }
        }
        Direction::Down => {
            for c in 0..4 {
                let col = [before[3][c], before[2][c], before[1][c], before[0][c]];
                let (line, s) = slide_tile_line(col);
                for r in 0..4 {
                    after[3 - r][c] = line[r];
                }
                score += s;
            }
        }
    }

    let next = tiles_from_grid(after);

    let mut old_vals = [[0u32; 4]; 4];
    for t in tiles {
        old_vals[t.row as usize][t.col as usize] = t.value;
    }
    let mut new_vals = [[0u32; 4]; 4];
    for t in &next {
        new_vals[t.row as usize][t.col as usize] = t.value;
    }
    let moved = old_vals != new_vals;

    (next, score, moved)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rng() -> fastrand::Rng {
        fastrand::Rng::with_seed(42)
    }

    #[test]
    fn new_has_two_tiles() {
        let b = Board::new(&mut rng(), 2048);
        assert_eq!(b.tiles().len(), 2);
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
        let moved = b.try_move_with_spawn(Direction::Left, &mut rng());
        assert!(moved);
        assert_eq!(b.cells()[0][0], 4);
        assert_eq!(b.score(), 4);
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
        let moved = b.try_move_with_spawn(Direction::Left, &mut rng());
        assert!(!moved);
        assert_eq!(b.cells(), cells);
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
        b.try_move_with_spawn(Direction::Left, &mut rng());
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
        b.try_move_with_spawn(Direction::Left, &mut rng());
        assert_eq!(b.cells()[0][0], 2048);
        assert_eq!(b.status(), GameStatus::Won);
        assert!(!b.try_move(Direction::Left));
        b.continue_after_win();
        assert_eq!(b.status(), GameStatus::Playing);
        assert!(b.won_once());
    }

    #[test]
    fn win_on_custom_goal() {
        let mut b = Board::from_cells_with_goal(
            [
                [32, 32, 0, 0],
                [0, 0, 0, 0],
                [0, 0, 0, 0],
                [0, 0, 0, 0],
            ],
            64,
        );
        b.try_move_with_spawn(Direction::Left, &mut rng());
        assert_eq!(b.cells()[0][0], 64);
        assert_eq!(b.status(), GameStatus::Won);
        assert_eq!(b.win_tile(), 64);
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
        assert!(b.try_move_with_spawn(Direction::Up, &mut rng()));
        assert_eq!(b.cells()[0][0], 2);

        let mut b = Board::from_cells([
            [2, 0, 0, 0],
            [0, 0, 0, 0],
            [0, 0, 0, 0],
            [0, 0, 0, 0],
        ]);
        assert!(b.try_move_with_spawn(Direction::Down, &mut rng()));
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
        b.try_move_with_spawn(Direction::Left, &mut rng());
        assert!(b.score() > 0);
        b.reset(&mut rng());
        assert_eq!(b.score(), 0);
        assert_eq!(b.status(), GameStatus::Playing);
    }

    #[test]
    fn slide_preserves_tile_id() {
        let mut b = Board::from_cells([
            [0, 0, 0, 2],
            [0, 0, 0, 0],
            [0, 0, 0, 0],
            [0, 0, 0, 0],
        ]);
        let id = b.tiles()[0].id;
        b.try_move(Direction::Left);
        assert_eq!(b.tiles().len(), 1);
        assert_eq!(b.tiles()[0].id, id);
        assert_eq!(b.tiles()[0].col, 0);
        assert_eq!(b.tiles()[0].value, 2);
    }
}
