//! spatial(M8)— CPU 空间索引(Plan 3 L3,0011 §3.3②)。
//!
//! 均匀网格:对象按 AABB 入所有覆盖到的格;视口查询 → 可见对象集。CPU 侧管对象(裁剪 /
//! hit-test / 脏区),GPU 侧另用扁平实例;两消费者两套结构,不互掺(§3.3②)。纯逻辑,native 可测。

use std::collections::{HashMap, HashSet};

use crate::camera::Rect;

/// 默认格边长(世界 px)。
const DEFAULT_CELL: f32 = 256.0;

/// 均匀网格空间索引。对象用 `usize` id(app 里是块下标)。
pub struct SpatialGrid {
    cells: HashMap<(i32, i32), Vec<usize>>,
    cell: f32,
}

impl Default for SpatialGrid {
    fn default() -> Self {
        Self::new()
    }
}

impl SpatialGrid {
    pub fn new() -> Self {
        Self::with_cell(DEFAULT_CELL)
    }

    pub fn with_cell(cell: f32) -> Self {
        Self {
            cells: HashMap::new(),
            cell: cell.max(1.0),
        }
    }

    pub fn clear(&mut self) {
        self.cells.clear();
    }

    fn cell_range(&self, r: &Rect) -> (i32, i32, i32, i32) {
        let c = self.cell;
        (
            (r.x / c).floor() as i32,
            (r.y / c).floor() as i32,
            ((r.x + r.w) / c).floor() as i32,
            ((r.y + r.h) / c).floor() as i32,
        )
    }

    /// 把对象 `id` 按其 AABB 插入所有覆盖格。
    pub fn insert(&mut self, id: usize, r: &Rect) {
        let (x0, y0, x1, y1) = self.cell_range(r);
        for cy in y0..=y1 {
            for cx in x0..=x1 {
                self.cells.entry((cx, cy)).or_default().push(id);
            }
        }
    }

    /// 查询与 `r` 相交格里的对象(去重 + 升序,确定性)。
    pub fn query(&self, r: &Rect) -> Vec<usize> {
        let (x0, y0, x1, y1) = self.cell_range(r);
        let mut set: HashSet<usize> = HashSet::new();
        for cy in y0..=y1 {
            for cx in x0..=x1 {
                if let Some(ids) = self.cells.get(&(cx, cy)) {
                    set.extend(ids.iter().copied());
                }
            }
        }
        let mut out: Vec<usize> = set.into_iter().collect();
        out.sort_unstable();
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_returns_overlapping_only() {
        let mut g = SpatialGrid::with_cell(100.0);
        g.insert(0, &Rect::new(0.0, 0.0, 50.0, 50.0)); // 近原点
        g.insert(1, &Rect::new(0.0, 1000.0, 50.0, 50.0)); // 远处
        let vis = g.query(&Rect::new(0.0, 0.0, 200.0, 200.0));
        assert_eq!(vis, vec![0], "只应命中原点附近的对象");
    }

    #[test]
    fn spanning_object_found_from_any_cell() {
        let mut g = SpatialGrid::with_cell(100.0);
        g.insert(7, &Rect::new(50.0, 50.0, 300.0, 300.0)); // 跨多格
        assert_eq!(g.query(&Rect::new(320.0, 320.0, 10.0, 10.0)), vec![7]);
        assert_eq!(g.query(&Rect::new(60.0, 60.0, 10.0, 10.0)), vec![7]);
    }

    #[test]
    fn dedup_and_sorted() {
        let mut g = SpatialGrid::with_cell(50.0);
        g.insert(3, &Rect::new(0.0, 0.0, 500.0, 10.0)); // 横跨多格
        g.insert(1, &Rect::new(0.0, 0.0, 10.0, 10.0));
        let vis = g.query(&Rect::new(0.0, 0.0, 500.0, 10.0));
        assert_eq!(vis, vec![1, 3], "去重 + 升序");
    }

    #[test]
    fn clear_empties() {
        let mut g = SpatialGrid::new();
        g.insert(0, &Rect::new(0.0, 0.0, 10.0, 10.0));
        g.clear();
        assert!(g.query(&Rect::new(0.0, 0.0, 100.0, 100.0)).is_empty());
    }
}
