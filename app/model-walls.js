/* SPDX-License-Identifier: Apache-2.0 */
/**
 * model-walls.js — walls you draw (0.5 part 3, BM.3): wall types, joins cleaned automatically, and the IFC they are
 * written as. No DOM: bin/test-bimcraft.mjs --model runs it in Node.
 *
 *   WALL_TYPES                       starter types: key, layers exterior → interior ({ mat, t } mm), exterior or not
 *   typeThickness(type)              total thickness, mm
 *   wallOutlines(walls, { tol })     plan outlines with L / T / X joins cleaned: walls [{ key, a, b, t }] (centre lines, mm)
 *                                    → Map(key → [[x, y]…] CCW); see "Joins" below
 *   readWalls(w, storey?)            the walls the tools made (pset CadCraft_Wall), in each storey's own plan axes:
 *                                    [{ id, guid, storey, a, b, len, t, height, baseOffset, topLevel, topOffset, typeKey,
 *                                       typeName, extSide, lp }]
 *   addWall(w, { storey, a, b, typeKey, baseOffset, topLevel, topOffset, height, extSide, guid })   → { id, guid }
 *   updateWall(w, guid, changes, { carryJoined })   a, b, typeKey, baseOffset, topLevel, topOffset, height, extSide
 *   deleteWalls(w, guids)            with what they host (openings, doors, windows)
 *   regenWalls(w, storey, guids?)    shapes, quantities and the pset of the walls named and every wall joined to them
 *                                    (all walls of the storey when guids is null) — after any change of walls or levels
 *
 * Written as (Design Transfer View): IfcWall (.STANDARD.) placed relative to its storey at the start point, X along the
 * wall; 'Axis' Curve2D (IfcPolyline from 0,0 to the length); 'Body' SweptSolid (IfcExtrudedAreaSolid of the joined outline
 * as an IfcArbitraryClosedProfileDef, up by the height); IfcMaterialLayerSetUsage (AXIS2; the first layer is the
 * exterior: POSITIVE from −t/2 when the exterior is on the wall's right, NEGATIVE from +t/2 on its left) of the type's
 * IfcMaterialLayerSet; IfcWallType per wall type (pset CadCraft_WallType Key); Pset_WallCommon (Reference, IsExternal,
 * LoadBearing); Qto_WallBaseQuantities (Length, Width, Height, GrossFootprintArea, GrossSideArea, NetSideArea,
 * GrossVolume, NetVolume — openings taken off the net values); CadCraft_Wall (TypeKey, Length, Thickness, Height,
 * BaseOffset, TopLevel = the level its top follows (GlobalId; empty = a fixed height), TopOffset, ExteriorSide).
 *
 * Joins: wall ends within `tol` of each other meet at a node; an end on another wall's side (within half its thickness of
 * its centre line, not at its ends) is a T. At a node the walls are sorted by direction and each pair of neighbours
 * meets where one's left face crosses the other's right face (a mitre for two, the corners round a node for three or
 * more). Faces that never cross (two walls in a line) end square; a mitre longer than four thicknesses is cut square.
 * An outline that comes out crossing itself (a wall shorter than its mitres) falls back to a plain rectangle.
 */
import { ids, val, deleteLine } from './ifc-util.js';
import { round } from './model-write.js';
import { readLevels } from './model-levels.js';
import { t as tr } from './i18n.js';

export const WALL_PSET = 'CadCraft_Wall';
export const WALLTYPE_PSET = 'CadCraft_WallType';
const IN = 25.4;
/** Starter wall types. Layers from the exterior side inward; names from the language file (model.wt.<key>, model.mat.<mat>). */
export const WALL_TYPES = [
  { key: 'ext-2x6', exterior: true, layers: [{ mat: 'siding', t: 0.75 * IN }, { mat: 'osb', t: 0.5 * IN }, { mat: 'studs', t: 5.5 * IN }, { mat: 'gypsum', t: 0.5 * IN }] },
  { key: 'ext-2x4', exterior: true, layers: [{ mat: 'siding', t: 0.75 * IN }, { mat: 'osb', t: 0.5 * IN }, { mat: 'studs', t: 3.5 * IN }, { mat: 'gypsum', t: 0.5 * IN }] },
  { key: 'brick-2x4', exterior: true, layers: [{ mat: 'brick', t: 3.625 * IN }, { mat: 'air', t: 1 * IN }, { mat: 'osb', t: 0.5 * IN }, { mat: 'studs', t: 3.5 * IN }, { mat: 'gypsum', t: 0.5 * IN }] },
  { key: 'int-2x4', exterior: false, layers: [{ mat: 'gypsum', t: 0.5 * IN }, { mat: 'studs', t: 3.5 * IN }, { mat: 'gypsum', t: 0.5 * IN }] },
  { key: 'int-2x6', exterior: false, layers: [{ mat: 'gypsum', t: 0.5 * IN }, { mat: 'studs', t: 5.5 * IN }, { mat: 'gypsum', t: 0.5 * IN }] },
  { key: 'cmu-8', exterior: true, layers: [{ mat: 'cmu', t: 7.625 * IN }] },
  { key: 'concrete-200', exterior: true, layers: [{ mat: 'concrete', t: 200 }] },
  { key: 'ext-300', exterior: true, layers: [{ mat: 'render', t: 15 }, { mat: 'insulation', t: 100 }, { mat: 'block', t: 175 }, { mat: 'plaster', t: 10 }] },
  { key: 'int-100', exterior: false, layers: [{ mat: 'gypsum', t: 12.5 }, { mat: 'studs', t: 75 }, { mat: 'gypsum', t: 12.5 }] },
];
/**
 * A wall type by key: a starter, or a starter at another thickness — key "<starter>@<mm>" (Properties › Thickness): the
 * thickest layer (the studs, the block) takes up the difference, the others keep theirs.
 */
export function wallType(key) {
  const k = String(key || '');
  const base = WALL_TYPES.find((x) => x.key === k.split('@')[0]) || null;
  if (!base || !k.includes('@')) return base;
  const want = Number(k.split('@')[1]);
  const rest = base.layers.reduce((s, l) => s + l.t, 0) - Math.max(...base.layers.map((l) => l.t));
  if (!(want > rest + 1 && want <= 2000)) return null;
  const core = base.layers.indexOf(base.layers.reduce((a, b) => (b.t > a.t ? b : a)));
  return { ...base, key: k, base: base.key, layers: base.layers.map((l, i) => (i === core ? { ...l, t: want - rest } : l)) };
}
/** The key of `type` made `mm` thick (null when its other layers alone are thicker than that). */
export function typeAt(typeKey, mm) {
  const base = wallType(String(typeKey).split('@')[0]);
  if (!base) return null;
  if (Math.abs(typeThickness(base) - mm) < 0.05) return base.key;
  const k = base.key + '@' + (Math.round(mm * 10) / 10);
  return wallType(k) ? k : null;
}
export const typeThickness = (type) => type.layers.reduce((s, l) => s + l.t, 0);
export const typeName = (type) => (type.base ? tr('model.wt.custom', { name: tr('model.wt.' + type.base), t: Math.round(typeThickness(type)) }) : tr('model.wt.' + type.key));
const matName = (mat) => tr('model.mat.' + mat);

// ---------------------------------------------------------------------------------------------------------------- joins
const sub = (a, b) => [a[0] - b[0], a[1] - b[1]];
const add = (a, b) => [a[0] + b[0], a[1] + b[1]];
const mul = (a, k) => [a[0] * k, a[1] * k];
const dot = (a, b) => a[0] * b[0] + a[1] * b[1];
const cross = (a, b) => a[0] * b[1] - a[1] * b[0];
const dist = (a, b) => Math.hypot(a[0] - b[0], a[1] - b[1]);
const left = (d) => [-d[1], d[0]];
/** Where p1 + s·d1 meets p2 + t·d2, or null when they are parallel. */
function meet(p1, d1, p2, d2) {
  const c = cross(d1, d2);
  if (Math.abs(c) < 1e-9) return null;
  const s = cross(sub(p2, p1), d2) / c;
  return add(p1, mul(d1, s));
}

/** The nodes where walls meet: { p, ends: [{ w, end }], tees: [{ w, s }] } (w = index into the prepared list). */
export function wallNodes(W, tol = 1) {
  const nodes = [];
  for (let i = 0; i < W.length; i++) for (const end of ['a', 'b']) {
    const p = W[i][end];
    let n = nodes.find((x) => dist(x.p, p) <= tol);
    if (!n) { n = { p, ends: [], tees: [] }; nodes.push(n); }
    n.ends.push({ w: i, end });
  }
  for (const n of nodes) {
    const here = new Set(n.ends.map((e) => e.w));
    for (let j = 0; j < W.length; j++) {
      if (here.has(j)) continue;
      const w = W[j], s = dot(sub(n.p, w.a), w.u), off = Math.abs(dot(sub(n.p, w.a), w.n));
      if (s > tol && s < w.L - tol && off <= w.h + tol) n.tees.push({ w: j, s });
    }
  }
  return nodes;
}

function prepare(walls) {
  return walls.map((w) => {
    const L = dist(w.a, w.b);
    const u = L > 0 ? mul(sub(w.b, w.a), 1 / L) : [1, 0];
    return { ...w, L, u, n: left(u), h: (w.t || 0) / 2 };
  }).filter((w) => w.L > 0 && w.h > 0);
}

export function wallOutlines(walls, { tol = 1 } = {}) {
  const W = prepare(walls);
  const nodes = wallNodes(W, tol);
  const ends = W.map(() => ({ a: null, b: null }));
  for (const n of nodes) {
    // each ray: a direction out of the node and a point its centre line passes through (a T's through wall: its own axis)
    const rays = n.ends.map((e) => ({ w: e.w, end: e.end, d: e.end === 'a' ? W[e.w].u : mul(W[e.w].u, -1), o: n.p, h: W[e.w].h, real: true }));
    for (const t of n.tees) {
      const T = W[t.w], q = add(T.a, mul(T.u, t.s));
      rays.push({ w: t.w, d: T.u, o: q, h: T.h, real: false }, { w: t.w, d: mul(T.u, -1), o: q, h: T.h, real: false });
    }
    const p = n.p;
    for (const r of rays) { r.ang = Math.atan2(r.d[1], r.d[0]); r.left = add(r.o, mul(left(r.d), r.h)); r.right = add(r.o, mul(left(r.d), -r.h)); }
    if (rays.length > 1) {
      rays.sort((x, y) => x.ang - y.ang);
      for (let i = 0; i < rays.length; i++) {
        const A = rays[i], B = rays[(i + 1) % rays.length];
        if (!A.real && !B.real) continue;   // the two halves of a T's through wall
        // the space between A (its left face) and B, the next one anticlockwise (its right face)
        const c = meet(add(A.o, mul(left(A.d), A.h)), A.d, add(B.o, mul(left(B.d), -B.h)), B.d);
        const turn = cross(A.d, B.d), ahead = dot(A.d, B.d);
        if (!c || (turn <= 1e-9 && ahead > 0)) continue;   // parallel or overlapping: square ends
        if (dist(c, p) > 4 * Math.max(A.h, B.h) + tol) continue;   // too sharp a mitre: square
        A.left = c; B.right = c;
      }
    }
    const centre = rays.length >= 3 && !n.tees.length ? p : null;
    for (const r of rays) if (r.real) ends[r.w][r.end] = { left: r.left, right: r.right, centre };
  }
  const out = new Map();
  W.forEach((w, i) => {
    const A = ends[i].a, B = ends[i].b;
    let poly = [A.right, B.left, B.centre, B.right, A.left, A.centre].filter(Boolean);
    poly = clean(poly);
    if (!simpleCCW(poly)) poly = rect(w);
    out.set(w.key, poly);
  });
  return out;
}
const rect = (w) => [add(w.a, mul(w.n, -w.h)), add(w.b, mul(w.n, -w.h)), add(w.b, mul(w.n, w.h)), add(w.a, mul(w.n, w.h))];
/** Drop repeated and collinear points. */
function clean(poly) {
  let p = poly.filter((q, i) => i === 0 || dist(q, poly[i - 1]) > 1e-6);
  if (p.length > 1 && dist(p[0], p[p.length - 1]) <= 1e-6) p.pop();
  let changed = true;
  while (changed && p.length > 3) {
    changed = false;
    for (let i = 0; i < p.length; i++) {
      const a = p[(i + p.length - 1) % p.length], b = p[i], c = p[(i + 1) % p.length];
      const ab = sub(b, a), bc = sub(c, b);
      if (Math.abs(cross(ab, bc)) <= 1e-6 * (Math.hypot(...ab) * Math.hypot(...bc) + 1e-12) && dot(ab, bc) >= 0) { p.splice(i, 1); changed = true; break; }
    }
  }
  return p;
}
export function polyArea(p) { let s = 0; for (let i = 0; i < p.length; i++) s += cross(p[i], p[(i + 1) % p.length]); return s / 2; }
function simpleCCW(p) {
  if (p.length < 3 || polyArea(p) <= 1e-6) return false;
  const n = p.length;
  for (let i = 0; i < n; i++) for (let j = i + 1; j < n; j++) {
    if (j === i + 1 || (i === 0 && j === n - 1)) continue;
    if (segCross(p[i], p[(i + 1) % n], p[j], p[(j + 1) % n])) return false;
  }
  return true;
}
function segCross(a, b, c, d) {
  const o = (p, q, r) => Math.sign(cross(sub(q, p), sub(r, p)));
  return o(a, b, c) * o(a, b, d) < 0 && o(c, d, a) * o(c, d, b) < 0;
}

// --------------------------------------------------------------------------------------------------------------- reading
/** Every wall the tools made, in its storey's plan axes (mm). `storey` limits it to one storey. */
export function readWalls(w, storey = null) {
  const out = [];
  let list = [];
  try { list = ids(w.api, w.mid, 'IFCWALL', true); } catch (e) { list = ids(w.api, w.mid, 'IFCWALL'); }
  const levels = readLevels(w);
  const so = w.storeyOf();
  for (const id of new Set(list)) {
    const ps = w.readPset(id, WALL_PSET);
    if (!ps) continue;
    const st = so.get(id) ?? null;
    if (storey !== null && st !== storey) continue;
    const line = w.api.GetLine(w.mid, id, false);
    const stLine = st ? w.api.GetLine(w.mid, st, false) : null;
    const lp = line.ObjectPlacement?.value;
    if (!lp || !stLine?.ObjectPlacement) continue;
    const M = w.relMatrix(stLine.ObjectPlacement.value, lp);
    const len = Number(ps.Length) * w.unitMm, th = Number(ps.Thickness) * w.unitMm;
    const a = [M[12], M[13]], dir = [M[0], M[1]];
    const dl = Math.hypot(dir[0], dir[1]) || 1;
    const b = [a[0] + dir[0] / dl * len, a[1] + dir[1] / dl * len];
    const base = levels.find((l) => l.id === st);
    const wall = {
      id, guid: val(line.GlobalId), storey: st, a, b, len, t: th, z: M[14],
      baseOffset: Number(ps.BaseOffset || 0) * w.unitMm, topLevel: ps.TopLevel || '', topOffset: Number(ps.TopOffset || 0) * w.unitMm,
      fixedHeight: Number(ps.Height || 0) * w.unitMm, typeKey: ps.TypeKey || '', typeName: val(line.ObjectType) || val(line.Name) || '', extSide: ps.ExteriorSide === 'right' ? 'right' : 'left', lp,
      name: val(line.Name) || '',
    };
    wall.height = wallHeight(wall, levels, base);
    out.push(wall);
  }
  return out;
}
/** A wall's height: to the level its top follows (+ offset) when that level is there, else its own height. */
export function wallHeight(wall, levels, base = levels.find((l) => l.id === wall.storey)) {
  const top = wall.topLevel ? levels.find((l) => l.guid === wall.topLevel) : null;
  if (top && base) { const h = top.elevation + wall.topOffset - (base.elevation + wall.baseOffset); if (h > 1) return h; }
  return wall.fixedHeight > 0 ? wall.fixedHeight : 2438.4;
}

// --------------------------------------------------------------------------------------------------------------- writing
/** The IfcWallType for a starter type: found by its CadCraft_WallType Key, else made with its layer set. */
export function wallTypeLine(w, key) {
  const type = wallType(key);
  if (!type) throw Object.assign(new Error('model.err.walltype'), { key: 'model.err.walltype' });
  for (const id of ids(w.api, w.mid, 'IFCWALLTYPE')) {
    const tl = w.api.GetLine(w.mid, id, false);
    for (const h of tl.HasPropertySets || []) {
      const ps = w.api.GetLine(w.mid, h.value, false);
      if (val(ps?.Name) !== WALLTYPE_PSET) continue;
      for (const p of ps.HasProperties || []) { const pl = w.api.GetLine(w.mid, p.value, false); if (val(pl?.Name) === 'Key' && val(pl.NominalValue) === key) return { id, set: layerSetOf(w, id), type }; }
    }
  }
  const layers = type.layers.map((l) => w.make('IFCMATERIALLAYER', w.H(w.material(matName(l.mat))), round(w.u(l.t)), null, ...(w.v4 ? [w.L(matName(l.mat)), null, null, null] : [])));
  const set = w.make('IFCMATERIALLAYERSET', layers.map((x) => w.H(x)), w.L(typeName(type)), ...(w.v4 ? [null] : []));
  const ps = w.pset(WALLTYPE_PSET, [['Key', 'IFCIDENTIFIER', key], ['Thickness', 'IFCPOSITIVELENGTHMEASURE', round(w.u(typeThickness(type)))]]);
  const id = w.make('IFCWALLTYPE', w.guid(), w.ownerHistory(), w.L(typeName(type)), null, null, [w.H(ps)], null, null, w.L(typeName(type)), 'STANDARD');
  w.associateMaterial(id, set);
  return { id, set, type };
}
function layerSetOf(w, typeId) {
  const m = w.materialRel(typeId);
  return m?.def ?? null;
}

export function addWall(w, { storey, a, b, typeKey = 'ext-2x6', baseOffset = 0, topLevel = '', topOffset = 0, height = 0, extSide = 'left', guid } = {}) {
  const L = dist(a, b);
  if (!(L >= 1)) throw Object.assign(new Error('model.err.short'), { key: 'model.err.short' });
  const st = w.api.GetLine(w.mid, storey, false);
  if (!st || !/BuildingStorey/i.test(st.constructor?.name || '')) throw new Error(tr('place.err.pick_storey'));
  const T = wallTypeLine(w, typeKey);
  const th = typeThickness(T.type);
  const ang = Math.atan2(b[1] - a[1], b[0] - a[0]) * 180 / Math.PI;
  const lp = w.placeAt(st.ObjectPlacement.value, a[0], a[1], baseOffset, ang);
  const g = w.gid(guid);
  const name = typeName(T.type);
  const id = w.make('IFCWALL', g, w.ownerHistory(), w.L(name), null, w.L(name), w.H(lp), null, null, ...(w.v4 ? ['STANDARD'] : []));
  w.noteGuid(g, id);
  w.contain(id, storey);
  w.relateType(id, T.id);
  w.associateMaterial(id, usage(w, T.set, th, extSide));
  w.upsertPset(id, WALL_PSET, [['TypeKey', 'IFCIDENTIFIER', typeKey], ['Length', 'IFCPOSITIVELENGTHMEASURE', round(w.u(L))], ['Thickness', 'IFCPOSITIVELENGTHMEASURE', round(w.u(th))],
    ['Height', 'IFCPOSITIVELENGTHMEASURE', round(w.u(height > 0 ? height : 2438.4))], ['BaseOffset', 'IFCLENGTHMEASURE', round(w.u(baseOffset))], ['TopLevel', 'IFCIDENTIFIER', topLevel || ''],
    ['TopOffset', 'IFCLENGTHMEASURE', round(w.u(topOffset))], ['ExteriorSide', 'IFCLABEL', extSide]]);
  w.upsertPset(id, 'Pset_WallCommon', [['Reference', 'IFCIDENTIFIER', name], ['IsExternal', 'IFCBOOLEAN', !!T.type.exterior], ['LoadBearing', 'IFCBOOLEAN', false]]);
  return { id, guid: g };
}
function usage(w, set, th, extSide) {
  const right = extSide === 'right';
  return w.make('IFCMATERIALLAYERSETUSAGE', w.H(set), 'AXIS2', right ? 'POSITIVE' : 'NEGATIVE', round(w.u(right ? -th / 2 : th / 2)), ...(w.v4 ? [null] : []));
}

/**
 * Change a wall. `changes`: a, b (plan points, mm), typeKey, baseOffset, topLevel (GlobalId or ''), topOffset, height
 * (a fixed height; sets topLevel ''), extSide. carryJoined: walls whose end sat on a moved end move with it.
 * → { guids } the walls changed (the page regenerates them and their neighbours).
 */
export function updateWall(w, guid, changes = {}, { carryJoined = true } = {}) {
  const all = readWalls(w);
  const wall = all.find((x) => x.guid === guid);
  if (!wall) throw Object.assign(new Error('model.err.nowall'), { key: 'model.err.nowall' });
  const touched = new Set([guid]);
  const moveEnd = (x, end, to) => {
    const na = end === 'a' ? to : x.a, nb = end === 'b' ? to : x.b;
    if (dist(na, nb) < 1) throw Object.assign(new Error('model.err.short'), { key: 'model.err.short' });
    setGeom(w, x, na, nb);
    x.a = na; x.b = nb;
  };
  const a = changes.a || wall.a, b = changes.b || wall.b;
  if (dist(a, wall.a) > 1e-6 || dist(b, wall.b) > 1e-6) {
    if (carryJoined) {
      for (const [end, old, to] of [['a', wall.a, a], ['b', wall.b, b]]) {
        if (dist(old, to) <= 1e-6) continue;
        for (const o of all) {
          if (o.guid === guid || o.storey !== wall.storey) continue;
          if (dist(o.a, old) <= 1) { moveEnd(o, 'a', to); touched.add(o.guid); } else if (dist(o.b, old) <= 1) { moveEnd(o, 'b', to); touched.add(o.guid); }
        }
      }
    }
    if (dist(a, b) < 1) throw Object.assign(new Error('model.err.short'), { key: 'model.err.short' });
    setGeom(w, wall, a, b);
  }
  const rows = [];
  if (changes.typeKey && changes.typeKey !== wall.typeKey) {
    const T = wallTypeLine(w, changes.typeKey);
    const th = typeThickness(T.type);
    // the type relation: out of the old one (gone when it is left empty), into the new type's
    for (const rid of ids(w.api, w.mid, 'IFCRELDEFINESBYTYPE')) {
      const r = w.api.GetLine(w.mid, rid, false);
      if (!(r.RelatedObjects || []).some((h) => h.value === wall.id) || r.RelatingType?.value === T.id) continue;
      const rest = r.RelatedObjects.filter((h) => h.value !== wall.id);
      const l = w.touch(rid);
      if (rest.length) { l.RelatedObjects = rest; w.write(l); } else deleteLine(w.api, w.mid, rid);
    }
    w.relateType(wall.id, T.id);
    const name = typeName(T.type);
    const line = w.touch(wall.id); line.Name = w.L(name); line.ObjectType = w.L(name); w.write(line);
    rows.push(['TypeKey', 'IFCIDENTIFIER', changes.typeKey], ['Thickness', 'IFCPOSITIVELENGTHMEASURE', round(w.u(th))]);
    w.upsertPset(wall.id, 'Pset_WallCommon', [['Reference', 'IFCIDENTIFIER', name], ['IsExternal', 'IFCBOOLEAN', !!T.type.exterior]]);
    setUsage(w, wall.id, T.set, th, changes.extSide || wall.extSide);
  } else if (changes.extSide && changes.extSide !== wall.extSide) {
    const m = w.materialRel(wall.id);
    const u = m && w.api.GetLine(w.mid, m.def, false);
    if (u?.ForLayerSet) setUsage(w, wall.id, u.ForLayerSet.value, wall.t, changes.extSide);
  }
  if (changes.extSide) rows.push(['ExteriorSide', 'IFCLABEL', changes.extSide === 'right' ? 'right' : 'left']);
  if (Number.isFinite(changes.baseOffset)) {
    rows.push(['BaseOffset', 'IFCLENGTHMEASURE', round(w.u(changes.baseOffset))]);
    const M = w.relMatrix(w.api.GetLine(w.mid, wall.storey, false).ObjectPlacement.value, wall.lp);
    const ang = Math.atan2(b[1] - a[1], b[0] - a[0]) * 180 / Math.PI;
    if (Math.abs(M[14] - changes.baseOffset) > 1e-6) w.movePlacement(wall.lp, a[0], a[1], changes.baseOffset, ang);
  }
  if (changes.topLevel !== undefined) rows.push(['TopLevel', 'IFCIDENTIFIER', changes.topLevel || '']);
  if (Number.isFinite(changes.topOffset)) rows.push(['TopOffset', 'IFCLENGTHMEASURE', round(w.u(changes.topOffset))]);
  if (Number.isFinite(changes.height) && changes.height > 0) rows.push(['Height', 'IFCPOSITIVELENGTHMEASURE', round(w.u(changes.height))], ['TopLevel', 'IFCIDENTIFIER', '']);
  if (rows.length) w.upsertPset(wall.id, WALL_PSET, dedupe(rows));
  return { guids: [...touched] };
}
const dedupe = (rows) => [...new Map(rows.map((r) => [r[0], r])).values()];
function setGeom(w, wall, a, b) {
  const ang = Math.atan2(b[1] - a[1], b[0] - a[0]) * 180 / Math.PI;
  const M = w.relMatrix(w.api.GetLine(w.mid, wall.storey, false).ObjectPlacement.value, wall.lp);
  w.movePlacement(wall.lp, a[0], a[1], M[14], ang);
  w.upsertPset(wall.id, WALL_PSET, [['Length', 'IFCPOSITIVELENGTHMEASURE', round(w.u(dist(a, b)))]]);
}
function setUsage(w, wallId, set, th, extSide) {
  const m = w.materialRel(wallId);
  if (m) {
    const r = w.touch(m.rel);
    r.RelatingMaterial = w.H(usage(w, set, th, extSide));
    w.write(r);
    const old = w.api.GetLine(w.mid, m.def, false);
    if (/USAGE$/i.test(old?.constructor?.name || '')) { w.touch(m.def); deleteLine(w.api, w.mid, m.def); }
  } else w.associateMaterial(wallId, usage(w, set, th, extSide));
}

/** Delete walls with what they host: openings (IfcRelVoidsElement) and what fills them. → the storeys touched. */
export function deleteWalls(w, guids) {
  const R = w.hostIndex();
  const list = [], storeys = new Set();
  const so = w.storeyOf();
  for (const g of guids) {
    const id = w.byGuid(g);
    if (!id) continue;
    list.push(id);
    storeys.add(so.get(id));
    for (const op of R.voids.get(id) || []) { list.push(op); for (const f of R.fills.get(op) || []) list.push(f); }
  }
  if (list.length) w.dropProduct(list);
  return { storeys: [...storeys].filter((s) => s !== undefined && s !== null) };
}

/**
 * Bring a storey's walls up to date: every wall whose joined outline, length or height differs from what its shape holds
 * gets a new shape, quantities and pset values; `force` (GlobalIds) are redone anyway (their openings changed).
 * Call it after any change to walls, levels or openings. → { walls: how many were rewritten }
 */
export function regenWalls(w, storey, force = null) {
  const walls = readWalls(w, storey);
  if (!walls.length) return { walls: 0 };
  const outlines = wallOutlines(walls.map((x) => ({ key: x.guid, a: x.a, b: x.b, t: x.t })));
  const forced = new Set(force || []);
  const host = w.hostIndex();
  let n = 0;
  for (const wall of walls) {
    const poly = outlines.get(wall.guid);
    if (!poly) continue;
    const local = toLocal(wall, poly);
    if (!forced.has(wall.guid)) {
      const cur = storedShape(w, wall.id);
      if (cur && Math.abs(cur.depth - wall.height) < 0.01 && Math.abs(cur.len - wall.len) < 0.01 && samePoly(cur.points, local)) continue;
    }
    writeWallShape(w, wall, poly, host);
    n++;
  }
  return { walls: n };
}
const toLocal = (wall, poly) => {
  const ang = Math.atan2(wall.b[1] - wall.a[1], wall.b[0] - wall.a[0]), c = Math.cos(ang), s = Math.sin(ang);
  return poly.map(([x, y]) => [(x - wall.a[0]) * c + (y - wall.a[1]) * s, -(x - wall.a[0]) * s + (y - wall.a[1]) * c]);
};
function samePoly(a, b) {
  if (a.length !== b.length) return false;
  for (let k = 0; k < a.length; k++) { if (a.every((p, i) => dist(p, b[(i + k) % b.length]) < 0.01)) return true; }
  return false;
}
/** What the wall's shape holds now: its body outline (wall axes, mm, closing point dropped), extrusion depth, axis length. */
export function storedShape(w, id) {
  const g = (x) => (x ? w.api.GetLine(w.mid, x.value ?? x, false) : null);
  const line = g(w.api.GetLine(w.mid, id, false)?.Representation);
  if (!line) return null;
  let points = null, depth = null, len = null;
  for (const h of line.Representations || []) {
    const r = g(h);
    const ident = val(r?.RepresentationIdentifier);
    const item = g((r?.Items || [])[0]);
    if (ident === 'Body' && item?.SweptArea) {
      depth = Number(val(item.Depth)) * w.unitMm;
      const curve = g(g(item.SweptArea)?.OuterCurve);
      points = (curve?.Points || []).map((p) => g(p).Coordinates.map((v) => Number(val(v)) * w.unitMm));
      if (points.length > 1 && dist(points[0], points[points.length - 1]) < 1e-6) points.pop();
    } else if (ident === 'Axis' && item?.Points) {
      const pts = item.Points.map((p) => g(p).Coordinates.map((v) => Number(val(v)) * w.unitMm));
      len = pts.length > 1 ? dist(pts[0], pts[pts.length - 1]) : 0;
    }
  }
  return points ? { points, depth, len } : null;
}

/** The wall's Axis and Body from its outline (plan, mm) and height; quantities with its openings taken off. */
export function writeWallShape(w, wall, poly, host = w.hostIndex()) {
  const local = toLocal(wall, poly);
  const axis = w.rep('axis', 'Axis', 'Curve2D', [w.polyline2([[0, 0], [wall.len, 0]])]);
  const body = w.rep('body', 'Body', 'SweptSolid', [w.extrude(w.profile(local), 0, wall.height)]);
  w.setShape(wall.id, [axis, body]);
  const foot = Math.abs(polyArea(local));
  let openArea = 0, openVol = 0;
  for (const op of host.voids.get(wall.id) || []) {
    const q = w.readPset(op, 'CadCraft_Opening');
    if (!q) continue;
    const ow = Number(q.Width) * w.unitMm, oh = Number(q.Height) * w.unitMm;
    if (ow > 0 && oh > 0) { openArea += ow * oh; openVol += ow * oh * wall.t; }
  }
  w.upsertQto(wall.id, 'Qto_WallBaseQuantities', [['Length', 'length', wall.len], ['Width', 'length', wall.t], ['Height', 'length', wall.height],
    ['GrossFootprintArea', 'area', foot], ['GrossSideArea', 'area', wall.len * wall.height], ['NetSideArea', 'area', Math.max(0, wall.len * wall.height - openArea)],
    ['GrossVolume', 'volume', foot * wall.height], ['NetVolume', 'volume', Math.max(0, foot * wall.height - openVol)]]);
  const rows = [['Length', 'IFCPOSITIVELENGTHMEASURE', round(w.u(wall.len))], ['Thickness', 'IFCPOSITIVELENGTHMEASURE', round(w.u(wall.t))]];
  if (wall.topLevel) rows.push(['Height', 'IFCPOSITIVELENGTHMEASURE', round(w.u(wall.height))]);
  w.upsertPset(wall.id, WALL_PSET, rows);
}
