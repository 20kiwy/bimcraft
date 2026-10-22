//! BimCraft geometry: f64 scalar math with a Z-up convention (Blender-style).
//!
//! Clean-room foundation for the BIMCraft Rust edition. The parity roadmap
//! (docs/PARITY_ROADMAP.md) describes how this crate is progressively aligned
//! with `cadcraft-geom` (MIT OR Apache-2.0).

/// 3D vector, f64, Z-up.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Vec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Vec3 {
    pub const ZERO: Vec3 = Vec3 { x: 0.0, y: 0.0, z: 0.0 };

    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    pub fn splat(v: f64) -> Self {
        Self { x: v, y: v, z: v }
    }

    pub fn xy(self) -> Vec2 {
        Vec2 { x: self.x, y: self.y }
    }

    pub fn dot(self, o: Self) -> f64 {
        self.x * o.x + self.y * o.y + self.z * o.z
    }

    pub fn cross(self, o: Self) -> Self {
        Self { x: self.y * o.z - self.z * o.y, y: self.z * o.x - self.x * o.z, z: self.x * o.y - self.y * o.x }
    }

    pub fn length(self) -> f64 {
        self.dot(self).sqrt()
    }

    pub fn normalize(self) -> Self {
        let l = self.length();
        if l > 1e-12 {
            self * (1.0 / l)
        } else {
            Self::ZERO
        }
    }

    pub fn dist(self, o: Self) -> f64 {
        (self - o).length()
    }

    pub fn with_z(self, z: f64) -> Vec3 {
        Vec3 { x: self.x, y: self.y, z }
    }
}

impl std::ops::Add for Vec3 {
    type Output = Vec3;
    fn add(self, o: Self) -> Self {
        Self { x: self.x + o.x, y: self.y + o.y, z: self.z + o.z }
    }
}

impl std::ops::Sub for Vec3 {
    type Output = Vec3;
    fn sub(self, o: Self) -> Self {
        Self { x: self.x - o.x, y: self.y - o.y, z: self.z - o.z }
    }
}

impl std::ops::Mul<f64> for Vec3 {
    type Output = Vec3;
    fn mul(self, s: f64) -> Self {
        Self { x: self.x * s, y: self.y * s, z: self.z * s }
    }
}

/// 2D vector, f64 (plan/XY plane).
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Vec2 {
    pub x: f64,
    pub y: f64,
}

impl Vec2 {
    pub const ZERO: Vec2 = Vec2 { x: 0.0, y: 0.0 };

    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    pub fn to3(self, z: f64) -> Vec3 {
        Vec3::new(self.x, self.y, z)
    }

    pub fn dot(self, o: Self) -> f64 {
        self.x * o.x + self.y * o.y
    }

    pub fn length(self) -> f64 {
        self.dot(self).sqrt()
    }

    pub fn dist(self, o: Self) -> f64 {
        (self - o).length()
    }

    pub fn normalize(self) -> Self {
        let l = self.length();
        if l > 1e-12 {
            self * (1.0 / l)
        } else {
            Self::ZERO
        }
    }

    /// Left-hand perpendicular in the XY plane.
    pub fn perp(self) -> Self {
        Self { x: -self.y, y: self.x }
    }

    pub fn angle(self) -> f64 {
        self.y.atan2(self.x)
    }

    pub fn rotate(self, by: f64) -> Self {
        let (s, c) = by.sin_cos();
        Self { x: self.x * c - self.y * s, y: self.x * s + self.y * c }
    }
}

impl std::ops::Add for Vec2 {
    type Output = Vec2;
    fn add(self, o: Self) -> Self {
        Self { x: self.x + o.x, y: self.y + o.y }
    }
}

impl std::ops::Sub for Vec2 {
    type Output = Vec2;
    fn sub(self, o: Self) -> Self {
        Self { x: self.x - o.x, y: self.y - o.y }
    }
}

impl std::ops::Mul<f64> for Vec2 {
    type Output = Vec2;
    fn mul(self, s: f64) -> Self {
        Self { x: self.x * s, y: self.y * s }
    }
}

impl std::ops::Div<f64> for Vec2 {
    type Output = Vec2;
    fn div(self, s: f64) -> Self {
        Self { x: self.x / s, y: self.y / s }
    }
}

/// Signed area of a closed XY polygon (positive = counter-clockwise).
pub fn poly_area(pts: &[Vec2]) -> f64 {
    let n = pts.len();
    if n < 3 {
        return 0.0;
    }
    let mut a = 0.0;
    for i in 0..n {
        let p = pts[i];
        let q = pts[(i + 1) % n];
        a += p.x * q.y - q.x * p.y;
    }
    a / 2.0
}

/// Polygon centroid (planar).
pub fn poly_centroid(pts: &[Vec2]) -> Vec2 {
    let n = pts.len();
    if n == 0 {
        return Vec2::ZERO;
    }
    if n < 3 || poly_area(pts).abs() < 1e-12 {
        let sum = pts.iter().fold(Vec2::ZERO, |acc, p| acc + *p);
        return sum / (n as f64);
    }
    let mut cx = 0.0;
    let mut cy = 0.0;
    let a = poly_area(pts);
    for i in 0..n {
        let p = pts[i];
        let q = pts[(i + 1) % n];
        let cross = p.x * q.y - q.x * p.y;
        cx += (p.x + q.x) * cross;
        cy += (p.y + q.y) * cross;
    }
    Vec2::new(cx / (6.0 * a), cy / (6.0 * a))
}

/// Ear-clipping triangulation of a simple (non self-intersecting) polygon.
/// Returns vertex indices of triangles; handles concave outlines.
pub fn triangulate(pts: &[Vec2]) -> Vec<[usize; 3]> {
    let n = pts.len();
    if n < 3 {
        return Vec::new();
    }
    // Work in counter-clockwise order.
    let ccw = poly_area(pts) > 0.0;
    let mut idx: Vec<usize> = (0..n).collect();
    if !ccw {
        idx.reverse();
    }
    let mut out = Vec::with_capacity(n.saturating_sub(2));
    let cross2 = |o: Vec2, a: Vec2, b: Vec2| (a.x - o.x) * (b.y - o.y) - (a.y - o.y) * (b.x - o.x);
    let point_in_tri = |p: Vec2, t: &[Vec2; 3]| {
        let d1 = cross2(t[0], t[1], p);
        let d2 = cross2(t[1], t[2], p);
        let d3 = cross2(t[2], t[0], p);
        d1 >= -1e-12 && d2 >= -1e-12 && d3 >= -1e-12
    };
    let mut guard = 0;
    while idx.len() > 3 && guard < 10_000 {
        guard += 1;
        let mut clipped = false;
        for k in 0..idx.len() {
            let i0 = idx[(k + idx.len() - 1) % idx.len()];
            let i1 = idx[k];
            let i2 = idx[(k + 1) % idx.len()];
            let a = pts[i0];
            let b = pts[i1];
            let c = pts[i2];
            // Convex ear?
            if cross2(a, b, c) <= 1e-12 {
                continue;
            }
            let tri = [a, b, c];
            // No other vertex inside the ear?
            let contains_other = idx.iter().any(|&j| j != i0 && j != i1 && j != i2 && point_in_tri(pts[j], &tri));
            if contains_other {
                continue;
            }
            out.push([i0, i1, i2]);
            idx.remove(k);
            clipped = true;
            break;
        }
        if !clipped {
            break;
        }
    }
    if idx.len() == 3 {
        out.push([idx[0], idx[1], idx[2]]);
    }
    out
}

/// Intersect a ray (origin `o`, normalized direction `d`) with the Z=0 ground plane.
/// Returns None when the ray is parallel or points away from the plane.
pub fn ray_ground(o: Vec3, d: Vec3) -> Option<Vec3> {
    if d.z.abs() < 1e-9 {
        return None;
    }
    let t = -o.z / d.z;
    if t <= 1e-9 {
        return None;
    }
    Some(o + d * t)
}

/// Shortest distance from point `p` to segment `a-b` (XY plane).
pub fn dist_point_segment_xy(p: Vec2, a: Vec2, b: Vec2) -> f64 {
    let ab = b - a;
    let l2 = ab.dot(ab);
    if l2 < 1e-12 {
        return p.dist(a);
    }
    let t = ((p - a).dot(ab) / l2).clamp(0.0, 1.0);
    p.dist(a + ab * t)
}

/// Parameter `t` (0..1) of the projection of `p` onto segment `a-b`.
pub fn project_on_segment_xy(p: Vec2, a: Vec2, b: Vec2) -> f64 {
    let ab = b - a;
    let l2 = ab.dot(ab);
    if l2 < 1e-12 {
        return 0.0;
    }
    ((p - a).dot(ab) / l2).clamp(0.0, 1.0)
}

/// Axis-aligned 2D bounding box.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Aabb2 {
    pub min: Vec2,
    pub max: Vec2,
}

impl Aabb2 {
    pub fn from_points(pts: &[Vec2]) -> Option<Aabb2> {
        let mut it = pts.iter();
        let first = *it.next()?;
        let mut bb = Aabb2 { min: first, max: first };
        for p in it {
            bb.min.x = bb.min.x.min(p.x);
            bb.min.y = bb.min.y.min(p.y);
            bb.max.x = bb.max.x.max(p.x);
            bb.max.y = bb.max.y.max(p.y);
        }
        Some(bb)
    }

    pub fn contains(&self, p: Vec2) -> bool {
        p.x >= self.min.x - 1e-9 && p.x <= self.max.x + 1e-9 && p.y >= self.min.y - 1e-9 && p.y <= self.max.y + 1e-9
    }

    pub fn expand(&mut self, p: Vec2) {
        self.min.x = self.min.x.min(p.x);
        self.min.y = self.min.y.min(p.y);
        self.max.x = self.max.x.max(p.x);
        self.max.y = self.max.y.max(p.y);
    }

    /// Rectangle footprint of a rotated box: center, size (w, d), rotation.
    pub fn from_rotated_box(center: Vec2, w: f64, d: f64, rot: f64) -> Aabb2 {
        let hw = w / 2.0;
        let hd = d / 2.0;
        let corners = [
            center + Vec2::new(-hw, -hd).rotate(rot),
            center + Vec2::new(hw, -hd).rotate(rot),
            center + Vec2::new(hw, hd).rotate(rot),
            center + Vec2::new(-hw, hd).rotate(rot),
        ];
        Aabb2::from_points(&corners).unwrap_or(Aabb2 { min: center, max: center })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vec_basics() {
        let a = Vec3::new(3.0, 0.0, 4.0);
        assert!((a.length() - 5.0).abs() < 1e-12);
        let c = Vec3::new(1.0, 0.0, 0.0).cross(Vec3::new(0.0, 1.0, 0.0));
        assert!((c.z - 1.0).abs() < 1e-12);
    }

    #[test]
    fn polygon_area() {
        let sq = [Vec2::ZERO, Vec2::new(2.0, 0.0), Vec2::new(2.0, 2.0), Vec2::new(0.0, 2.0)];
        assert!((poly_area(&sq) - 4.0).abs() < 1e-12);
    }

    #[test]
    fn ground_pick() {
        let hit = ray_ground(Vec3::new(0.0, 0.0, 10.0), Vec3::new(0.0, 0.0, -1.0));
        assert_eq!(hit, Some(Vec3::ZERO));
    }

    #[test]
    fn triangulate_square_and_lshape() {
        let sq = [Vec2::ZERO, Vec2::new(2.0, 0.0), Vec2::new(2.0, 2.0), Vec2::new(0.0, 2.0)];
        let tris = triangulate(&sq);
        assert_eq!(tris.len(), 2);
        // L shape (concave, clockwise input).
        let l = [Vec2::new(0.0, 0.0), Vec2::new(3.0, 0.0), Vec2::new(3.0, 1.0), Vec2::new(1.0, 1.0), Vec2::new(1.0, 3.0), Vec2::new(0.0, 3.0)];
        let tris = triangulate(&l);
        assert_eq!(tris.len(), 4);
        let area: f64 = tris.iter().map(|t| poly_area(&[l[t[0]], l[t[1]], l[t[2]]]).abs()).sum();
        assert!((area - 5.0).abs() < 1e-9);
    }

    #[test]
    fn rotated_box_bounds() {
        let bb = Aabb2::from_rotated_box(Vec2::new(1.0, 1.0), 2.0, 1.0, std::f64::consts::FRAC_PI_2);
        assert!((bb.max.x - bb.min.x - 1.0).abs() < 1e-9);
        assert!((bb.max.y - bb.min.y - 2.0).abs() < 1e-9);
    }
}
