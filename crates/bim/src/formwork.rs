//! Wall formwork generation (DetailCraft-style concrete detailing, clean-room):
//! a sheathing-sheet grid on both faces of every wall segment plus corner studs
//! at each vertex, so each wall and each corner gets its panels.
//!
//! Panels are metric plywood sheets: 1.20 m wide × 2.40 m tall, cut to fit at
//! run and height ends. Panel plan points sit on the wall *centerline*
//! segment; the renderer offsets them by `side * (thickness / 2 + standoff)` to
//! place the sheathing face. Heights are world z (level floor + base offset).

use bimcraft_geom::Vec2;
use serde::{Deserialize, Serialize};

use crate::Wall;

/// Sheet width along the wall (m).
pub const FORMWORK_PANEL_W: f64 = 1.2;
/// Sheet height (m).
pub const FORMWORK_PANEL_H: f64 = 2.4;
/// Sheathing standoff from the wall face (m).
pub const FORMWORK_OFFSET: f64 = 0.06;
/// Corner stud square side (m).
pub const CORNER_POST_SIZE: f64 = 0.1;

/// One sheathing sheet: `a`/`b` are the panel's base edge on the host wall
/// centerline segment (in segment order), `side` is -1 (left face) or +1
/// (right face), `z0..z1` the world-z band, and `row`/`col` the grid indices.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct FormworkPanel {
    pub side: f64,
    pub a: bimcraft_geom::Vec2,
    pub b: bimcraft_geom::Vec2,
    pub z0: f64,
    pub z1: f64,
    pub row: usize,
    pub col: usize,
}

impl FormworkPanel {
    pub fn width(&self) -> f64 {
        (self.b - self.a).length()
    }

    pub fn height(&self) -> f64 {
        self.z1 - self.z0
    }

    pub fn area(&self) -> f64 {
        self.width() * self.height()
    }
}

/// A corner stud: plan center, half-size, world z-span.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct CornerPost {
    pub center: bimcraft_geom::Vec2,
    pub half: f64,
    pub z0: f64,
    pub z1: f64,
}

/// Sheathing grid on both faces of every wall segment between `base_z` and
/// `top_z` (world z). Columns of [`FORMWORK_PANEL_W`] (last one cut) and rows
/// of [`FORMWORK_PANEL_H`] (bottom row starts at `base_z`, top row cut).
pub fn wall_formwork(wall: &Wall, base_z: f64, top_z: f64) -> Vec<FormworkPanel> {
    let mut out = Vec::new();
    let h = top_z - base_z;
    if h < 0.05 {
        return out;
    }
    let rows = ((h / FORMWORK_PANEL_H).ceil() as usize).max(1);
    for (a, b) in wall.segments() {
        let len = (b - a).length();
        if len < 0.05 {
            continue;
        }
        let dir = (b - a) / len;
        let cols = ((len / FORMWORK_PANEL_W).ceil() as usize).max(1);
        for side in [-1.0_f64, 1.0_f64] {
            for col in 0..cols {
                let s0 = col as f64 * FORMWORK_PANEL_W;
                let s1 = len.min(s0 + FORMWORK_PANEL_W);
                if s1 - s0 < 0.05 {
                    continue;
                }
                for row in 0..rows {
                    let z0 = base_z + row as f64 * FORMWORK_PANEL_H;
                    let z1 = top_z.min(base_z + (row + 1) as f64 * FORMWORK_PANEL_H);
                    if z1 - z0 < 0.02 {
                        continue;
                    }
                    out.push(FormworkPanel { side, a: a + dir * s0, b: a + dir * s1, z0, z1, row, col });
                }
            }
        }
    }
    out
}

/// Corner studs at every wall vertex on both faces (full height). These are
/// the "corners" of the formwork: a doubled stud boxing each panel run end.
pub fn wall_corner_posts(wall: &Wall, base_z: f64, top_z: f64) -> Vec<CornerPost> {
    let mut out = Vec::new();
    if top_z - base_z < 0.05 {
        return out;
    }
    let half = CORNER_POST_SIZE / 2.0;
    for (a, b) in wall.segments() {
        let dir = (b - a).normalize();
        let normal = Vec2::new(-dir.y, dir.x);
        for (v, side) in [(a, -1.0_f64), (a, 1.0_f64), (b, -1.0_f64), (b, 1.0_f64)] {
            let center = v + normal * side * (wall.thickness / 2.0 + FORMWORK_OFFSET);
            if out.iter().any(|p: &CornerPost| (p.center - center).length() < 1e-4) {
                continue;
            }
            out.push(CornerPost { center, half, z0: base_z, z1: top_z });
        }
    }
    out
}

/// Total sheathing area of a panel list, m².
pub fn formwork_area(panels: &[FormworkPanel]) -> f64 {
    panels.iter().map(|p| p.area()).sum()
}

/// Formwork for a whole model level: every wall, keyed by element id.
pub fn level_formwork(model: &crate::Model, level: crate::Id) -> Vec<(crate::Id, Vec<FormworkPanel>, Vec<CornerPost>)> {
    model
        .walls()
        .filter_map(|e| match &e.data {
            crate::ElementData::Wall(w) if w.level == level => {
                let base = w.base_z(model);
                let top = w.top_z(model);
                Some((e.id, wall_formwork(w, base, top), wall_corner_posts(w, base, top)))
            }
            _ => None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Model;
    use bimcraft_geom::Vec2;

    fn straight_wall() -> Wall {
        Wall { level: 0, pts: vec![Vec2::ZERO, Vec2::new(6.0, 0.0)], ..Wall::default() }
    }

    #[test]
    fn panel_grid_fits_height_and_length() {
        // 6 m long, 3.0 m tall: 5 columns x 2 rows x 2 faces = 20 panels.
        let w = straight_wall();
        let panels = wall_formwork(&w, 0.0, 3.0);
        assert_eq!(panels.len(), 20);
        assert!(panels.iter().all(|p| p.height() <= FORMWORK_PANEL_H + 1e-9));
        assert!(panels.iter().all(|p| p.width() <= FORMWORK_PANEL_W + 1e-9));
        // Bottom row sits on the base, top row ends at the top.
        assert!(panels.iter().any(|p| p.row == 0 && p.z0.abs() < 1e-9 && p.z1.abs() - FORMWORK_PANEL_H < 1e-9));
        assert!(panels.iter().any(|p| p.row == 1 && (p.z1 - 3.0).abs() < 1e-9 && (p.z0 - FORMWORK_PANEL_H).abs() < 1e-9));
        // Last column of each face is cut to the run end.
        let last = panels.iter().find(|p| p.side == 1.0 && p.col == 4 && p.row == 0).unwrap();
        assert!((last.width() - 1.2).abs() < 1e-9); // 6 / 1.2 = 5 exact
        assert!((last.b.x - 6.0).abs() < 1e-9);
    }

    #[test]
    fn cut_panels_on_odd_runs() {
        let w = Wall { level: 0, pts: vec![Vec2::ZERO, Vec2::new(5.0, 0.0)], height: 2.4, ..Wall::default() };
        let panels = wall_formwork(&w, 0.0, 2.4);
        // 5 m => 5 columns (last 0.2 m), 1 row, 2 faces = 10 panels.
        assert_eq!(panels.len(), 10);
        let cut = panels.iter().find(|p| p.col == 4).unwrap();
        assert!((cut.width() - 0.2).abs() < 1e-9);
    }

    #[test]
    fn corner_posts_on_both_faces() {
        let w = straight_wall();
        let posts = wall_corner_posts(&w, 0.0, 3.0);
        // Two end vertices x two faces = 4 posts.
        assert_eq!(posts.len(), 4);
        // Posts stand on the sheathing line, offset from the centerline.
        let p = posts.iter().find(|p| p.center.x < 0.5 && p.center.y > 0.0).unwrap();
        assert!((p.center.y - (0.1 + FORMWORK_OFFSET)).abs() < 1e-9);
        assert!((p.half - CORNER_POST_SIZE / 2.0).abs() < 1e-9);
    }

    #[test]
    fn area_sums_panels() {
        let w = straight_wall();
        let panels = wall_formwork(&w, 0.0, 3.0);
        let a = formwork_area(&panels);
        // 6 m run: both faces fully sheathed = 2 * 6 * 3 = 36 m².
        assert!((a - 36.0).abs() < 1e-9);
    }

    #[test]
    fn level_formwork_covers_walls() {
        let mut m = Model::new();
        let lvl = m.levels[0].id;
        m.add_wall(lvl, vec![Vec2::ZERO, Vec2::new(6.0, 0.0)], 0.2, 3.0);
        m.add_wall(lvl, vec![Vec2::new(0.0, 1.0), Vec2::new(3.0, 1.0)], 0.2, 3.0);
        let fw = level_formwork(&m, lvl);
        assert_eq!(fw.len(), 2);
        assert!(fw.iter().all(|(_, panels, _)| !panels.is_empty()));
    }
}
