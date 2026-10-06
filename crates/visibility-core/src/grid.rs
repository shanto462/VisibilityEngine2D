//! Flat uniform grid over item bounding boxes.
//!
//! Every cell's items live in one contiguous array (CSR layout), so a lookup is an
//! integer multiply-add and iterating a cell is a slice walk. No hashing, no per-cell
//! allocations.

use crate::geom::{Aabb, Vec2};

/// Uniform grid of item ids, built once from item bounding boxes.
#[derive(Clone, Debug)]
pub struct UniformGrid {
    origin: Vec2,
    cell: f64,
    inv_cell: f64,
    cols: usize,
    rows: usize,
    /// `starts[c]..starts[c + 1]` is the item range of cell `c`.
    starts: Vec<u32>,
    items: Vec<u32>,
}

/// Inclusive cell index range.
#[derive(Clone, Copy, Debug)]
struct CellRange {
    c0: usize,
    r0: usize,
    c1: usize,
    r1: usize,
}

impl UniformGrid {
    /// Builds the grid covering `bounds` with square cells of size `cell`.
    /// Item `i` is inserted into every cell its box overlaps.
    ///
    /// # Panics
    ///
    /// Panics if `cell` is not positive.
    pub fn build(bounds: Aabb, cell: f64, boxes: &[Aabb]) -> Self {
        assert!(cell > 0.0, "cell size must be positive");
        let bounds = if bounds.is_empty() {
            Aabb::new(Vec2::ZERO, Vec2::new(1.0, 1.0))
        } else {
            bounds
        };
        let cols = ((bounds.width() / cell).floor() as usize + 1).max(1);
        let rows = ((bounds.height() / cell).floor() as usize + 1).max(1);
        let mut grid = Self {
            origin: bounds.min,
            cell,
            inv_cell: 1.0 / cell,
            cols,
            rows,
            starts: vec![0; cols * rows + 1],
            items: Vec::new(),
        };

        // Pass 1: count items per cell.
        for b in boxes {
            if let Some(r) = grid.cell_range(b) {
                for row in r.r0..=r.r1 {
                    for col in r.c0..=r.c1 {
                        grid.starts[row * cols + col + 1] += 1;
                    }
                }
            }
        }
        // Prefix sum turns counts into offsets.
        for i in 1..grid.starts.len() {
            grid.starts[i] += grid.starts[i - 1];
        }
        // Pass 2: scatter ids.
        grid.items = vec![0; *grid.starts.last().unwrap() as usize];
        let mut cursor = grid.starts.clone();
        for (id, b) in boxes.iter().enumerate() {
            if let Some(r) = grid.cell_range(b) {
                for row in r.r0..=r.r1 {
                    for col in r.c0..=r.c1 {
                        let c = row * cols + col;
                        grid.items[cursor[c] as usize] = id as u32;
                        cursor[c] += 1;
                    }
                }
            }
        }
        grid
    }

    /// Number of (columns, rows).
    #[inline]
    pub fn dims(&self) -> (usize, usize) {
        (self.cols, self.rows)
    }

    #[inline]
    fn cell_items(&self, c: usize) -> &[u32] {
        &self.items[self.starts[c] as usize..self.starts[c + 1] as usize]
    }

    fn cell_range(&self, b: &Aabb) -> Option<CellRange> {
        let max_c = (self.cols - 1) as f64;
        let max_r = (self.rows - 1) as f64;
        let c0 = ((b.min.x - self.origin.x) * self.inv_cell).floor();
        let c1 = ((b.max.x - self.origin.x) * self.inv_cell).floor();
        let r0 = ((b.min.y - self.origin.y) * self.inv_cell).floor();
        let r1 = ((b.max.y - self.origin.y) * self.inv_cell).floor();
        if c1 < 0.0 || r1 < 0.0 || c0 > max_c || r0 > max_r || b.is_empty() {
            return None;
        }
        Some(CellRange {
            c0: c0.clamp(0.0, max_c) as usize,
            r0: r0.clamp(0.0, max_r) as usize,
            c1: c1.clamp(0.0, max_c) as usize,
            r1: r1.clamp(0.0, max_r) as usize,
        })
    }

    /// Items whose cells overlap `b`. Each id is reported once (deduplicated with `stamps`).
    pub fn for_each_in_aabb(&self, b: &Aabb, stamps: &mut Stamps, mut f: impl FnMut(u32)) {
        let Some(r) = self.cell_range(b) else { return };
        stamps.next_epoch();
        for row in r.r0..=r.r1 {
            for col in r.c0..=r.c1 {
                for &id in self.cell_items(row * self.cols + col) {
                    if stamps.mark(id) {
                        f(id);
                    }
                }
            }
        }
    }

    /// Items in the single cell that contains `p` (may contain items whose box does not contain `p`).
    pub fn items_at(&self, p: Vec2) -> &[u32] {
        match self.cell_range(&Aabb::new(p, p)) {
            Some(r) => self.cell_items(r.r0 * self.cols + r.c0),
            None => &[],
        }
    }

    /// Walks the cells pierced by the ray `origin + t * dir` for `t` in `[0, t_max]`, nearest first
    /// (Amanatides and Woo DDA). `visit(items, t_enter, t_exit)` returns `false` to stop early.
    pub fn traverse_ray(&self, origin: Vec2, dir: Vec2, t_max: f64, mut visit: impl FnMut(&[u32], f64, f64) -> bool) {
        // Clip the ray to the grid box first (slab test), so origins outside the grid work too.
        let gmin = self.origin;
        let gmax = Vec2::new(
            gmin.x + self.cols as f64 * self.cell,
            gmin.y + self.rows as f64 * self.cell,
        );
        let mut t0: f64 = 0.0;
        let mut t1 = t_max;
        for (o, d, lo, hi) in [(origin.x, dir.x, gmin.x, gmax.x), (origin.y, dir.y, gmin.y, gmax.y)] {
            if d.abs() < 1e-300 {
                if o < lo || o > hi {
                    return;
                }
            } else {
                let inv = 1.0 / d;
                let (mut a, mut b) = ((lo - o) * inv, (hi - o) * inv);
                if a > b {
                    std::mem::swap(&mut a, &mut b);
                }
                t0 = t0.max(a);
                t1 = t1.min(b);
            }
        }
        if t0 > t1 {
            return;
        }

        let start = origin + dir * t0;
        let gx = (start.x - gmin.x) * self.inv_cell;
        let gy = (start.y - gmin.y) * self.inv_cell;
        let mut col = (gx.floor() as isize).clamp(0, self.cols as isize - 1);
        let mut row = (gy.floor() as isize).clamp(0, self.rows as isize - 1);

        let (step_c, mut next_x, delta_x) = axis_setup(dir.x, origin.x, gmin.x, col, self.cell);
        let (step_r, mut next_y, delta_y) = axis_setup(dir.y, origin.y, gmin.y, row, self.cell);

        let mut t_enter = t0;
        loop {
            let t_exit = next_x.min(next_y).min(t1);
            let c = row as usize * self.cols + col as usize;
            if !visit(self.cell_items(c), t_enter, t_exit) || t_exit >= t1 {
                return;
            }
            t_enter = t_exit;
            if next_x < next_y {
                col += step_c;
                next_x += delta_x;
                if col < 0 || col >= self.cols as isize {
                    return;
                }
            } else {
                row += step_r;
                next_y += delta_y;
                if row < 0 || row >= self.rows as isize {
                    return;
                }
            }
        }
    }
}

/// Returns (step, t of first boundary crossing, t between crossings) for one axis.
#[inline]
fn axis_setup(d: f64, o: f64, gmin: f64, cell_index: isize, cell: f64) -> (isize, f64, f64) {
    if d > 0.0 {
        let boundary = gmin + (cell_index + 1) as f64 * cell;
        (1, (boundary - o) / d, cell / d)
    } else if d < 0.0 {
        let boundary = gmin + cell_index as f64 * cell;
        (-1, (boundary - o) / d, -cell / d)
    } else {
        (0, f64::INFINITY, f64::INFINITY)
    }
}

/// Generation-stamped visited set: O(1) clear, no hashing.
#[derive(Clone, Debug, Default)]
pub struct Stamps {
    marks: Vec<u32>,
    epoch: u32,
}

impl Stamps {
    /// A visited-set for ids in `0..len`.
    pub fn new(len: usize) -> Self {
        Self {
            marks: vec![0; len],
            epoch: 0,
        }
    }

    /// Grows the set to hold ids in `0..len`.
    pub fn resize(&mut self, len: usize) {
        if self.marks.len() < len {
            self.marks.resize(len, 0);
        }
    }

    /// Clears the set in O(1) (starts a new epoch).
    #[inline]
    pub fn next_epoch(&mut self) {
        self.epoch = self.epoch.wrapping_add(1);
        if self.epoch == 0 {
            self.marks.fill(0);
            self.epoch = 1;
        }
    }

    /// Marks `id` and returns `true` the first time it is seen in this epoch.
    #[inline]
    pub fn mark(&mut self, id: u32) -> bool {
        let slot = &mut self.marks[id as usize];
        if *slot == self.epoch {
            false
        } else {
            *slot = self.epoch;
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_dedupes_items_spanning_cells() {
        let boxes = [
            Aabb::new(Vec2::new(0.0, 0.0), Vec2::new(25.0, 25.0)),
            Aabb::new(Vec2::new(80.0, 80.0), Vec2::new(90.0, 90.0)),
        ];
        let g = UniformGrid::build(Aabb::new(Vec2::ZERO, Vec2::new(100.0, 100.0)), 10.0, &boxes);
        let mut stamps = Stamps::new(boxes.len());
        let mut seen = Vec::new();
        g.for_each_in_aabb(&Aabb::new(Vec2::ZERO, Vec2::new(50.0, 50.0)), &mut stamps, |id| {
            seen.push(id);
        });
        assert_eq!(seen, vec![0]);
    }

    #[test]
    fn ray_visits_cells_in_order() {
        let g = UniformGrid::build(Aabb::new(Vec2::ZERO, Vec2::new(99.0, 99.0)), 10.0, &[]);
        let mut enters = Vec::new();
        g.traverse_ray(Vec2::new(5.0, 5.0), Vec2::new(1.0, 0.0), 40.0, |_, t0, t1| {
            enters.push((t0, t1));
            true
        });
        assert_eq!(enters.len(), 5);
        assert!(enters.windows(2).all(|w| (w[0].1 - w[1].0).abs() < 1e-12));
        assert_eq!(enters.last().unwrap().1, 40.0);
    }
}
