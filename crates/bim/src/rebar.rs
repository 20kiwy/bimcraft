//! Reinforcement generation for foundations and wall corners (clean-room,
//! following the DetailCraft rebar engine's shape: a bar table, even
//! layouts, hook lengths, cut lengths, checks and a bending schedule).
//!
//! Nothing generated is stored: bars are re-derived from the current
//! foundation geometry and walls on every build, so moving a wall re-bars it.
//! The stored seed is only [`RebarSpec`] — bar size, spacing, cover, hooks,
//! starters — hanging off the foundation / model.

use bimcraft_geom::Vec2;
use serde::{Deserialize, Serialize};

use crate::foundation::{FoundationPart, FoundationRole, ResolvedSizes};
use crate::Wall;

/// One reinforcing bar size (ASTM A615 #3–#11, soft-metric diameters).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BarSize {
    pub label: &'static str,
    /// Nominal diameter (meters).
    pub db: f64,
    /// Nominal mass (kg/m).
    pub mass: f64,
    /// Minimum bend diameter as a multiple of `db` (6 for #3–#8, 8 after,
    /// ACI 318-19 Table 25.3.1).
    pub bend_factor: f64,
}

/// The bar table. Index into [`BARS`] is stored on specs and bars.
pub const BARS: &[BarSize] = &[
    BarSize { label: "#3", db: 0.0095, mass: 0.560, bend_factor: 6.0 },
    BarSize { label: "#4", db: 0.0127, mass: 0.994, bend_factor: 6.0 },
    BarSize { label: "#5", db: 0.0159, mass: 1.550, bend_factor: 6.0 },
    BarSize { label: "#6", db: 0.0191, mass: 2.240, bend_factor: 6.0 },
    BarSize { label: "#7", db: 0.0222, mass: 3.050, bend_factor: 6.0 },
    BarSize { label: "#8", db: 0.0254, mass: 3.982, bend_factor: 6.0 },
    BarSize { label: "#9", db: 0.0287, mass: 5.060, bend_factor: 8.0 },
    BarSize { label: "#10", db: 0.0323, mass: 6.418, bend_factor: 8.0 },
    BarSize { label: "#11", db: 0.0358, mass: 7.900, bend_factor: 8.0 },
];

pub fn bar(i: usize) -> &'static BarSize {
    BARS.get(i).unwrap_or(&BARS[1])
}

/// Bar shape codes (DetailCraft codes; S maps to BS 8666 code 00, L to 11,
/// U to 21 / ACI 315 Type 2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum RebarShape {
    /// Straight bar.
    S,
    /// One 90° bend (L bar, e.g. starters).
    L,
    /// Two 90° bends (U / hooked bar).
    U,
}

impl RebarShape {
    pub fn code(self) -> &'static str {
        match self {
            RebarShape::S => "00",
            RebarShape::L => "11",
            RebarShape::U => "21",
        }
    }
}

/// What a generated bar does (for schedule grouping and picking).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum RebarRole {
    /// Transverse bars across a strip footing width.
    BottomTransverse,
    /// Longitudinal bars running with the footing.
    BottomLongitudinal,
    /// Vertical starter bars tying footing to wall / wall corner to wall.
    Starter,
}

impl RebarRole {
    pub fn label(self) -> &'static str {
        match self {
            RebarRole::BottomTransverse => "Footing transverse",
            RebarRole::BottomLongitudinal => "Footing longitudinal",
            RebarRole::Starter => "Starter",
        }
    }
}

/// Stored rebar intent for one host (foundation or wall corner set).
/// Every field is a designer choice; geometry is derived.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct RebarSpec {
    /// Index into [`BARS`].
    pub bar: usize,
    /// Bar spacing along the run (meters).
    pub spacing: f64,
    /// Concrete cover to the bar surface (meters).
    pub cover: f64,
    /// Bend hooked ends (S → U) where the run allows it.
    pub hooks: bool,
    /// Generate starter bars up the stem / at wall corners.
    pub starters: bool,
}

impl Default for RebarSpec {
    fn default() -> Self {
        RebarSpec { bar: 3, spacing: 0.30, cover: 0.075, hooks: false, starters: true }
    }
}

impl RebarSpec {
    pub fn effective_spacing(&self) -> f64 {
        if self.spacing < 0.05 {
            0.30
        } else {
            self.spacing
        }
    }
}

/// One generated bar: centerline polyline (world z), shape, size, role and
/// the cut length derived from it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RebarBar {
    pub pts: Vec<Vec2>,
    pub z: f64,
    /// Optional vertical tail (starters rise): bar top z when > `z`.
    pub z_top: f64,
    pub shape: RebarShape,
    pub bar: usize,
    pub role: RebarRole,
}

impl RebarBar {
    /// Polyline length in 3D (plan legs + the vertical rise when present).
    pub fn cut_length(&self) -> f64 {
        let mut len = 0.0;
        for w in self.pts.windows(2) {
            len += (w[1] - w[0]).length();
        }
        if self.z_top > self.z {
            len += self.z_top - self.z;
        }
        // Centerline bend deduction (BS 8666: A + B − 0.5r − d per 90° bend).
        let b = bar(self.bar);
        let r = b.bend_factor * b.db;
        let bends = match self.shape {
            RebarShape::S => 0,
            RebarShape::L => 1,
            RebarShape::U => 2,
        };
        (len - bends as f64 * (0.5 * r + b.db)).max(0.0)
    }

    pub fn mass(&self) -> f64 {
        self.cut_length() * bar(self.bar).mass
    }
}

/// Even distribution along a run of width `w` at spacing `s` (DetailCraft
/// `layout`): `n = ceil(w/s) + 1` (min 2), spread evenly from cover to cover.
pub fn layout(w: f64, s: f64) -> (usize, f64) {
    if w <= 1e-6 {
        return (0, 0.0);
    }
    let n = (((w / s).ceil() as usize) + 1).max(2);
    (n, w / (n - 1) as f64)
}

/// Hook leg: development-length style hook = D/2 + db + 12·db (ACI 318),
/// clamped to the room available by the caller.
pub fn hook_leg(i: usize) -> f64 {
    let b = bar(i);
    let d = b.bend_factor * b.db;
    d / 2.0 + b.db + 12.0 * b.db
}

/// Generated reinforcement for one foundation: bars + checks.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RebarSet {
    pub bars: Vec<RebarBar>,
    /// Hard refusals (no bars generated where asked).
    pub problems: Vec<String>,
    /// Advisory checks (spacings below the usual minimum).
    pub warnings: Vec<String>,
}

impl RebarSet {
    pub fn total_mass(&self) -> f64 {
        self.bars.iter().map(|b| b.mass()).sum()
    }

    pub fn total_length(&self) -> f64 {
        self.bars.iter().map(|b| b.cut_length()).sum()
    }
}

/// Bottom mat + starters for a foundation whose parts come from
/// [`crate::foundation::foundation_geometry`] (world z via `floor_z`).
///
/// The strip footing ring gets transverse bars across its width on every
/// side (spaced [`RebarSpec::effective_spacing`]) plus two longitudinal
/// bars per side running with the wall; both ways, like a real footing mat.
/// Starters rise from the footing up the stem with an L leg into the footing
/// when [`RebarSpec::starters`] is set.
pub fn footing_rebar(parts: &[FoundationPart], p: &ResolvedSizes, spec: &RebarSpec, floor_z: f64) -> RebarSet {
    let mut out = RebarSet::default();
    let Some(foot) = parts.iter().find(|f| f.role == FoundationRole::StripFooting) else {
        // Slab-on-grade: a thin mat under the slab, both ways.
        if p.kind == crate::foundation::FoundationKind::Slab && spec.spacing > 0.0 {
            add_slab_mat(&mut out, &parts.iter().find(|f| f.role == FoundationRole::Slab).cloned().unwrap_or(crate::foundation::FoundationPart {
                role: FoundationRole::Slab, outer: Vec::new(), inner: None, z0: 0.0, z1: 0.0,
            }), spec, floor_z);
        } else {
            out.problems.push("No footing to reinforce (slab kinds take a mat when present)".into());
        }
        return out;
    };
    let b = bar(spec.bar);
    let z_bot = floor_z + foot.z0 + spec.cover + b.db / 2.0;
    let z_top_bar = z_bot + b.db; // longitudinal above transverse
    let ring = |out: &mut RebarSet, outer: &[Vec2], inner: &[Vec2]| {
        let n = outer.len().min(inner.len());
        // Transverse bars: across the footing width, spaced along each side.
        for i in 0..n {
            let a0 = outer[i];
            let a1 = outer[(i + 1) % n];
            let b0 = inner[i];
            let b1 = inner[(i + 1) % n];
            let edge_len = (a1 - a0).length();
            if edge_len < 0.05 {
                continue;
            }
            let (count, _step) = layout(edge_len, spec.effective_spacing());
            for k in 0..count {
                let t = if count > 1 { k as f64 / (count - 1) as f64 } else { 0.5 };
                // Keep ends clear of the corner: inset by cover.
                let inset = spec.cover.clamp(0.05, 0.45);
                let t = inset + t * (1.0 - 2.0 * inset);
                let p_out = a0 + (a1 - a0) * t;
                let p_in = b0 + (b1 - b0) * t;
                let (hook0, hook1, shape) = if spec.hooks {
                    let leg = hook_leg(spec.bar).min((p_in - p_out).length() * 0.45);
                    let d = (p_in - p_out).normalize() * leg;
                    (p_out + d, p_in - d, RebarShape::U)
                } else {
                    (p_out, p_in, RebarShape::S)
                };
                out.bars.push(RebarBar {
                    pts: vec![hook0, hook1],
                    z: z_bot,
                    z_top: z_bot,
                    shape,
                    bar: spec.bar,
                    role: RebarRole::BottomTransverse,
                });
            }
            // Two longitudinal bars per side (quarter points of the width).
            for q in [0.25, 0.75] {
                let q0 = a0 + (b0 - a0) * q;
                let q1 = a1 + (b1 - a1) * q;
                // Lap past the corner a little so the ring is continuous.
                let dir = (q1 - q0).normalize();
                let lap = (0.4_f64).min(edge_len * 0.3);
                out.bars.push(RebarBar {
                    pts: vec![q0 - dir * lap, q1 + dir * lap],
                    z: z_top_bar,
                    z_top: z_top_bar,
                    shape: RebarShape::S,
                    bar: spec.bar,
                    role: RebarRole::BottomLongitudinal,
                });
            }
        }
    };
    ring(&mut out, &foot.outer, foot.inner.as_deref().unwrap_or(&foot.outer));

    if spec.starters {
        if let Some(stem) = parts.iter().find(|f| f.role == FoundationRole::Stem) {
            let stem_top = floor_z + stem.z1;
            let spacing = (spec.effective_spacing() * 2.0).max(0.4);
            let n = stem.outer.len().min(stem.inner.as_deref().map(|i| i.len()).unwrap_or(stem.outer.len()));
            for i in 0..n {
                let a0 = stem.outer[i];
                let a1 = stem.outer[(i + 1) % n];
                let b0 = stem.inner.as_deref().map(|inn| inn[i]).unwrap_or(a0);
                let b1 = stem.inner.as_deref().map(|inn| inn[(i + 1) % n]).unwrap_or(a1);
                let edge_len = (a1 - a0).length();
                if edge_len < 0.1 {
                    continue;
                }
                let (count, _step) = layout(edge_len, spacing);
                for k in 0..count {
                    let t = if count > 1 { k as f64 / (count - 1) as f64 } else { 0.5 };
                    let mid_out = a0 + (a1 - a0) * t;
                    let mid_in = b0 + (b1 - b0) * t;
                    let center = (mid_out + mid_in) / 2.0;
                    let dir = (mid_out - mid_in).normalize();
                    let leg = hook_leg(spec.bar).min(p.stem_t / 2.0 + 0.15);
                    out.bars.push(RebarBar {
                        // Vertical bar with an L leg bent into the footing.
                        pts: vec![center - dir * leg, center],
                        z: floor_z + foot.z1 - spec.cover,
                        z_top: stem_top + 0.40,
                        shape: RebarShape::L,
                        bar: spec.bar,
                        role: RebarRole::Starter,
                    });
                }
            }
        }
    }

    // Checks (advisory, mirroring DetailCraft's footingProblems spirit).
    let clear = spec.effective_spacing() - b.db;
    let min_clear = 0.0254_f64.max(b.db);
    if clear < min_clear {
        out.warnings.push(format!(
            "Clear spacing {:.0} mm < min {:.0} mm (ACI 318-19 25.2.1)",
            clear * 1000.0,
            min_clear * 1000.0
        ));
    }
    if (foot.z1 - foot.z0) < spec.cover * 2.0 + b.db * 3.0 {
        out.warnings.push("Footing too shallow for two bar layers plus cover".into());
    }
    out
}

/// Both-ways mat under a slab-on-grade (thin simplified mat: bars along the
/// slab polygon's bounding box, kept only where both ends sit inside the
/// polygon with cover to the edge).
fn add_slab_mat(out: &mut RebarSet, slab: &FoundationPart, spec: &RebarSpec, floor_z: f64) {
    use crate::rooms::in_poly;
    if slab.outer.len() < 3 {
        out.problems.push("Slab outline missing for the mat".into());
        return;
    }
    let b = bar(spec.bar);
    let z = floor_z + slab.z0 + spec.cover + b.db / 2.0;
    let mut min = Vec2::new(f64::INFINITY, f64::INFINITY);
    let mut max = Vec2::new(f64::NEG_INFINITY, f64::NEG_INFINITY);
    for p in &slab.outer {
        min.x = min.x.min(p.x);
        min.y = min.y.min(p.y);
        max.x = max.x.max(p.x);
        max.y = max.y.max(p.y);
    }
    let cover = spec.cover.max(0.05);
    min = Vec2::new(min.x + cover, min.y + cover);
    max = Vec2::new(max.x - cover, max.y - cover);
    // Bars running along X, spaced across Y (transverse).
    let (nx, step_y) = layout(max.y - min.y, spec.effective_spacing());
    for k in 0..nx {
        let y = min.y + k as f64 * step_y;
        let (a, bb) = (Vec2::new(min.x, y), Vec2::new(max.x, y));
        if in_poly(a.x, a.y, &slab.outer) && in_poly(bb.x, bb.y, &slab.outer) {
            out.bars.push(RebarBar { pts: vec![a, bb], z, z_top: z, shape: RebarShape::S, bar: spec.bar, role: RebarRole::BottomTransverse });
        }
    }
    // Bars running along Y, spaced across X (longitudinal).
    let (ny, step_x) = layout(max.x - min.x, spec.effective_spacing());
    for k in 0..ny {
        let x = min.x + k as f64 * step_x;
        let (a, bb) = (Vec2::new(x, min.y), Vec2::new(x, max.y));
        if in_poly(a.x, a.y, &slab.outer) && in_poly(bb.x, bb.y, &slab.outer) {
            out.bars.push(RebarBar { pts: vec![a, bb], z, z_top: z, shape: RebarShape::S, bar: spec.bar, role: RebarRole::BottomLongitudinal });
        }
    }
}

/// L-shaped starter bars at every corner of the given walls (each polyline
/// vertex where two wall segments meet or two wall ends touch): a vertical
/// bar per corner with a horizontal leg bent into each adjoining wall.
pub fn corner_starters(walls: &[&Wall], z0: f64, z1: f64, spec: &RebarSpec) -> Vec<RebarBar> {
    let mut out = Vec::new();
    if z1 - z0 < 0.2 {
        return out;
    }
    let leg = hook_leg(spec.bar).clamp(0.30, 0.60);
    for w in walls {
        for (i, v) in w.pts.iter().enumerate() {
            // Corner = an interior vertex of the polyline (a real wall corner).
            if i == 0 || i + 1 >= w.pts.len() {
                continue;
            }
            let v = *v;
            let prev = (v - w.pts[i - 1]).normalize();
            let next = (w.pts[i + 1] - v).normalize();
            if prev.dot(next) > 0.999 {
                continue; // straight-through point, not a corner
            }
            let off = prev.perp() * (w.thickness / 4.0) + next.perp() * (w.thickness / 4.0);
            let c = v + off * 0.5;
            out.push(RebarBar {
                pts: vec![c - prev * leg, c, c + next * leg],
                z: z0 + 0.05,
                z_top: z1 - 0.05,
                shape: RebarShape::L,
                bar: spec.bar,
                role: RebarRole::Starter,
            });
        }
    }
    out
}

/// One bending-schedule row: identical bars (size + shape + cut to the mm)
/// share a mark (DetailCraft `"<size>F<n>"`).
#[derive(Clone, Debug, PartialEq)]
pub struct RebarRow {
    pub mark: String,
    pub size: String,
    pub shape_code: &'static str,
    pub count: usize,
    pub cut_length: f64,
    pub total_length: f64,
    pub total_mass: f64,
    pub role: String,
}

/// Group bars into a bending schedule, ordered by role then size.
pub fn bending_schedule(bars: &[RebarBar]) -> Vec<RebarRow> {
    use std::collections::BTreeMap;
    let mut groups: BTreeMap<(String, usize, RebarShape, u64, u8), (usize, RebarRole)> = BTreeMap::new();
    for b in bars {
        let cut_mm = (b.cut_length() * 1000.0).round() as u64;
        let key = (bar(b.bar).label.to_string(), b.bar, b.shape, cut_mm, b.role as u8);
        groups.entry(key).and_modify(|(n, _)| *n += 1).or_insert((1, b.role));
    }
    let mut rows: Vec<RebarRow> = groups
        .into_iter()
        .map(|((label, bar_i, shape, cut_mm, _role_u8), (count, role))| {
            let cut = cut_mm as f64 / 1000.0;
            let b = bar(bar_i);
            RebarRow {
                mark: format!("{label}F0"),
                size: label,
                shape_code: shape.code(),
                count,
                cut_length: cut,
                total_length: cut * count as f64,
                total_mass: cut * count as f64 * b.mass,
                role: role.label().into(),
            }
        })
        .collect();
    rows.sort_by(|a, b2| a.role.cmp(&b2.role).then(a.size.cmp(&b2.size)));
    // Re-mark sequentially after sorting.
    for (i, r) in rows.iter_mut().enumerate() {
        r.mark = format!("{}F{}", r.size, i + 1);
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::foundation::{foundation_geometry, foundation_params, FoundationKind, FoundationSizes};

    fn spec() -> RebarSpec {
        RebarSpec { bar: 3, spacing: 0.3, cover: 0.075, hooks: false, starters: true }
    }

    #[test]
    fn layout_spreads_evenly() {
        let (n, step) = layout(1.5, 0.3);
        assert_eq!(n, 6);
        assert!((step - 0.3).abs() < 1e-9);
        let (n2, step2) = layout(1.0, 0.3);
        assert_eq!(n2, 5);
        assert!((step2 - 0.25).abs() < 1e-9);
    }

    #[test]
    fn strip_footing_gets_both_ways_and_starters() {
        let outline = crate::Model::rect_poly(Vec2::new(0.0, 0.0), Vec2::new(8.0, 6.0));
        let p = foundation_params(FoundationKind::StemSlab, &FoundationSizes::default(), 2, false);
        let parts = foundation_geometry(&outline, &p);
        let set = footing_rebar(&parts, &p, &spec(), 0.0);
        assert!(!set.problems.is_empty() || set.bars.len() > 10);
        assert!(set.bars.iter().any(|b| b.role == RebarRole::BottomTransverse));
        assert!(set.bars.iter().any(|b| b.role == RebarRole::BottomLongitudinal));
        assert!(set.bars.iter().any(|b| b.role == RebarRole::Starter));
        assert!(set.total_mass() > 0.0);
    }

    #[test]
    fn hooks_turn_transverse_into_u() {
        let outline = crate::Model::rect_poly(Vec2::new(0.0, 0.0), Vec2::new(8.0, 6.0));
        let p = foundation_params(FoundationKind::StemSlab, &FoundationSizes::default(), 2, false);
        let parts = foundation_geometry(&outline, &p);
        let mut s = spec();
        s.hooks = true;
        let set = footing_rebar(&parts, &p, &s, 0.0);
        assert!(set.bars.iter().any(|b| b.role == RebarRole::BottomTransverse && b.shape == RebarShape::U));
    }

    #[test]
    fn slab_mat_fills_both_ways() {
        let outline = crate::Model::rect_poly(Vec2::new(0.0, 0.0), Vec2::new(6.0, 5.0));
        let p = foundation_params(FoundationKind::Slab, &FoundationSizes::default(), 1, false);
        let parts = foundation_geometry(&outline, &p);
        let set = footing_rebar(&parts, &p, &spec(), 0.0);
        assert!(!set.problems.is_empty() || set.bars.iter().any(|b| b.role == RebarRole::BottomTransverse));
        assert!(set.bars.iter().any(|b| b.role == RebarRole::BottomLongitudinal));
    }

    #[test]
    fn corner_starters_at_wall_vertices_only() {
        let wall = crate::Wall { level: 1, pts: vec![Vec2::new(0.0, 0.0), Vec2::new(5.0, 0.0), Vec2::new(5.0, 4.0)], thickness: 0.2, height: 3.0, ..Wall::default() };
        let walls = vec![&wall];
        let bars = corner_starters(&walls, 0.0, 3.0, &spec());
        assert_eq!(bars.len(), 1, "one interior corner");
        assert_eq!(bars[0].pts.len(), 3, "L legs");
        assert!(bars[0].cut_length() > 0.0);
    }

    #[test]
    fn schedule_groups_and_marks() {
        let bars = vec![
            RebarBar { pts: vec![Vec2::ZERO, Vec2::new(2.0, 0.0)], z: 0.0, z_top: 0.0, shape: RebarShape::S, bar: 1, role: RebarRole::BottomTransverse },
            RebarBar { pts: vec![Vec2::ZERO, Vec2::new(2.0, 0.0)], z: 0.0, z_top: 0.0, shape: RebarShape::S, bar: 1, role: RebarRole::BottomTransverse },
            RebarBar { pts: vec![Vec2::ZERO, Vec2::new(2.0, 0.0)], z: 0.0, z_top: 0.5, shape: RebarShape::L, bar: 1, role: RebarRole::Starter },
        ];
        let rows = bending_schedule(&bars);
        assert_eq!(rows.len(), 2);
        assert!(rows.iter().all(|r| r.mark.ends_with("F1") || r.mark.ends_with("F2")));
        assert_eq!(rows.iter().find(|r| r.shape_code == "00").unwrap().count, 2);
    }
}
