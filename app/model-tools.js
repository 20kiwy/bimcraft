/* SPDX-License-Identifier: Apache-2.0 */
/**
 * model-tools.js — where a click lands in the plan view (0.5 part 3, BM.3): snapping for the drawing tools. No DOM:
 * bin/test-bimcraft.mjs --model runs it in Node. Plan mm throughout.
 *
 *   snapPoint(p, { walls, points }, { tol, from, ortho, step, round })
 *     → { p: [x, y], kind: 'end' | 'mid' | 'on' | 'angle' | 'ortho' | 'free', angle?, wall? }
 *   walls [{ a, b, t }] (centre lines; their faces snap too), points [[x, y]] (the chain being drawn, other free points)
 *   tol: how near counts (mm — the page passes so many screen pixels in mm); from: the last point (angles, lengths);
 *   ortho: only 0 / 90 / 180 / 270° from `from` (Shift); step: angle step when nothing else snaps (15°, held when within
 *   3°); round: length step for a free direction (1/2" or 10 mm), so typed-looking numbers come out of clicks.
 *   closesChain(points, p, tol) → true when p is back on the chain's first point (a room closes)
 *   loopSide(points) → 'right' | 'left': where the outside is for walls drawn along this closed loop (anticlockwise
 *   loop: outside on the right)
 */
const D2R = Math.PI / 180;
const dist = (a, b) => Math.hypot(a[0] - b[0], a[1] - b[1]);

export function snapPoint(p, ctx = {}, { tol = 150, from = null, ortho = false, step = 15, round = 0 } = {}) {
  const walls = ctx.walls || [], extra = ctx.points || [];
  let best = null;
  const offer = (q, kind, rank, wall) => { const d = dist(p, q); if (d <= tol && (!best || rank < best.rank || (rank === best.rank && d < best.d))) best = { p: q, kind, rank, d, wall }; };
  for (const w of walls) {
    offer(w.a, 'end', 0, w); offer(w.b, 'end', 0, w);
    offer([(w.a[0] + w.b[0]) / 2, (w.a[1] + w.b[1]) / 2], 'mid', 1, w);
    // the face corners at each end
    const L = dist(w.a, w.b);
    if (L > 0 && w.t > 0) {
      const n = [-(w.b[1] - w.a[1]) / L * w.t / 2, (w.b[0] - w.a[0]) / L * w.t / 2];
      for (const e of [w.a, w.b]) { offer([e[0] + n[0], e[1] + n[1]], 'end', 0, w); offer([e[0] - n[0], e[1] - n[1]], 'end', 0, w); }
    }
  }
  for (const q of extra) offer(q, 'end', 0, null);
  if (best) return { p: best.p, kind: best.kind, wall: best.wall };
  // along a direction from the last point (ortho, or an angle step)
  if (from) {
    const dx = p[0] - from[0], dy = p[1] - from[1], len = Math.hypot(dx, dy);
    if (len > 0) {
      let ang = Math.atan2(dy, dx) / D2R, kind = null;
      if (ortho) { ang = Math.round(ang / 90) * 90; kind = 'ortho'; }
      else if (step > 0) { const s = Math.round(ang / step) * step; if (Math.abs(s - ang) <= 3) { ang = s; kind = 'angle'; } }
      if (kind) {
        let l = len * Math.cos((Math.atan2(dy, dx) / D2R - ang) * D2R);
        // where this direction crosses a wall (its centre line or a face) near the pointer, stop there
        const dir = [Math.cos(ang * D2R), Math.sin(ang * D2R)];
        let hit = null;
        for (const w of walls) {
          for (const off of [0, w.t / 2, -w.t / 2]) {
            const L = dist(w.a, w.b); if (!L) continue;
            const u = [(w.b[0] - w.a[0]) / L, (w.b[1] - w.a[1]) / L], n = [-u[1], u[0]];
            const a = [w.a[0] + n[0] * off, w.a[1] + n[1] * off];
            const c = dir[0] * u[1] - dir[1] * u[0];
            if (Math.abs(c) < 1e-9) continue;
            const s = ((a[0] - from[0]) * u[1] - (a[1] - from[1]) * u[0]) / c, tt = ((a[0] - from[0]) * dir[1] - (a[1] - from[1]) * dir[0]) / c;
            if (s > 1 && tt >= -1 && tt <= L + 1 && Math.abs(s - l) <= tol && (!hit || Math.abs(s - l) < Math.abs(hit.s - l))) hit = { s, w };
          }
        }
        if (hit) return { p: [from[0] + dir[0] * hit.s, from[1] + dir[1] * hit.s], kind: 'on', angle: ang, wall: hit.w };
        if (round > 0) l = Math.max(round, Math.round(l / round) * round);
        return { p: [from[0] + dir[0] * l, from[1] + dir[1] * l], kind, angle: ang };
      }
    }
  }
  // on a wall: its centre line or a face, nearest point
  let on = null;
  for (const w of walls) {
    const L = dist(w.a, w.b); if (!L) continue;
    const u = [(w.b[0] - w.a[0]) / L, (w.b[1] - w.a[1]) / L], n = [-u[1], u[0]];
    for (const off of [0, w.t / 2, -w.t / 2]) {
      const a = [w.a[0] + n[0] * off, w.a[1] + n[1] * off];
      const s = Math.max(0, Math.min(L, (p[0] - a[0]) * u[0] + (p[1] - a[1]) * u[1]));
      const q = [a[0] + u[0] * s, a[1] + u[1] * s], d = dist(p, q);
      if (d <= tol * 0.6 && (!on || d < on.d)) on = { p: q, d, w };
    }
  }
  if (on) return { p: on.p, kind: 'on', wall: on.w };
  if (from && round > 0) {
    const dx = p[0] - from[0], dy = p[1] - from[1], len = Math.hypot(dx, dy);
    if (len > 0) { const l = Math.max(round, Math.round(len / round) * round); return { p: [from[0] + dx / len * l, from[1] + dy / len * l], kind: 'free' }; }
  }
  return { p: [p[0], p[1]], kind: 'free' };
}

export const closesChain = (points, p, tol = 1) => points.length >= 3 && dist(points[0], p) <= tol;

/** Signed area of a closed loop of points (anticlockwise > 0). */
export function loopArea(points) { let s = 0; for (let i = 0; i < points.length; i++) { const a = points[i], b = points[(i + 1) % points.length]; s += a[0] * b[1] - a[1] * b[0]; } return s / 2; }
export const loopSide = (points) => (loopArea(points) > 0 ? 'right' : 'left');
