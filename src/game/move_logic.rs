//! Line slide/merge for both bare values (tests) and tracked tiles (animation).

/// Slide and merge one line of 4 values toward index 0.
/// `[2,2,2,2]` → `[4,4,0,0]` (no chain merge).
#[cfg_attr(not(test), allow(dead_code))]
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

/// One cell in a line before sliding (for identity-preserving merges).
#[derive(Clone, Copy, Debug)]
pub struct LineTile {
    pub id: u64,
    pub value: u32,
}

/// Result cell after sliding.
#[derive(Clone, Copy, Debug)]
pub struct SlidTile {
    pub id: u64,
    pub value: u32,
    pub is_merged: bool,
}

/// Slide/merge a line of optional tiles toward index 0, preserving survivor ids.
pub fn slide_tile_line(line: [Option<LineTile>; 4]) -> ([Option<SlidTile>; 4], u32) {
    let packed: Vec<LineTile> = line.into_iter().flatten().collect();
    let mut merged: Vec<SlidTile> = Vec::with_capacity(4);
    let mut score = 0u32;
    let mut i = 0;
    while i < packed.len() {
        if i + 1 < packed.len() && packed[i].value == packed[i + 1].value {
            let value = packed[i].value * 2;
            score += value;
            merged.push(SlidTile {
                id: packed[i].id, // keep the first tile's identity for CSS transition
                value,
                is_merged: true,
            });
            i += 2;
        } else {
            merged.push(SlidTile {
                id: packed[i].id,
                value: packed[i].value,
                is_merged: false,
            });
            i += 1;
        }
    }
    let mut out = [None; 4];
    for (idx, t) in merged.into_iter().enumerate() {
        if idx < 4 {
            out[idx] = Some(t);
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

    #[test]
    fn tile_line_keeps_first_id_on_merge() {
        let line = [
            Some(LineTile { id: 1, value: 2 }),
            Some(LineTile { id: 2, value: 2 }),
            None,
            None,
        ];
        let (out, score) = slide_tile_line(line);
        assert_eq!(score, 4);
        let t = out[0].unwrap();
        assert_eq!(t.id, 1);
        assert_eq!(t.value, 4);
        assert!(t.is_merged);
        assert!(out[1].is_none());
    }
}
