//! Brick wall finishes (clean-room, following real brick dimensions and
//! stretcher-bond coursing as masons lay it).
//!
//! A wall can carry a brick finish (`Wall::finish` naming a [`BrickType`]).
//! The renderer turns that into courses of bricks on the wall faces: rows of
//! [`BrickType::height`] plus a mortar joint, each row shifted half a brick
//! against the previous one (stretcher bond), end bricks cut to the wall
//! length. Every preset mirrors a real product so the schedule can report
//! believable counts per m².

use serde::{Deserialize, Serialize};

/// One brick product: nominal face dimensions (length × height, meters,
/// without joints), course width (the wall thickness it builds), the mortar
/// joint thickness, and the fired-clay / mortar colors (0xRRGGBB).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct BrickType {
    pub key: &'static str,
    pub name: &'static str,
    /// Brick length along the wall (meters).
    pub length: f64,
    /// Brick height, one course (meters).
    pub height: f64,
    /// Course width — the wall thickness this brick builds (meters).
    pub width: f64,
    /// Mortar joint thickness (meters).
    pub joint: f64,
    /// Brick body color (0xRRGGBB).
    pub color: u32,
    /// Mortar color (0xRRGGBB).
    pub mortar: u32,
}

impl BrickType {
    /// Course pitch (brick + head joint) and row pitch (brick + bed joint).
    pub fn pitches(&self) -> (f64, f64) {
        (self.length + self.joint, self.height + self.joint)
    }
}

/// The brick presets offered in the properties panel. Dimensions follow
/// common metric products (UK/EU face + engineering bricks, US modular).
pub const BRICK_TYPES: &[BrickType] = &[
    BrickType { key: "brick-red", name: "Red face brick", length: 0.215, height: 0.065, width: 0.1025, joint: 0.010, color: 0x9b_4f_3a, mortar: 0xb6_ad_a2 },
    BrickType { key: "brick-common", name: "Common brick", length: 0.240, height: 0.071, width: 0.115, joint: 0.012, color: 0xa8_5c_44, mortar: 0xb0_a8_9c },
    BrickType { key: "brick-engineering", name: "Engineering blue", length: 0.215, height: 0.065, width: 0.1025, joint: 0.010, color: 0x3e_4a_55, mortar: 0x9a_98_92 },
    BrickType { key: "brick-clinker", name: "Clinker", length: 0.230, height: 0.071, width: 0.110, joint: 0.012, color: 0x7a_46_30, mortar: 0xa8_9e_90 },
    BrickType { key: "brick-roman", name: "Roman (low)", length: 0.235, height: 0.040, width: 0.115, joint: 0.010, color: 0xb2_6a_4c, mortar: 0xbc_b2_a6 },
    BrickType { key: "brick-norwegian", name: "Norwegian", length: 0.228, height: 0.054, width: 0.108, joint: 0.011, color: 0xc0_7e_5a, mortar: 0xc4_ba_ac },
    BrickType { key: "brick-fire", name: "Fire brick", length: 0.230, height: 0.064, width: 0.114, joint: 0.005, color: 0xd8_c9_9e, mortar: 0xd0_c4_a0 },
    BrickType { key: "brick-sandlime", name: "Sandlime white", length: 0.250, height: 0.067, width: 0.120, joint: 0.010, color: 0xd9_d4_c8, mortar: 0xcc_c6_ba },
    BrickType { key: "brick-brown", name: "Brown stock", length: 0.215, height: 0.065, width: 0.1025, joint: 0.010, color: 0x6e_50_3c, mortar: 0xac_a4_98 },
    BrickType { key: "brick-black", name: "Glazed black", length: 0.240, height: 0.071, width: 0.115, joint: 0.008, color: 0x2c_2c_30, mortar: 0x8a_88_84 },
];

pub fn brick_type(key: &str) -> Option<&'static BrickType> {
    BRICK_TYPES.iter().find(|b| b.key == key)
}

/// The wall assembly presets that are brick by construction
/// (`WALL_TYPES` keys): a new wall of these types starts with the matching
/// finish when nothing else is set.
pub fn default_finish_for_thickness(thickness: f64) -> Option<&'static str> {
    match (thickness * 1000.0).round() as i64 {
        230 => Some("brick-red"),
        _ => None,
    }
}

/// One brick in face-local coordinates: `(u0, z0, du, dz)` — meters along the
/// face from its left end, meters up from its bottom, then the brick's cut
/// width and height including its share of mortar (the brick body itself is
/// drawn inset by the joint).
pub type BrickCell = [f64; 4];

/// Lay stretcher-bond bricks over a wall face.
///
/// * `face_len` — the face length along the wall (meters).
/// * `face_h` — the face height (meters).
/// * `t` — the brick preset (pitches come from it).
/// * `cap` — safety cap on the number of bricks (the software renderer works
///   on display lists; a long facade in Roman brick could otherwise produce
///   tens of thousands of cells). When the face needs more than `cap`, the
///   rows are thinned uniformly so the texture stays representative.
///
/// Rows start at the face bottom; the bottom row is uncut (masons start with
/// a full brick at the quoin). Each row shifts half a pitch against the
/// previous one; the half brick closes the row end.
pub fn face_bricks(face_len: f64, face_h: f64, t: &BrickType, cap: usize) -> Vec<BrickCell> {
    let (pitch_u, pitch_z) = t.pitches();
    if face_len < t.length * 0.5 || face_h < t.height * 0.5 || pitch_u <= 0.0 || pitch_z <= 0.0 {
        return Vec::new();
    }
    let cols = ((face_len + t.joint) / pitch_u).ceil() as usize;
    let rows = ((face_h + t.joint) / pitch_z).ceil() as usize;
    // Thin rows (not columns) when over budget so the bond pattern survives.
    let row_step = rows.div_ceil((cap / cols.max(1)).max(1));
    let mut out = Vec::with_capacity(cap.min(cols * rows));
    for row in (0..rows).step_by(row_step.max(1)) {
        let z0 = row as f64 * pitch_z;
        let z1 = (z0 + t.height).min(face_h);
        if z1 - z0 < t.height * 0.25 {
            break;
        }
        // Half-brick offset on odd rows (stretcher bond).
        let shift = if row % 2 == 1 { t.length / 2.0 + t.joint / 2.0 } else { 0.0 };
        let mut u = -shift;
        while u < face_len {
            // Clip to the face: cut bricks at both ends.
            let u0 = u.max(0.0);
            let u1 = (u + t.length).min(face_len);
            let w = u1 - u0;
            if w >= t.length * 0.25 {
                out.push([u0, z0, w, z1 - z0]);
            }
            u += pitch_u;
            if out.len() >= cap {
                return out;
            }
        }
    }
    out
}

/// Bricks needed for one square meter of face (schedule estimate).
pub fn bricks_per_m2(t: &BrickType) -> f64 {
    let (pu, pz) = t.pitches();
    if pu <= 0.0 || pz <= 0.0 {
        0.0
    } else {
        1.0 / (pu * pz)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn brick_lookup_and_defaults() {
        assert!(brick_type("brick-red").is_some());
        assert!(brick_type("nope").is_none());
        assert_eq!(default_finish_for_thickness(0.23), Some("brick-red"));
        assert_eq!(default_finish_for_thickness(0.20), None);
    }

    #[test]
    fn stretcher_bond_offsets_alternate() {
        let t = brick_type("brick-red").unwrap();
        // 4.3 m long, 2.7 m high face.
        let cells = face_bricks(4.3, 2.7, t, 100_000);
        assert!(!cells.is_empty());
        // First row starts flush with a full brick; the second row starts
        // with a cut half brick, then full bricks land half a pitch over.
        assert!(cells.iter().any(|c| c[0] < 1e-9 && c[2] >= t.length - 1e-9));
        let second = cells.iter().find(|c| c[1] > t.height).unwrap();
        assert!(second[2] < t.length, "odd rows open with a half brick");
        let full_in_second = cells.iter().find(|c| {
            c[1] > t.height && (c[0] - (t.length / 2.0 + t.joint / 2.0)).abs() < 1e-9
        });
        assert!(full_in_second.is_some(), "stretcher bond shifts full bricks");
        // No cell exceeds the face.
        for c in &cells {
            assert!(c[0] + c[2] <= 4.3 + 1e-9);
            assert!(c[1] + c[3] <= 2.7 + 1e-9);
        }
    }

    #[test]
    fn cap_limits_cells() {
        let t = brick_type("brick-roman").unwrap();
        let cells = face_bricks(30.0, 6.0, t, 500);
        assert!(cells.len() <= 500);
        assert!(!cells.is_empty());
    }

    #[test]
    fn tiny_faces_yield_no_bricks() {
        let t = brick_type("brick-red").unwrap();
        assert!(face_bricks(0.02, 1.0, t, 1000).is_empty());
        assert!(face_bricks(2.0, 0.01, t, 1000).is_empty());
    }

    #[test]
    fn bricks_per_m2_is_realistic() {
        let t = brick_type("brick-red").unwrap();
        let n = bricks_per_m2(t);
        // UK face brick: ~60/m² single skin (±10%).
        assert!((54.0..=68.0).contains(&n), "per m2 = {n}");
    }
}
