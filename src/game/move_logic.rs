/// Slide and merge one line of 4 cells toward index 0 (left).
/// Returns (new line, score gained from merges).
///
/// Rules:
/// - Non-zero tiles pack toward the start
/// - Equal adjacent tiles merge once per move
/// - `[2,2,2,2]` → `[4,4,0,0]` (no chain merge to 8)
pub fn slide_line(line: [u32; 4]) -> ([u32; 4], u32) {
    let mut tiles: Vec<u32> = line.into_iter().filter(|&v| v != 0).collect();
    let mut score = 0u32;
    let mut i = 0;
    while i + 1 < tiles.len() {
        if tiles[i] == tiles[i + 1] {
            tiles[i] *= 2;
            score += tiles[i];
            tiles.remove(i + 1);
        }
        i += 1;
    }
    let mut out = [0u32; 4];
    for (idx, v) in tiles.into_iter().enumerate() {
        if idx < 4 {
            out[idx] = v;
        }
    }
    (out, score)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_stays_empty() {
        assert_eq!(slide_line([0, 0, 0, 0]), ([0, 0, 0, 0], 0));
    }

    #[test]
    fn slides_left() {
        assert_eq!(slide_line([0, 2, 0, 2]), ([4, 0, 0, 0], 4));
        assert_eq!(slide_line([0, 0, 0, 2]), ([2, 0, 0, 0], 0));
        assert_eq!(slide_line([2, 0, 0, 0]), ([2, 0, 0, 0], 0));
    }

    #[test]
    fn merge_once_no_chain() {
        assert_eq!(slide_line([2, 2, 2, 2]), ([4, 4, 0, 0], 8));
        assert_eq!(slide_line([4, 4, 4, 0]), ([8, 4, 0, 0], 8));
        assert_eq!(slide_line([2, 2, 4, 0]), ([4, 4, 0, 0], 4));
    }

    #[test]
    fn different_no_merge() {
        assert_eq!(slide_line([2, 4, 8, 16]), ([2, 4, 8, 16], 0));
    }
}
