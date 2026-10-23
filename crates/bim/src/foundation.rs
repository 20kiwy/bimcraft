//! Foundations generated from the wall outlines (parity with the JS edition's
//! `res-foundation.js`, clean-room): slab-on-grade with a thickened edge, stem
//! walls on strip footings with a slab, a crawlspace, and a basement.
//!
//! A foundation is stored as a seed point plus a kind and optional size
//! overrides (0 = automatic); the outline it serves is re-derived from the
//! current walls, so the geometry follows wall moves without any sync step.
//! Footing sizes follow the usual light-frame table for 1500 psf soil —
//! labelled defaults for the designer, not engineering.
//!
//! Heights are measured from the level floor (z = 0): a slab-on-grade's top is
//! the floor; a crawlspace's stem walls stop under the floor framing
//! (`stem_drop` below it); a basement's walls run from its footing up to there
//! and its slab sits `basement_h` below that.

use bimcraft_geom::{poly_area, Vec2};
use serde::{Deserialize, Serialize};

use crate::Id;

/// The four foundation kinds (parity: `FOUNDATION_KINDS`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum FoundationKind {
    /// Slab-on-grade with a thickened edge under the perimeter.
    #[default]
    Slab,
    /// Stem walls on strip footings plus an interior slab.
    StemSlab,
    /// Stem walls on footings, no slab (crawlspace).
    Crawl,
    /// Full-height basement walls, footings, and a slab.
    Basement,
}

impl FoundationKind {
    pub const ALL: [FoundationKind; 4] = [FoundationKind::Slab, FoundationKind::StemSlab, FoundationKind::Crawl, FoundationKind::Basement];

    pub fn label(self) -> &'static str {
        match self {
            FoundationKind::Slab => "Slab on grade",
            FoundationKind::StemSlab => "Stem walls + slab",
            FoundationKind::Crawl => "Crawlspace",
            FoundationKind::Basement => "Basement",
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            FoundationKind::Slab => "slab",
            FoundationKind::StemSlab => "stemslab",
            FoundationKind::Crawl => "crawl",
            FoundationKind::Basement => "basement",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.key() == key)
    }

    /// Whether this kind places an interior slab.
    pub fn has_slab(self) -> bool {
        matches!(self, FoundationKind::Slab | FoundationKind::StemSlab | FoundationKind::Basement)
    }

    /// Whether stem walls are part of this kind.
    pub fn has_stem(self) -> bool {
        !matches!(self, FoundationKind::Slab)
    }
}

/// Stored sizes (meters). Every 0.0 falls back to the kind's automatic value.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct FoundationSizes {
    pub slab_t: f64,
    pub edge_w: f64,
    pub edge_d: f64,
    pub stem_t: f64,
    pub foot_w: f64,
    pub foot_d: f64,
    /// Footing underside below the floor (crawlspace / basement depth).
    pub bottom: f64,
    /// How far below the floor the stem / basement walls stop (floor framing depth).
    pub stem_drop: f64,
    pub basement_h: f64,
}

/// A foundation element: kind + seed point inside a building outline + sizes.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Foundation {
    pub id: Id,
    pub level: Id,
    pub kind: FoundationKind,
    /// A point inside the building outline this foundation serves.
    pub seed: Vec2,
    #[serde(default)]
    pub sizes: FoundationSizes,
    #[serde(default)]
    pub name: String,
    /// Reinforcement intent (bar size, spacing, cover, hooks, starters);
    /// the bars themselves are derived (see `rebar::footing_rebar`).
    #[serde(default)]
    pub rebar: crate::rebar::RebarSpec,
}

/// One resolved size set (every field is the final value in meters).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ResolvedSizes {
    pub kind: FoundationKind,
    pub slab_t: f64,
    pub edge_w: f64,
    pub edge_d: f64,
    pub stem_t: f64,
    pub foot_w: f64,
    pub foot_d: f64,
    pub bottom: f64,
    pub stem_drop: f64,
    pub basement_h: f64,
}

impl ResolvedSizes {
    /// Height of the stem / basement wall top relative to the floor (negative
    /// when the walls stop under the floor framing).
    pub fn stem_top(&self) -> f64 {
        -self.stem_drop
    }
}

/// Strip footing width and depth after the usual light-frame table for 1500 psf
/// soil: 12" / 15" / 23" wide for 1 / 2 / 3 stories (18" / 22" / 27" under a
/// crawlspace or basement), 4" wider under masonry or concrete walls, 8" deep
/// (10" from 24" wide). A designer or engineer checks them for the soil.
pub fn footing_size(kind: FoundationKind, stories: u32, heavy: bool) -> (f64, f64) {
    const IN: f64 = 0.0254;
    let n = stories.clamp(1, 3) as usize - 1;
    let base = if matches!(kind, FoundationKind::Crawl | FoundationKind::Basement) { [18.0, 22.0, 27.0][n] } else { [12.0, 15.0, 23.0][n] }
        + if heavy { 4.0 } else { 0.0 };
    (base * IN, if base >= 24.0 { 10.0 * IN } else { 8.0 * IN })
}

/// Every size a foundation of `kind` needs: stored values where set (> 0), else
/// the kind's defaults after the wall loads (`stories`, `heavy`).
pub fn foundation_params(kind: FoundationKind, stored: &FoundationSizes, stories: u32, heavy: bool) -> ResolvedSizes {
    let (foot_w, foot_d) = footing_size(kind, stories, heavy);
    let d = ResolvedSizes {
        kind,
        slab_t: 0.1016,
        edge_w: 0.3048,
        edge_d: 0.3048,
        stem_t: if heavy { 0.254 } else { 0.2032 },
        foot_w,
        foot_d,
        bottom: if kind == FoundationKind::Crawl { 0.9144 } else { 0.6096 },
        stem_drop: if matches!(kind, FoundationKind::Crawl | FoundationKind::Basement) { 0.3048 } else { 0.0 },
        basement_h: 2.4384,
    };
    let pick = |stored_v: f64, auto: f64| if stored_v > 0.0 { stored_v } else { auto };
    ResolvedSizes {
        slab_t: pick(stored.slab_t, d.slab_t),
        edge_w: pick(stored.edge_w, d.edge_w),
        edge_d: pick(stored.edge_d, d.edge_d),
        stem_t: pick(stored.stem_t, d.stem_t),
        foot_w: pick(stored.foot_w, d.foot_w),
        foot_d: pick(stored.foot_d, d.foot_d),
        bottom: pick(stored.bottom, d.bottom),
        stem_drop: pick(stored.stem_drop, d.stem_drop),
        basement_h: pick(stored.basement_h, d.basement_h),
        kind,
    }
}

/// Role of one generated foundation part.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FoundationRole {
    Slab,
    /// The thickened edge under a slab-on-grade perimeter.
    Edge,
    StripFooting,
    Stem,
}

impl FoundationRole {
    pub fn label(self) -> &'static str {
        match self {
            FoundationRole::Slab => "Slab",
            FoundationRole::Edge => "Thickened edge",
            FoundationRole::StripFooting => "Strip footing",
            FoundationRole::Stem => "Stem wall",
        }
    }
}

/// One extruded ring (or solid polygon) of concrete, heights from the floor.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FoundationPart {
    pub role: FoundationRole,
    pub outer: Vec<Vec2>,
    pub inner: Option<Vec<Vec2>>,
    pub z0: f64,
    pub z1: f64,
}

impl FoundationPart {
    pub fn volume(&self) -> f64 {
        let a_out = poly_area(&self.outer).abs();
        let a_in = self.inner.as_ref().map(|i| poly_area(i).abs()).unwrap_or(0.0);
        (a_out - a_in).abs() * (self.z1 - self.z0)
    }

    /// Centerline length of a ring part (average of outer and inner perimeters).
    pub fn ring_length(&self) -> f64 {
        match &self.inner {
            Some(inner) => (ring_perimeter(&self.outer) + ring_perimeter(inner)) / 2.0,
            None => ring_perimeter(&self.outer),
        }
    }
}

fn ring_perimeter(poly: &[Vec2]) -> f64 {
    let n = poly.len();
    if n < 2 {
        return 0.0;
    }
    (0..n).map(|i| (poly[(i + 1) % n] - poly[i]).length()).sum()
}

/// Move an anticlockwise polygon inward by `d` (outward when `d < 0`), corners
/// mitred. The result keeps `poly.len()` vertices; consecutive offset edges are
/// intersected, and parallel edges are simply shifted.
pub fn inset_poly(poly: &[Vec2], d: f64) -> Vec<Vec2> {
    let n = poly.len();
    if n < 3 {
        return poly.to_vec();
    }
    let edges: Vec<(Vec2, Vec2, Vec2)> = (0..n)
        .map(|i| {
            let a = poly[i];
            let b = poly[(i + 1) % n];
            let len = (b - a).length();
            let u = if len > 1e-12 { (b - a) / len } else { Vec2::new(1.0, 0.0) };
            let normal = Vec2::new(-u.y, u.x); // inward for anticlockwise input
            (a, u, normal)
        })
        .collect();
    (0..n)
        .map(|i| {
            let (a, u, nrm) = edges[i];
            let (fa, fu, fnrm) = edges[(i + n - 1) % n];
            let p0 = fa + fnrm * d;
            let p1 = a + nrm * d;
            let cross = fu.x * u.y - fu.y * u.x;
            if cross.abs() < 1e-9 {
                return p1;
            }
            let t = ((p1.x - p0.x) * u.y - (p1.y - p0.y) * u.x) / cross;
            p0 + fu * t
        })
        .collect()
}

/// Concrete volume of generated parts (holes subtracted), m³.
pub fn foundation_concrete_volume(parts: &[FoundationPart]) -> f64 {
    parts.iter().map(|p| p.volume()).sum()
}

/// The parts of a foundation under `outline` (anticlockwise outer wall faces),
/// heights measured from the level floor.
pub fn foundation_geometry(outline: &[Vec2], p: &ResolvedSizes) -> Vec<FoundationPart> {
    let mut parts = Vec::new();
    if outline.len() < 3 {
        return parts;
    }
    let area_ok = |poly: &[Vec2]| poly_area(poly).abs() > 1e-4;
    let ring = |parts: &mut Vec<FoundationPart>, role: FoundationRole, d_out: f64, d_in: f64, z0: f64, z1: f64| {
        let outer = inset_poly(outline, d_out);
        let inner = inset_poly(outline, d_in);
        if area_ok(&inner) && z1 > z0 + 1e-9 {
            parts.push(FoundationPart { role, outer, inner: Some(inner), z0, z1 });
        }
    };
    let slab = |parts: &mut Vec<FoundationPart>, d: f64, z0: f64, z1: f64| {
        let poly = inset_poly(outline, d);
        if area_ok(&poly) && z1 > z0 + 1e-9 {
            parts.push(FoundationPart { role: FoundationRole::Slab, outer: poly, inner: None, z0, z1 });
        }
    };
    if p.kind == FoundationKind::Slab {
        slab(&mut parts, 0.0, -p.slab_t, 0.0);
        ring(&mut parts, FoundationRole::Edge, 0.0, p.edge_w, -p.edge_d, -p.slab_t);
    } else if matches!(p.kind, FoundationKind::StemSlab | FoundationKind::Crawl) {
        let foot_top = -p.bottom + p.foot_d;
        ring(&mut parts, FoundationRole::Stem, 0.0, p.stem_t, foot_top, p.stem_top());
        ring(&mut parts, FoundationRole::StripFooting, (p.stem_t - p.foot_w) / 2.0, (p.stem_t + p.foot_w) / 2.0, foot_top - p.foot_d, foot_top);
        if p.kind == FoundationKind::StemSlab {
            slab(&mut parts, p.stem_t, -p.slab_t, 0.0);
        }
    } else {
        // Basement: slab_top = stem_top - basement_h.
        let slab_top = p.stem_top() - p.basement_h;
        let foot_top = slab_top - p.slab_t;
        ring(&mut parts, FoundationRole::Stem, 0.0, p.stem_t, foot_top, p.stem_top());
        ring(&mut parts, FoundationRole::StripFooting, (p.stem_t - p.foot_w) / 2.0, (p.stem_t + p.foot_w) / 2.0, foot_top - p.foot_d, foot_top);
        slab(&mut parts, p.stem_t, slab_top - p.slab_t, slab_top);
    }
    parts
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Anticlockwise 10 × 6 m rectangle.
    fn rect() -> Vec<Vec2> {
        vec![Vec2::new(0.0, 0.0), Vec2::new(10.0, 0.0), Vec2::new(10.0, 6.0), Vec2::new(0.0, 6.0)]
    }

    #[test]
    fn inset_poly_shrinks_and_keeps_vertices() {
        let r = rect();
        let inner = inset_poly(&r, 0.2);
        assert_eq!(inner.len(), r.len());
        assert!((poly_area(&inner).abs() - 9.6 * 5.6).abs() < 1e-9);
        // First vertex: offset edges y=0.2 and x=0.2 meet at (0.2, 0.2).
        assert!((inner[0].x - 0.2).abs() < 1e-9 && (inner[0].y - 0.2).abs() < 1e-9);
        // Negative d grows the polygon.
        let outer = inset_poly(&r, -0.1);
        assert!(poly_area(&outer).abs() > poly_area(&r).abs());
    }

    #[test]
    fn inset_poly_mitres_oblique_corners() {
        // Anticlockwise triangle; the inset vertex is the intersection of the
        // two offset edges, strictly inside.
        let tri = vec![Vec2::new(0.0, 0.0), Vec2::new(6.0, 0.0), Vec2::new(0.0, 8.0)];
        let inner = inset_poly(&tri, 0.5);
        assert_eq!(inner.len(), 3);
        let a = poly_area(&tri).abs();
        let ai = poly_area(&inner).abs();
        assert!(ai < a && ai > 0.0);
        for v in &inner {
            assert!(v.x > -1e-9 && v.y > -1e-9 && v.x < 6.0 && v.y < 8.0);
        }
    }

    #[test]
    fn footing_table_matches_light_frame_defaults() {
        let (w, d) = footing_size(FoundationKind::Slab, 1, false);
        assert!((w - 0.3048).abs() < 1e-9 && (d - 0.2032).abs() < 1e-9);
        let (w, d) = footing_size(FoundationKind::Crawl, 2, true);
        assert!((w - 26.0 * 0.0254).abs() < 1e-9 && (d - 0.254).abs() < 1e-9);
        let (w, _) = footing_size(FoundationKind::Basement, 3, false);
        assert!((w - 27.0 * 0.0254).abs() < 1e-9);
    }

    #[test]
    fn stored_sizes_override_defaults() {
        let stored = FoundationSizes { slab_t: 0.2, ..FoundationSizes::default() };
        let p = foundation_params(FoundationKind::Slab, &stored, 1, false);
        assert!((p.slab_t - 0.2).abs() < 1e-9);
        assert!((p.edge_w - 0.3048).abs() < 1e-9);
        assert!((p.stem_t - 0.2032).abs() < 1e-9);
    }

    #[test]
    fn slab_on_grade_parts() {
        let p = foundation_params(FoundationKind::Slab, &FoundationSizes::default(), 1, false);
        let parts = foundation_geometry(&rect(), &p);
        let roles: Vec<FoundationRole> = parts.iter().map(|x| x.role).collect();
        assert!(roles.contains(&FoundationRole::Slab) && roles.contains(&FoundationRole::Edge));
        let slab = parts.iter().find(|x| x.role == FoundationRole::Slab).unwrap();
        assert!(slab.inner.is_none() && (slab.z0 + p.slab_t).abs() < 1e-9 && slab.z1.abs() < 1e-9);
        let edge = parts.iter().find(|x| x.role == FoundationRole::Edge).unwrap();
        // Edge runs from the wall outer face inward by edge_w.
        let inner = edge.inner.as_ref().unwrap();
        assert!((poly_area(&edge.outer).abs() - 60.0).abs() < 1e-6);
        assert!((poly_area(inner).abs() - (10.0 - 2.0 * p.edge_w) * (6.0 - 2.0 * p.edge_w)).abs() < 1e-6);
        assert!(edge.z1 < slab.z0 + 1e-9);
    }

    #[test]
    fn stemslab_crawl_and_basement_parts() {
        let r = rect();
        let stem = foundation_params(FoundationKind::StemSlab, &FoundationSizes::default(), 1, false);
        let parts = foundation_geometry(&r, &stem);
        for role in [FoundationRole::Stem, FoundationRole::StripFooting, FoundationRole::Slab] {
            assert!(parts.iter().any(|x| x.role == role), "missing {role:?}");
        }
        let footing = parts.iter().find(|x| x.role == FoundationRole::StripFooting).unwrap();
        assert!((footing.z1 - (-stem.bottom + stem.foot_d)).abs() < 1e-9);
        assert!((footing.z0 - (footing.z1 - stem.foot_d)).abs() < 1e-9);
        // Interior slab is inset by the stem thickness.
        let slab = parts.iter().find(|x| x.role == FoundationRole::Slab).unwrap();
        assert!((poly_area(&slab.outer).abs() - (10.0 - 2.0 * stem.stem_t) * (6.0 - 2.0 * stem.stem_t)).abs() < 1e-6);

        let crawl = foundation_params(FoundationKind::Crawl, &FoundationSizes::default(), 1, false);
        let parts = foundation_geometry(&r, &crawl);
        assert!(!parts.iter().any(|x| x.role == FoundationRole::Slab));
        assert!(parts.iter().any(|x| x.role == FoundationRole::Stem));

        let base = foundation_params(FoundationKind::Basement, &FoundationSizes::default(), 1, false);
        let parts = foundation_geometry(&r, &base);
        let stem_p = parts.iter().find(|x| x.role == FoundationRole::Stem).unwrap();
        assert!((stem_p.z1 - base.stem_top()).abs() < 1e-9);
        // Basement slab sits basement_h below the stem top.
        let slab = parts.iter().find(|x| x.role == FoundationRole::Slab).unwrap();
        assert!((slab.z1 - (base.stem_top() - base.basement_h)).abs() < 1e-9);
        assert!(slab.z0 < slab.z1);
    }

    #[test]
    fn concrete_volume_sums_parts() {
        let p = foundation_params(FoundationKind::StemSlab, &FoundationSizes::default(), 1, false);
        let parts = foundation_geometry(&rect(), &p);
        let manual: f64 = parts.iter().map(|x| x.volume()).sum();
        assert!(foundation_concrete_volume(&parts) > 0.0);
        assert!((foundation_concrete_volume(&parts) - manual).abs() < 1e-12);
    }

    #[test]
    fn kind_keys_roundtrip() {
        for k in FoundationKind::ALL {
            assert_eq!(FoundationKind::from_key(k.key()), Some(k));
        }
        assert_eq!(FoundationKind::from_key("nope"), None);
    }
}
