//! Rooms and building outlines found from the wall outlines.
//!
//! Rust port of the JavaScript edition's `app/model-rooms.js` (Apache-2.0,
//! getcadcraft/bimcraft) — the algorithm is kept faithful: split every outline
//! edge where edges cross or overlap, keep the pieces that have wall on one
//! side and free space on the other, walk them into closed loops with the free
//! side on the left, then classify: anticlockwise loops are rooms, clockwise
//! loops are either holes (islands) inside a room or the building outlines.
//!
//! The input outlines come from [wall_joined_outlines], the straight-wall port
//! of the JS `wallOutlines` join engine (model-walls.js): L / T / X junctions
//! are mitered so per-wall outlines touch exactly, which is the contract
//! `planLoops` expects. The JS edition works in millimetres; probe distances
//! and area thresholds here are converted to metres (5 cm probe, 1 cm²
//! minimum area). See PROVENANCE.md for the attribution statement.

use crate::Wall;
use bimcraft_geom::Vec2;

const EPS: f64 = 1e-6;

/// A closed space detected on one level.
#[derive(Clone, Debug)]
pub struct RoomLoop {
    /// Boundary polygon (inner wall faces, anticlockwise).
    pub poly: Vec<Vec2>,
    /// Free-standing walls inside the room, reversed to match winding.
    pub holes: Vec<Vec<Vec2>>,
    /// Floor area in m² (holes subtracted).
    pub area: f64,
    /// Boundary perimeter in m (holes included).
    pub perimeter: f64,
    /// A point well inside the polygon, good for a label.
    pub label: Vec2,
}

/// Result of [plan_loops] for one level.
#[derive(Clone, Debug, Default)]
pub struct PlanLoops {
    pub rooms: Vec<RoomLoop>,
    pub outlines: Vec<RoomLoop>,
}

#[derive(Clone, Copy)]
struct Edge {
    a: Vec2,
    b: Vec2,
}

fn cross(a: Vec2, b: Vec2) -> f64 {
    a.x * b.y - a.y * b.x
}

fn dot(a: Vec2, b: Vec2) -> f64 {
    a.x * b.x + a.y * b.y
}

/// Detect rooms / outlines from the wall outline polygons of one level.
///
/// `polys` are the joined wall outlines (see [wall_joined_outlines]); units
/// are meters, `snap` is the vertex-merge grid (1 mm like the JS edition).
pub fn plan_loops(polys: &[Vec<Vec2>]) -> PlanLoops {
    plan_loops_snap(polys, 1e-3)
}

pub fn plan_loops_snap(polys: &[Vec<Vec2>], snap: f64) -> PlanLoops {
    let mut edges: Vec<Edge> = Vec::new();
    for p in polys {
        for i in 0..p.len() {
            let a = p[i];
            let b = p[(i + 1) % p.len()];
            if (b - a).length() > EPS {
                edges.push(Edge { a, b });
            }
        }
    }

    // Split every edge at the points where others cross or touch it.
    let mut cuts: Vec<Vec<f64>> = vec![vec![0.0, 1.0]; edges.len()];
    for i in 0..edges.len() {
        for j in (i + 1)..edges.len() {
            let (a, b) = (edges[i].a, edges[i].b);
            let (c, d) = (edges[j].a, edges[j].b);
            let r = b - a;
            let s = d - c;
            let den = cross(r, s);
            let ca = c - a;
            if den.abs() < 1e-12 * r.length() * s.length() {
                // Parallel: overlapping when collinear — each cut at the other's ends.
                if cross(ca, r).abs() > 1e-6 * r.length() {
                    continue;
                }
                let (rr, ss) = (dot(r, r), dot(s, s));
                if rr > 0.0 {
                    for q in [c, d] {
                        let t = dot(q - a, r) / rr;
                        if t > EPS && t < 1.0 - EPS {
                            cuts[i].push(t);
                        }
                    }
                }
                if ss > 0.0 {
                    for q in [a, b] {
                        let t = dot(q - c, s) / ss;
                        if t > EPS && t < 1.0 - EPS {
                            cuts[j].push(t);
                        }
                    }
                }
                continue;
            }
            let t = cross(ca, s) / den;
            let u = cross(ca, r) / den;
            if !(-EPS..=1.0 + EPS).contains(&t) || !(-EPS..=1.0 + EPS).contains(&u) {
                continue;
            }
            if t > EPS && t < 1.0 - EPS {
                cuts[i].push(t);
            }
            if u > EPS && u < 1.0 - EPS {
                cuts[j].push(u);
            }
        }
    }

    // Merge vertices on a snap grid; keep unique undirected pieces.
    let mut pts: Vec<Vec2> = Vec::new();
    let node = |p: Vec2, pts: &mut Vec<Vec2>| -> usize {
        let kx = (p.x / snap).round();
        let ky = (p.y / snap).round();
        for (idx, q) in pts.iter().enumerate() {
            if (q.x / snap).round() == kx && (q.y / snap).round() == ky {
                return idx;
            }
        }
        pts.push(p);
        pts.len() - 1
    };
    let mut pieces: Vec<(usize, usize)> = Vec::new();
    let mut seen: std::collections::HashSet<(usize, usize)> = std::collections::HashSet::new();
    for (i, e) in edges.iter().enumerate() {
        let mut ts = cuts[i].clone();
        ts.sort_by(|x, y| x.total_cmp(y));
        ts.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
        for k in 0..ts.len().saturating_sub(1) {
            let p = e.a + (e.b - e.a) * ts[k];
            let q = e.a + (e.b - e.a) * ts[k + 1];
            let kp = node(p, &mut pts);
            let kq = node(q, &mut pts);
            if kp == kq {
                continue;
            }
            let id = if kp < kq { (kp, kq) } else { (kq, kp) };
            if seen.insert(id) {
                pieces.push(id);
            }
        }
    }

    // Keep only boundary pieces, oriented so the free side is on the left.
    let inside = |x: f64, y: f64| polys.iter().any(|p| in_poly(x, y, p));
    let mut out: std::collections::BTreeMap<usize, Vec<(usize, f64)>> = std::collections::BTreeMap::new();
    for &(kp, kq) in &pieces {
        let (p, q) = (pts[kp], pts[kq]);
        let d = q - p;
        let len = d.length();
        if len < EPS {
            continue;
        }
        let m = (p + q) * 0.5;
        let n = Vec2::new(-d.y / len, d.x / len);
        // Probe just inside the wall band. The JS edition works in millimetres
        // (h = 0.5 mm); in metres this translates to 5 cm.
        let h = 0.05_f64.min(len / 4.0);
        let left = inside(m.x + n.x * h, m.y + n.y * h);
        let right = inside(m.x - n.x * h, m.y - n.y * h);
        if left == right {
            continue;
        }
        let (from, to) = if left { (kq, kp) } else { (kp, kq) };
        let ang = (pts[to].y - pts[from].y).atan2(pts[to].x - pts[from].x);
        out.entry(from).or_default().push((to, ang));
    }

    // Walk the loops: at each node take the first piece clockwise from the way back.
    struct Arc {
        to: usize,
        ang: f64,
        used: bool,
    }
    let mut out_used: std::collections::BTreeMap<usize, Vec<Arc>> =
        out.into_iter().map(|(k, v)| (k, v.into_iter().map(|(to, ang)| Arc { to, ang, used: false }).collect())).collect();

    let mut loops: Vec<Vec<usize>> = Vec::new();
    let keys: Vec<usize> = out_used.keys().copied().collect();
    for start in keys {
        let list_len = out_used.get(&start).map(|l| l.len()).unwrap_or(0);
        for e0 in 0..list_len {
            if out_used.get(&start).unwrap()[e0].used {
                continue;
            }
            let mut loop_nodes = vec![start];
            let mut cur = start;
            let mut ei = e0;
            let mut closed = false;
            let mut guard = 0usize;
            while guard < 100_000 {
                guard += 1;
                let at = {
                    let arcs = out_used.get_mut(&cur).unwrap();
                    if arcs[ei].used {
                        break;
                    }
                    arcs[ei].used = true;
                    arcs[ei].to
                };
                if at == start {
                    closed = true;
                    break;
                }
                loop_nodes.push(at);
                let back = (pts[cur].y - pts[at].y).atan2(pts[cur].x - pts[at].x);
                let mut best: Option<usize> = None;
                let mut best_a = f64::INFINITY;
                if let Some(arcs) = out_used.get(&at) {
                    for (k, c) in arcs.iter().enumerate() {
                        if c.used {
                            continue;
                        }
                        let mut a = back - c.ang;
                        while a <= 1e-9 {
                            a += std::f64::consts::TAU;
                        }
                        while a > std::f64::consts::TAU + 1e-9 {
                            a -= std::f64::consts::TAU;
                        }
                        if a < best_a {
                            best_a = a;
                            best = Some(k);
                        }
                    }
                }
                match best {
                    Some(k) => {
                        cur = at;
                        ei = k;
                    }
                    None => break,
                }
            }
            if loop_nodes.len() >= 3 && closed {
                loops.push(loop_nodes);
            }
        }
    }

    // Classify.
    let mut rooms: Vec<RoomLoop> = Vec::new();
    let mut islands: Vec<Vec<Vec2>> = Vec::new();
    let mut outlines: Vec<RoomLoop> = Vec::new();
    for l in loops {
        let poly = clean(&l.iter().map(|&k| pts[k]).collect::<Vec<_>>());
        if poly.len() < 3 {
            continue;
        }
        let a = bimcraft_geom::poly_area(&poly);
        // JS thresholds are mm² (|A| > 1 mm²); in metres this is 1 cm².
        if a > 1e-4 {
            let label = label_point(&poly);
            let per = perim(&poly);
            rooms.push(RoomLoop { poly, holes: Vec::new(), area: a, perimeter: per, label });
        } else if a < -1e-4 {
            islands.push(poly);
        }
    }
    // A clockwise loop inside a room is an island (hole); otherwise it runs round a building.
    for l in islands {
        let host = rooms
            .iter()
            .enumerate()
            .filter(|(_, r)| in_poly(l[0].x, l[0].y, &r.poly))
            .min_by(|(_, a), (_, b)| a.area.total_cmp(&b.area))
            .map(|(i, _)| i);
        match host {
            Some(i) if bimcraft_geom::poly_area(&l).abs() < rooms[i].area => {
                let mut h = l;
                h.reverse();
                rooms[i].holes.push(h);
            }
            _ => {
                let mut o = l;
                o.reverse();
                let a = bimcraft_geom::poly_area(&o).abs();
                let per = perim(&o);
                let label = label_point(&o);
                outlines.push(RoomLoop { poly: o, holes: Vec::new(), area: a, perimeter: per, label });
            }
        }
    }
    for r in &mut rooms {
        if r.holes.is_empty() {
            continue;
        }
        r.area -= r.holes.iter().map(|h| bimcraft_geom::poly_area(h).abs()).sum::<f64>();
        r.perimeter += r.holes.iter().map(|h| perim(h)).sum::<f64>();
    }
    rooms.sort_by(|a, b| a.label.y.total_cmp(&b.label.y).then(a.label.x.total_cmp(&b.label.x)));
    PlanLoops { rooms, outlines }
}

/// The smallest room loop containing `p`, or `None`.
pub fn room_at(rooms: &[RoomLoop], p: Vec2) -> Option<&RoomLoop> {
    rooms.iter().filter(|r| in_poly(p.x, p.y, &r.poly) && !r.holes.iter().any(|h| in_poly(p.x, p.y, h))).min_by(|a, b| a.area.total_cmp(&b.area))
}

pub fn perim(p: &[Vec2]) -> f64 {
    p.windows(2).map(|w| (w[1] - w[0]).length()).sum::<f64>() + if p.len() > 2 { (p[0] - p[p.len() - 1]).length() } else { 0.0 }
}

pub fn in_poly(x: f64, y: f64, p: &[Vec2]) -> bool {
    let mut inside = false;
    let n = p.len();
    if n == 0 {
        return false;
    }
    let mut j = n - 1;
    for i in 0..n {
        let (xi, yi) = (p[i].x, p[i].y);
        let (xj, yj) = (p[j].x, p[j].y);
        if (yi > y) != (yj > y) && x < (xj - xi) * (y - yi) / (yj - yi) + xi {
            inside = !inside;
        }
        j = i;
    }
    inside
}

/// A point well inside a polygon (for a label): the centroid when it is
/// inside, else the middle of the widest span through the centroid.
pub fn label_point(p: &[Vec2]) -> Vec2 {
    let c = bimcraft_geom::poly_centroid(p);
    if in_poly(c.x, c.y, p) {
        return c;
    }
    let mut xs: Vec<f64> = Vec::new();
    for i in 0..p.len() {
        let a = p[i];
        let b = p[(i + 1) % p.len()];
        if (a.y > c.y) != (b.y > c.y) {
            xs.push(a.x + (c.y - a.y) / (b.y - a.y) * (b.x - a.x));
        }
    }
    xs.sort_by(|a, b| a.total_cmp(b));
    let mut best: Option<(f64, f64)> = None;
    let mut i = 0;
    while i + 1 < xs.len() {
        if best.map(|(x0, x1)| xs[i + 1] - xs[i] > x1 - x0).unwrap_or(true) {
            best = Some((xs[i], xs[i + 1]));
        }
        i += 2;
    }
    match best {
        Some((x0, x1)) => Vec2::new((x0 + x1) / 2.0, c.y),
        None => c,
    }
}

/// Drop repeated and collinear points of a loop (parity: `clean`).
fn clean(p: &[Vec2]) -> Vec<Vec2> {
    let mut q: Vec<Vec2> = p.iter().enumerate().filter(|(i, v)| *i == 0 || (**v - p[*i - 1]).length() > 1e-6).map(|(_, v)| *v).collect();
    if q.len() > 1 && (q[0] - q[q.len() - 1]).length() <= 1e-6 {
        q.pop();
    }
    let mut changed = true;
    while changed && q.len() > 3 {
        changed = false;
        for i in 0..q.len() {
            let a = q[(i + q.len() - 1) % q.len()];
            let b = q[i];
            let c = q[(i + 1) % q.len()];
            let ab = b - a;
            let bc = c - b;
            if cross(ab, bc).abs() <= 1e-7 * (ab.length() * bc.length() + 1e-12) && dot(ab, bc) > 0.0 {
                q.remove(i);
                changed = true;
                break;
            }
        }
    }
    q
}

/// Raw per-wall mitered outlines (no junction joining; kept for wall fills).
pub fn wall_outlines<'a>(walls: impl IntoIterator<Item = &'a Wall>) -> Vec<Vec<Vec2>> {
    walls.into_iter().map(|w| w.outline()).collect()
}

// ------------------------------------------------- wall outline joining -----
// Straight-wall port of the JavaScript edition's `wallOutlines` join engine
// (model-walls.js): L / T / X junctions between wall segments are mitered so
// that per-segment outlines touch exactly — the input contract of [plan_loops].

const JOIN_TOL: f64 = 1e-3;

/// Joined per-segment outline polygons with L/T/X miters resolved.
///
/// Every wall is expanded into per-segment virtual walls; a ray-casting pass
/// miters each junction exactly, so adjacent outlines share their corner
/// points. Closed centerlines (first point == last point) are handled by the
/// same path: their corner nodes join their own segments.
pub fn wall_joined_outlines<'a>(walls: impl IntoIterator<Item = &'a Wall>) -> Vec<Vec<Vec2>> {
    // Virtual walls: one straight segment each.
    struct Vw {
        a: Vec2,
        b: Vec2,
        half: f64,
    }
    let mut vws: Vec<Vw> = Vec::new();
    for w in walls {
        if w.pts.len() < 2 {
            continue;
        }
        let half = w.thickness / 2.0;
        for (a, b) in w.segments() {
            if (b - a).length() > JOIN_TOL {
                vws.push(Vw { a, b, half });
            }
        }
    }
    if vws.is_empty() {
        return Vec::new();
    }

    // Real end rays: (virtual wall, at_start, pos, direction into the wall).
    let mut ends: Vec<(usize, bool, Vec2, Vec2)> = Vec::new();
    for (vi, v) in vws.iter().enumerate() {
        let d0 = (v.b - v.a).normalize();
        ends.push((vi, true, v.a, d0));
        ends.push((vi, false, v.b, d0 * -1.0));
    }

    let key = |p: Vec2| -> (i64, i64) { ((p.x / JOIN_TOL).round() as i64, (p.y / JOIN_TOL).round() as i64) };
    let mut nodes: std::collections::BTreeMap<(i64, i64), Vec<usize>> = std::collections::BTreeMap::new();
    for (ei, (_, _, pos, _)) in ends.iter().enumerate() {
        nodes.entry(key(*pos)).or_default().push(ei);
    }

    struct Ray {
        /// Virtual wall index (real rays only).
        wall: usize,
        /// True at the segment's start end, false at its end.
        at_start: bool,
        /// Direction pointing from the anchor INTO the wall.
        d: Vec2,
        /// Anchor: node position (real) or tee projection (fake).
        o: Vec2,
        /// Half thickness of the wall the face belongs to.
        h: f64,
        real: bool,
        /// Adjusted geometric-left / geometric-right outline points.
        adj_left: Option<Vec2>,
        adj_right: Option<Vec2>,
    }

    // Adjusted face points per virtual wall: [left_a, left_b, right_a, right_b].
    let mut adj: Vec<[Option<Vec2>; 4]> = vec![[None; 4]; vws.len()];

    for end_ids in nodes.values() {
        let node_pos = ends[end_ids[0]].2;
        let mut rays: Vec<Ray> = end_ids
            .iter()
            .map(|&ei| {
                let (vi, at_start, pos, d) = ends[ei];
                Ray { wall: vi, at_start, d, o: pos, h: vws[vi].half, real: true, adj_left: None, adj_right: None }
            })
            .collect();
        // Tee candidates: segments passing through this node whose wall has no
        // real end here (a T junction's through wall).
        let ends_walls: std::collections::HashSet<usize> = end_ids.iter().map(|&ei| ends[ei].0).collect();
        for (vi, v) in vws.iter().enumerate() {
            if ends_walls.contains(&vi) {
                continue;
            }
            if dist_point_segment_xy(node_pos, v.a, v.b) > JOIN_TOL {
                continue;
            }
            if (v.a - node_pos).length() <= JOIN_TOL || (v.b - node_pos).length() <= JOIN_TOL {
                continue;
            }
            let dseg = (v.b - v.a).normalize();
            let t = bimcraft_geom::project_on_segment_xy(node_pos, v.a, v.b);
            let q = v.a + (v.b - v.a) * t;
            rays.push(Ray { wall: vi, at_start: true, d: dseg, o: q, h: v.half, real: false, adj_left: None, adj_right: None });
            rays.push(Ray { wall: vi, at_start: false, d: dseg * -1.0, o: q, h: v.half, real: false, adj_left: None, adj_right: None });
        }

        if rays.len() > 1 {
            rays.sort_by(|x, y| (x.d.y).atan2(x.d.x).total_cmp(&(y.d.y).atan2(y.d.x)));
            let n = rays.len();
            for i in 0..n {
                let (ai, bi) = (i, (i + 1) % n);
                if ai == bi {
                    break;
                }
                if !rays[ai].real && !rays[bi].real {
                    continue; // the two halves of a through wall
                }
                let turn = rays[ai].d.x * rays[bi].d.y - rays[ai].d.y * rays[bi].d.x;
                let ahead = rays[ai].d.dot(rays[bi].d);
                if turn <= 1e-9 && ahead > 1e-9 {
                    continue; // parallel or overlapping: square ends
                }
                let al = rays[ai].o + rays[ai].d.perp() * rays[ai].h;
                let br = rays[bi].o - rays[bi].d.perp() * rays[bi].h;
                let Some(c) = meet(al, rays[ai].d, br, rays[bi].d) else { continue };
                if (c - node_pos).length() > 4.0 * rays[ai].h.max(rays[bi].h) + JOIN_TOL {
                    continue; // too sharp a mitre: square ends
                }
                // A takes c on its ray-left side, B on its ray-right side.
                // End a: ray-left = geometric left, ray-right = geometric right.
                // End b: ray-left = geometric right, ray-right = geometric left.
                if rays[ai].real {
                    if rays[ai].at_start {
                        rays[ai].adj_left = Some(c);
                    } else {
                        rays[ai].adj_right = Some(c);
                    }
                }
                if rays[bi].real {
                    if rays[bi].at_start {
                        rays[bi].adj_right = Some(c);
                    } else {
                        rays[bi].adj_left = Some(c);
                    }
                }
            }
        }
        for r in rays.iter().filter(|r| r.real) {
            let (li, ri) = if r.at_start { (0, 2) } else { (1, 3) };
            if let Some(c) = r.adj_left {
                adj[r.wall][li] = Some(c);
            }
            if let Some(c) = r.adj_right {
                adj[r.wall][ri] = Some(c);
            }
        }
    }

    // Assemble one outline quad per virtual wall.
    let mut out: Vec<Vec<Vec2>> = Vec::with_capacity(vws.len());
    for (vi, v) in vws.iter().enumerate() {
        let n = (v.b - v.a).normalize().perp();
        let la = adj[vi][0].unwrap_or(v.a + n * v.half);
        let lb = adj[vi][1].unwrap_or(v.b + n * v.half);
        let ra = adj[vi][2].unwrap_or(v.a - n * v.half);
        let rb = adj[vi][3].unwrap_or(v.b - n * v.half);
        out.push(vec![ra, rb, lb, la]);
    }
    out
}

/// Intersection of line (p + t·d) and line (q + s·e).
fn meet(p: Vec2, d: Vec2, q: Vec2, e: Vec2) -> Option<Vec2> {
    let den = d.x * e.y - d.y * e.x;
    if den.abs() < 1e-12 {
        return None;
    }
    let qp = q - p;
    let t = (qp.x * e.y - qp.y * e.x) / den;
    Some(p + d * t)
}

use bimcraft_geom::dist_point_segment_xy;

#[cfg(test)]
mod tests {
    use super::*;

    fn w(pts: &[&[f64]]) -> Wall {
        Wall { level: 0, pts: pts.iter().map(|p| Vec2::new(p[0], p[1])).collect(), thickness: 0.2, height: 3.0 }
    }

    #[test]
    fn one_rectangular_room() {
        // 4 m × 3 m closed room from a single closed centerline.
        let walls = vec![w(&[&[0.0, 0.0], &[4.0, 0.0], &[4.0, 3.0], &[0.0, 3.0], &[0.0, 0.0]])];
        let polys = wall_joined_outlines(&walls);
        let loops = plan_loops(&polys);
        assert_eq!(loops.rooms.len(), 1, "rooms: {}", loops.rooms.len());
        let r = &loops.rooms[0];
        // Inner face area ≈ (4 - 0.2) × (3 - 0.2).
        assert!((r.area - 3.8 * 2.8).abs() < 0.15, "area {}", r.area);
        assert!(!loops.outlines.is_empty());
        assert!(loops.outlines.iter().all(|o| o.area > r.area));
    }

    #[test]
    fn one_rectangular_room_from_four_walls() {
        let walls =
            vec![w(&[&[0.0, 0.0], &[4.0, 0.0]]), w(&[&[4.0, 0.0], &[4.0, 3.0]]), w(&[&[4.0, 3.0], &[0.0, 3.0]]), w(&[&[0.0, 3.0], &[0.0, 0.0]])];
        let polys = wall_joined_outlines(&walls);
        let loops = plan_loops(&polys);
        assert_eq!(loops.rooms.len(), 1, "rooms: {}", loops.rooms.len());
        assert!((loops.rooms[0].area - 3.8 * 2.8).abs() < 0.15, "area {}", loops.rooms[0].area);
    }

    #[test]
    fn two_rooms_split_by_wall() {
        let walls = vec![w(&[&[0.0, 0.0], &[8.0, 0.0], &[8.0, 4.0], &[0.0, 4.0], &[0.0, 0.0]]), w(&[&[4.0, 0.0], &[4.0, 4.0]])];
        let polys = wall_joined_outlines(&walls);
        let loops = plan_loops(&polys);
        assert_eq!(loops.rooms.len(), 2, "expected 2 rooms");
        for r in &loops.rooms {
            assert!(r.area > 1.0);
            assert!(in_poly(r.label.x, r.label.y, &r.poly), "label inside");
        }
    }

    #[test]
    fn tee_junction_wall() {
        // Interior wall ending on a through wall (T junction).
        let walls = vec![
            w(&[&[0.0, 0.0], &[8.0, 0.0], &[8.0, 4.0], &[0.0, 4.0], &[0.0, 0.0]]),
            w(&[&[4.0, 0.0], &[4.0, 4.0]]),
            w(&[&[2.0, 2.0], &[4.0, 2.0]]),
        ];
        let polys = wall_joined_outlines(&walls);
        let loops = plan_loops(&polys);
        assert!(loops.rooms.len() >= 2, "rooms: {}", loops.rooms.len());
    }

    #[test]
    fn free_standing_wall_is_hole() {
        let walls = vec![
            w(&[&[0.0, 0.0], &[6.0, 0.0], &[6.0, 6.0], &[0.0, 6.0], &[0.0, 0.0]]),
            w(&[&[2.0, 2.0], &[3.0, 2.0], &[3.0, 3.0], &[2.0, 3.0], &[2.0, 2.0]]),
        ];
        let polys = wall_joined_outlines(&walls);
        let loops = plan_loops(&polys);
        // The big room plus the tiny space the free-standing wall encloses.
        assert_eq!(loops.rooms.len(), 2, "rooms: {}", loops.rooms.len());
        let big = loops.rooms.iter().max_by(|a, b| a.area.total_cmp(&b.area)).unwrap();
        assert_eq!(big.holes.len(), 1, "the free-standing wall is a hole in the big room");
        assert!(big.area < 5.8 * 5.8, "hole must subtract area: {}", big.area);
        let small = loops.rooms.iter().min_by(|a, b| a.area.total_cmp(&b.area)).unwrap();
        assert!(small.area < 2.0, "closet room: {}", small.area);
    }

    #[test]
    fn open_wall_chain_has_no_room() {
        let walls = vec![w(&[&[0.0, 0.0], &[4.0, 0.0], &[4.0, 3.0]])];
        let polys = wall_joined_outlines(&walls);
        let loops = plan_loops(&polys);
        assert!(loops.rooms.is_empty());
    }

    #[test]
    fn room_at_picks_smallest() {
        let walls = vec![w(&[&[0.0, 0.0], &[8.0, 0.0], &[8.0, 4.0], &[0.0, 4.0], &[0.0, 0.0]]), w(&[&[4.0, 0.0], &[4.0, 4.0]])];
        let polys = wall_joined_outlines(&walls);
        let loops = plan_loops(&polys);
        let hit = room_at(&loops.rooms, Vec2::new(2.0, 2.0)).unwrap();
        assert!(hit.area < 18.0, "should pick the smaller room, got {}", hit.area);
    }
}
