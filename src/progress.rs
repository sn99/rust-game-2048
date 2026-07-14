//! Map board strength → background reveal / blur.

/// 0.0 at `from` (or below), 1.0 at `to` (or above). Linear in log2.
pub fn reveal_progress_range(max_tile: u32, from: u32, to: u32) -> f32 {
    if max_tile == 0 {
        return 0.0;
    }
    let from = from.max(2);
    let to = to.max(from);
    if max_tile <= from {
        return 0.0;
    }
    if max_tile >= to {
        return 1.0;
    }
    let a = (from as f32).log2();
    let b = (to as f32).log2();
    if (b - a).abs() < f32::EPSILON {
        return 1.0;
    }
    let log = (max_tile as f32).log2();
    ((log - a) / (b - a)).clamp(0.0, 1.0)
}

/// Classic: from tile 2 up to win goal.
pub fn reveal_progress(max_tile: u32, win_tile: u32) -> f32 {
    reveal_progress_range(max_tile, 2, win_tile.max(2))
}

pub fn blur_from_progress(progress: f32) -> f32 {
    const MAX_BLUR: f32 = 42.0;
    MAX_BLUR * (1.0 - progress.clamp(0.0, 1.0))
}

pub fn opacity_from_progress(progress: f32) -> f32 {
    0.45 + 0.55 * progress.clamp(0.0, 1.0)
}

pub fn veil_from_progress(progress: f32) -> f32 {
    0.72 - 0.42 * progress.clamp(0.0, 1.0)
}

/// CSS `blur()` radius in px.
pub fn blur_px(max_tile: u32, from: u32, to: u32) -> f32 {
    blur_from_progress(reveal_progress_range(max_tile, from, to))
}

pub fn image_opacity(max_tile: u32, from: u32, to: u32) -> f32 {
    opacity_from_progress(reveal_progress_range(max_tile, from, to))
}

pub fn veil_opacity(max_tile: u32, from: u32, to: u32) -> f32 {
    veil_from_progress(reveal_progress_range(max_tile, from, to))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_is_blurry() {
        assert!(blur_px(2, 2, 256) > 35.0);
        assert!(reveal_progress(2, 256) < 0.05);
    }

    #[test]
    fn mid_game_depends_on_goal() {
        let p_easy = reveal_progress(64, 256);
        let p_hard = reveal_progress(64, 2048);
        assert!(p_easy > p_hard);
        assert!(reveal_progress(128, 256) > 0.5);
        assert!(reveal_progress(128, 256) < 1.0);
    }

    #[test]
    fn win_is_fully_clear_any_level() {
        for goal in [64, 128, 256, 512, 1024, 2048, 4096] {
            assert_eq!(reveal_progress(goal, goal), 1.0);
            assert_eq!(blur_px(goal, 2, goal), 0.0);
        }
    }

    #[test]
    fn keep_going_range_starts_blurred() {
        // After winning at 16, new media unblurs from 16 → 32
        assert_eq!(reveal_progress_range(16, 16, 32), 0.0);
        assert!(reveal_progress_range(24, 16, 32) > 0.0); // won't happen in game but
        assert_eq!(reveal_progress_range(32, 16, 32), 1.0);
    }
}
