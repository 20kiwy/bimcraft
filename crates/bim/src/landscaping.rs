//! Site / landscaping elements (clean-room, following the CadCraft library's
//! entourage conventions: mm-scale low-poly entourage, plan symbols on a
//! green PLANTING layer, insertion at the object's ground center).
//!
//! Point objects (trees, shrubs) sit at [`SiteElement::pos`]; chain objects
//! (fence runs) and polygon objects (grass, flower beds, paths, driveways)
//! use [`SiteElement::pts`]. Colors and sizes mirror the openly licensed
//! entourage set (`tree-deciduous-6m` … `hedge-2000`) plus new fence/grass/
//! flower presets the library lacks.

use bimcraft_geom::Vec2;
use serde::{Deserialize, Serialize};

use crate::Id;

/// What kind of site element this is and how it is placed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SiteKind {
    TreeDeciduous6,
    TreeDeciduous10,
    TreeConifer8,
    TreeColumnar7,
    TreeOrnamental4,
    ShrubRound,
    ShrubSmall,
    Hedge,
    PottedPlant,
    /// Fence run (chain of posts + panels between pickets).
    Fence,
    /// Grass / lawn patch (closed polygon).
    Grass,
    /// Flower bed (closed polygon, flowers in rows).
    FlowerBed,
    /// Garden path (closed polygon strip).
    Path,
    /// Driveway (closed polygon strip).
    Driveway,
}

/// How a kind is drawn / clicked into being.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SiteShape {
    /// A single point (click to place).
    Point,
    /// A polyline chain (click points, Enter finishes).
    Chain,
    /// A closed polygon (click points, Enter closes).
    Polygon,
}

impl SiteKind {
    pub const ALL: [SiteKind; 14] = [
        SiteKind::TreeDeciduous6,
        SiteKind::TreeDeciduous10,
        SiteKind::TreeConifer8,
        SiteKind::TreeColumnar7,
        SiteKind::TreeOrnamental4,
        SiteKind::ShrubRound,
        SiteKind::ShrubSmall,
        SiteKind::Hedge,
        SiteKind::PottedPlant,
        SiteKind::Fence,
        SiteKind::Grass,
        SiteKind::FlowerBed,
        SiteKind::Path,
        SiteKind::Driveway,
    ];

    pub fn name(self) -> &'static str {
        match self {
            SiteKind::TreeDeciduous6 => "Deciduous tree 6 m",
            SiteKind::TreeDeciduous10 => "Deciduous tree 10 m",
            SiteKind::TreeConifer8 => "Conifer 8 m",
            SiteKind::TreeColumnar7 => "Columnar tree 7 m",
            SiteKind::TreeOrnamental4 => "Ornamental tree 4 m",
            SiteKind::ShrubRound => "Round shrub",
            SiteKind::ShrubSmall => "Small shrub",
            SiteKind::Hedge => "Hedge run",
            SiteKind::PottedPlant => "Potted plant",
            SiteKind::Fence => "Fence run",
            SiteKind::Grass => "Grass lawn",
            SiteKind::FlowerBed => "Flower bed",
            SiteKind::Path => "Garden path",
            SiteKind::Driveway => "Driveway",
        }
    }

    /// Palette tag for the Library dock (Entourage / Site).
    pub fn group(self) -> &'static str {
        match self {
            SiteKind::Fence | SiteKind::Grass | SiteKind::FlowerBed | SiteKind::Path | SiteKind::Driveway => "Site",
            _ => "Entourage",
        }
    }

    pub fn shape(self) -> SiteShape {
        match self {
            SiteKind::Fence => SiteShape::Chain,
            SiteKind::Grass | SiteKind::FlowerBed | SiteKind::Path | SiteKind::Driveway => SiteShape::Polygon,
            _ => SiteShape::Point,
        }
    }

    /// Whether the kind lives on the level floor (all of them do today; the
    /// level decides the world z).
    pub fn is_point(self) -> bool {
        self.shape() == SiteShape::Point
    }
}

/// Entourage colors (library CC0 palette): (trunk/pot, foliage/panel, accent).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SiteColors {
    pub trunk: u32,
    pub leaf: u32,
    pub accent: u32,
}

impl SiteKind {
    pub fn colors(self) -> SiteColors {
        match self {
            SiteKind::TreeDeciduous6 => SiteColors { trunk: 0x6b_4a_2f, leaf: 0x4f_7a_3a, accent: 0x5d_8a_3f },
            SiteKind::TreeDeciduous10 => SiteColors { trunk: 0x6b_4a_2f, leaf: 0x5d_8a_3f, accent: 0x3f_6b_35 },
            SiteKind::TreeConifer8 => SiteColors { trunk: 0x5a_3d_28, leaf: 0x2f_5a_3a, accent: 0x3d_6a_3a },
            SiteKind::TreeColumnar7 => SiteColors { trunk: 0x5a_3d_28, leaf: 0x3d_62_33, accent: 0x4f_7a_3a },
            SiteKind::TreeOrnamental4 => SiteColors { trunk: 0x6b_4a_2f, leaf: 0x6f_8f_3c, accent: 0x7f_9f_4a },
            SiteKind::ShrubRound => SiteColors { trunk: 0x3f_6b_35, leaf: 0x3f_6b_35, accent: 0x4f_7a_3a },
            SiteKind::ShrubSmall => SiteColors { trunk: 0x5d_8a_3f, leaf: 0x5d_8a_3f, accent: 0x6f_8f_3c },
            SiteKind::Hedge => SiteColors { trunk: 0x46_70_3a, leaf: 0x46_70_3a, accent: 0x3d_6a_3a },
            SiteKind::PottedPlant => SiteColors { trunk: 0xb0_60_3e, leaf: 0x4f_7a_3a, accent: 0x5d_8a_3f },
            SiteKind::Fence => SiteColors { trunk: 0x8a_6a_48, leaf: 0x8a_6a_48, accent: 0x6e_54_38 },
            SiteKind::Grass => SiteColors { trunk: 0x5d_8a_3f, leaf: 0x5d_8a_3f, accent: 0x4f_7a_3a },
            SiteKind::FlowerBed => SiteColors { trunk: 0x6b_4a_2f, leaf: 0x5d_8a_3f, accent: 0xc8_5a_78 },
            SiteKind::Path => SiteColors { trunk: 0xb8_b0_a2, leaf: 0xb8_b0_a2, accent: 0xa8_a0_92 },
            SiteKind::Driveway => SiteColors { trunk: 0x8e_8c_88, leaf: 0x8e_8c_88, accent: 0x7c_7a_76 },
        }
    }
}

/// Plan-symbol radius (meters) for point kinds: canopy circle for trees,
/// pot/shrub circle for shrubs. `None` for linear kinds.
pub fn plan_radius(kind: SiteKind) -> Option<f64> {
    Some(match kind {
        SiteKind::TreeDeciduous6 => 2.43,
        SiteKind::TreeDeciduous10 => 3.9,
        SiteKind::TreeConifer8 => 1.6,
        SiteKind::TreeColumnar7 => 0.8,
        SiteKind::TreeOrnamental4 => 1.55,
        SiteKind::ShrubRound => 0.6,
        SiteKind::ShrubSmall => 0.3,
        SiteKind::PottedPlant => 0.33,
        _ => return None,
    })
}

/// (width, depth, height) in meters for point/box kinds.
pub fn preset_size(kind: SiteKind) -> (f64, f64, f64) {
    match kind {
        SiteKind::TreeDeciduous6 => (4.7, 4.7, 6.0),
        SiteKind::TreeDeciduous10 => (7.8, 7.8, 10.0),
        SiteKind::TreeConifer8 => (3.2, 3.2, 8.0),
        SiteKind::TreeColumnar7 => (1.6, 1.6, 7.0),
        SiteKind::TreeOrnamental4 => (3.1, 3.1, 4.0),
        SiteKind::ShrubRound => (1.2, 1.2, 1.0),
        SiteKind::ShrubSmall => (0.6, 0.6, 0.55),
        SiteKind::Hedge => (2.0, 0.6, 1.2),
        SiteKind::PottedPlant => (0.66, 0.66, 1.1),
        SiteKind::Fence => (2.0, 0.1, 1.8),
        SiteKind::Grass => (2.0, 2.0, 0.05),
        SiteKind::FlowerBed => (2.0, 1.0, 0.3),
        SiteKind::Path => (1.2, 1.2, 0.06),
        SiteKind::Driveway => (3.0, 5.0, 0.08),
    }
}

/// One site element. Point kinds use `pos` only; chains/polygons use `pts`
/// (world plan meters). `variant` seeds the pseudo-random canopy shape so
/// two trees of one kind don't render as clones.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SiteElement {
    pub id: Id,
    pub kind: SiteKind,
    pub pos: Vec2,
    #[serde(default)]
    pub pts: Vec<Vec2>,
    /// Plan rotation (radians) for point kinds.
    #[serde(default)]
    pub rotation: f64,
    #[serde(default)]
    pub variant: u32,
}

impl SiteElement {
    /// Length of a chain run (fence).
    pub fn chain_length(&self) -> f64 {
        self.pts.windows(2).map(|w| (w[1] - w[0]).length()).sum()
    }

    /// Polygon area (grass, beds, paths).
    pub fn area(&self) -> f64 {
        bimcraft_geom::poly_area(&self.pts).abs()
    }

    /// World bounding circle center + radius (plan picking + bounds).
    pub fn plan_bounds(&self) -> (Vec2, f64) {
        let (w, d, _) = preset_size(self.kind);
        let r = (w.max(d) / 2.0).max(0.3);
        if self.pts.is_empty() {
            (self.pos, r)
        } else {
            let mut min = Vec2::new(f64::INFINITY, f64::INFINITY);
            let mut max = Vec2::new(f64::NEG_INFINITY, f64::NEG_INFINITY);
            for p in &self.pts {
                min.x = min.x.min(p.x);
                min.y = min.y.min(p.y);
                max.x = max.x.max(p.x);
                max.y = max.y.max(p.y);
            }
            let c = Vec2::new((min.x + max.x) / 2.0, (min.y + max.y) / 2.0);
            (c, r.max((max - min).length() / 2.0))
        }
    }

    /// Fence panels: 2.0 m runs along the chain with a post at each break.
    /// Returns (start, end) pairs in plan space.
    pub fn fence_runs(&self) -> Vec<(Vec2, Vec2)> {
        let (panel_w, _, _) = preset_size(SiteKind::Fence);
        let mut out = Vec::new();
        for (a, b) in self.pts.windows(2).map(|w| (w[0], w[1])) {
            let len = (b - a).length();
            if len < 1e-6 {
                continue;
            }
            let dir = (b - a) / len;
            let n = ((len / panel_w).ceil() as usize).max(1);
            let step = len / n as f64;
            for i in 0..n {
                let p0 = a + dir * (i as f64 * step);
                let p1 = a + dir * ((i + 1) as f64 * step);
                out.push((p0, p1));
            }
        }
        out
    }
}

/// Fence panel pitch (meters) — used by the renderer.
pub const FENCE_PANEL_W: f64 = 2.0;
/// Fence height (meters).
pub const FENCE_H: f64 = 1.8;
/// Flower bed height (meters, low brick edging + soil).
pub const FLOWER_BED_H: f64 = 0.3;

/// A few believable flower dot colors (rose, tulip, daisy, lavender).
pub const FLOWER_COLORS: [u32; 4] = [0xc8_5a_78, 0xd8_6a_4a, 0xd8_c8_5a, 0x9a_7a_c8];
/// Grass blade tone variation.
pub const GRASS_TONES: [u32; 3] = [0x5d_8a_3f, 0x4f_7a_3a, 0x6f_8f_3c];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds_report_shapes() {
        assert_eq!(SiteKind::TreeDeciduous6.shape(), SiteShape::Point);
        assert_eq!(SiteKind::Fence.shape(), SiteShape::Chain);
        assert_eq!(SiteKind::Grass.shape(), SiteShape::Polygon);
        assert_eq!(SiteKind::Hedge.shape(), SiteShape::Point);
    }

    #[test]
    fn fence_runs_split_evenly() {
        let e = SiteElement {
            id: 1,
            kind: SiteKind::Fence,
            pos: Vec2::ZERO,
            pts: vec![Vec2::new(0.0, 0.0), Vec2::new(7.0, 0.0)],
            rotation: 0.0,
            variant: 0,
        };
        let runs = e.fence_runs();
        assert_eq!(runs.len(), 4, "7 m → 4 panels of 1.75 m");
        assert!((runs[0].1 - runs[0].0).length() - 7.0 / 4.0 < 1e-9);
    }

    #[test]
    fn plan_bounds_cover_polygon() {
        let e = SiteElement {
            id: 2,
            kind: SiteKind::Grass,
            pos: Vec2::ZERO,
            pts: vec![Vec2::new(0.0, 0.0), Vec2::new(10.0, 0.0), Vec2::new(10.0, 6.0), Vec2::new(0.0, 6.0)],
            rotation: 0.0,
            variant: 0,
        };
        let (c, r) = e.plan_bounds();
        assert!((c.x - 5.0).abs() < 1e-9);
        assert!(r >= 3.0);
    }
}
