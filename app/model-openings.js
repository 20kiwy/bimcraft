/* SPDX-License-Identifier: Apache-2.0 */
/**
 * model-openings.js — doors and windows hosted in walls (0.5 part 3, BM.4): starter types, the IFC they are written as,
 * reading them back, and their plan symbols. No DOM: bin/test-bimcraft.mjs --model runs it in Node.
 *
 *   DOOR_TYPES, WINDOW_TYPES, openingType(key)       key, kind ('door' | 'window'), default width / height / sill (mm)
 *   readOpenings(w, storey?) → [{ id, guid, kind, typeKey, typeName, openingId, wall (id), wallGuid, storey, offset, width,
 *                                 height, sill, hand ('a' | 'b'), face ('left' | 'right') }]
 *   addOpening(w, { wall, typeKey, offset, width, height, sill, hand, face, guid })   → { id, guid, opening }
 *   updateOpening(w, guid, { offset, width, height, sill, typeKey, hand, face, wall })   (wall: another wall's GlobalId)
 *   deleteOpenings(w, guids) → { walls: [GlobalIds of the hosts] }
 *   regenOpenings(w, wall)       shapes of everything a wall hosts (after its thickness changed)
 *   fitOffset(wall, width, offset, { gap }) → the offset clamped so the opening stays in the wall, or null when it can't
 *   openingSymbol(op, wall)  → { lines, arcs, thin, dashed, poly } plan symbol (swing arc, leaves, panels, sash lines)
 *
 * Offsets run along the wall from its start (the wall's own X) to the opening's near edge; `sill` is above the level's
 * floor; `face` is the side of the wall a door swings to (its left = +Y of the wall's axes, or its right) — for a window,
 * the side its frame sits on (the outside); `hand` is the end of the opening the hinges are at ('a' = towards the wall's
 * start).
 *
 * Written as (Design Transfer View): IfcOpeningElement (.OPENING.) placed relative to the WALL's placement at (offset, 0,
 * sill − the wall's base offset), X along the wall, its Body a box through the wall (10 mm proud of each face), related
 * by IfcRelVoidsElement (the wall's shape is cut by it — cc-ifc booleans); the IfcDoor / IfcWindow placed relative to the
 * opening (same axes), OverallWidth / OverallHeight, PredefinedType + OperationType (doors) / PartitioningType (windows),
 * its Body built from the parameters (frame, leaves or sashes, glass — coloured with IfcStyledItem), IfcRelFillsElement,
 * contained in the storey, typed by an IfcDoorType / IfcWindowType per starter type (pset CadCraft_DoorType /
 * CadCraft_WindowType Key), Pset_DoorCommon / Pset_WindowCommon (Reference, IsExternal = the wall's), Qto_DoorBaseQuantities
 * / Qto_WindowBaseQuantities (Width, Height, Area), CadCraft_Door / CadCraft_Window (TypeKey, Width, Height, Sill, Hand,
 * Face); the opening gets CadCraft_Opening (Width, Height) so the wall's net quantities can take it off. Moving or
 * deleting the wall moves or deletes all of it (placements are relative; model-walls.js deleteWalls).
 */
import { ids, val, deleteLine } from './ifc-util.js';
import { round } from './model-write.js';
import { readWalls, WALL_PSET } from './model-walls.js';
import { t as tr } from './i18n.js';

const IN = 25.4;
export const DOOR_PSET = 'CadCraft_Door';
export const WINDOW_PSET = 'CadCraft_Window';
export const OPENING_PSET = 'CadCraft_Opening';
const PROUD = 10;   // the opening box stands this far out of each wall face, so the cut always goes through

/** Starter doors: width × height (mm), op = the IFC4 IfcDoorTypeOperationEnum for hinges at end 'a' / 'b'. */
export const DOOR_TYPES = [
  { key: 'door-single', kind: 'door', width: 36 * IN, height: 80 * IN, leaves: 1, op: ['SINGLE_SWING_LEFT', 'SINGLE_SWING_RIGHT'] },
  { key: 'door-double', kind: 'door', width: 60 * IN, height: 80 * IN, leaves: 2, op: ['DOUBLE_DOOR_SINGLE_SWING', 'DOUBLE_DOOR_SINGLE_SWING'] },
  { key: 'door-sliding', kind: 'door', width: 72 * IN, height: 80 * IN, slide: 2, op: ['SLIDING_TO_LEFT', 'SLIDING_TO_RIGHT'] },
  { key: 'door-pocket', kind: 'door', width: 32 * IN, height: 80 * IN, pocket: true, op: ['SLIDING_TO_LEFT', 'SLIDING_TO_RIGHT'] },
  { key: 'door-garage', kind: 'door', width: 16 * 12 * IN, height: 7 * 12 * IN, garage: true, op: ['ROLLINGUP', 'ROLLINGUP'] },
  // the residential set (0.5 part 4 RC.4, bim-residential; names res.ot.*)
  { key: 'door-bifold', kind: 'door', width: 36 * IN, height: 80 * IN, bifold: true, res: true, op: ['FOLDING_TO_LEFT', 'FOLDING_TO_RIGHT'] },
  { key: 'door-french', kind: 'door', width: 60 * IN, height: 80 * IN, leaves: 2, glazed: true, res: true, op: ['DOUBLE_DOOR_SINGLE_SWING', 'DOUBLE_DOOR_SINGLE_SWING'] },
  { key: 'door-barn', kind: 'door', width: 36 * IN, height: 84 * IN, barn: true, res: true, op: ['SLIDING_TO_LEFT', 'SLIDING_TO_RIGHT'] },
  { key: 'door-entry-sidelites', kind: 'door', width: 60 * IN, height: 80 * IN, sidelites: 12 * IN, res: true, op: ['SINGLE_SWING_LEFT', 'SINGLE_SWING_RIGHT'] },
];
/** Starter windows: width × height, sill (the head at 6'-8" like the doors), the IFC4 IfcWindowTypePartitioningEnum. */
export const WINDOW_TYPES = [
  { key: 'win-fixed', kind: 'window', width: 48 * IN, height: 48 * IN, sill: 32 * IN, part: 'SINGLE_PANEL' },
  { key: 'win-single-hung', kind: 'window', width: 36 * IN, height: 48 * IN, sill: 32 * IN, part: 'DOUBLE_PANEL_HORIZONTAL', rail: true },
  { key: 'win-double-hung', kind: 'window', width: 36 * IN, height: 60 * IN, sill: 20 * IN, part: 'DOUBLE_PANEL_HORIZONTAL', rail: true },
  { key: 'win-casement', kind: 'window', width: 30 * IN, height: 48 * IN, sill: 32 * IN, part: 'SINGLE_PANEL', casement: true },
  { key: 'win-slider', kind: 'window', width: 60 * IN, height: 48 * IN, sill: 32 * IN, part: 'DOUBLE_PANEL_VERTICAL', stile: true },
  { key: 'win-awning', kind: 'window', width: 36 * IN, height: 24 * IN, sill: 56 * IN, part: 'SINGLE_PANEL', awning: true, res: true },
  { key: 'win-picture', kind: 'window', width: 72 * IN, height: 60 * IN, sill: 20 * IN, part: 'SINGLE_PANEL', res: true },
  { key: 'win-transom', kind: 'window', width: 36 * IN, height: 12 * IN, sill: 82 * IN, part: 'SINGLE_PANEL', res: true },
];
export const openingType = (key) => DOOR_TYPES.find((x) => x.key === key) || WINDOW_TYPES.find((x) => x.key === key) || null;
export const openingTypeName = (type) => tr((type.res ? 'res.ot.' : 'model.ot.') + type.key);
const FRAME = 1.5 * IN, LEAF = 1.75 * IN, WFRAME = 2 * IN, WDEPTH = 4.5 * IN, GLASS = 6;

// ---------------------------------------------------------------------------------------------------------------- reading
export function readOpenings(w, storey = null, walls = readWalls(w)) {
  const R = w.hostIndex(), out = [];
  const byId = new Map(walls.map((x) => [x.id, x]));
  for (const cls of ['IFCDOOR', 'IFCWINDOW']) {
    let list = [];
    try { list = ids(w.api, w.mid, cls); } catch (e) { continue; }
    for (const id of list) {
      const ps = w.readPset(id, cls === 'IFCDOOR' ? DOOR_PSET : WINDOW_PSET);
      if (!ps) continue;
      const openingId = R.filledBy.get(id), wallId = openingId ? R.voidHost.get(openingId) : null, wall = byId.get(wallId);
      if (!wall || (storey !== null && wall.storey !== storey)) continue;
      const line = w.api.GetLine(w.mid, id, false), op = w.api.GetLine(w.mid, openingId, false);
      const M = w.relMatrix(wall.lp, op.ObjectPlacement.value);
      out.push({
        id, guid: val(line.GlobalId), kind: cls === 'IFCDOOR' ? 'door' : 'window', typeKey: ps.TypeKey || '', typeName: val(line.ObjectType) || val(line.Name) || '',
        openingId, opening: val(op.GlobalId), wall: wallId, wallGuid: wall.guid, storey: wall.storey,
        offset: M[12], width: Number(ps.Width) * w.unitMm, height: Number(ps.Height) * w.unitMm, sill: M[14] + wall.baseOffset,
        hand: ps.Hand === 'b' ? 'b' : 'a', face: ps.Face === 'right' ? 'right' : 'left', name: val(line.Name) || '', mark: val(line.Tag) || '',
      });
    }
  }
  return out;
}

/** Where an opening of `width` can go in `wall` (length len): the offset clamped to [gap, len − width − gap]; null if it can't fit. */
export function fitOffset(wall, width, offset, { gap = 0 } = {}) {
  const max = wall.len - width - gap;
  if (max < gap - 1e-6) return null;
  return Math.max(gap, Math.min(max, offset));
}

// ---------------------------------------------------------------------------------------------------------------- writing
function typeLine(w, type) {
  const door = type.kind === 'door', cls = door ? 'IFCDOORTYPE' : 'IFCWINDOWTYPE', pset = door ? 'CadCraft_DoorType' : 'CadCraft_WindowType';
  for (const id of ids(w.api, w.mid, cls)) {
    const tl = w.api.GetLine(w.mid, id, false);
    for (const h of tl.HasPropertySets || []) {
      const ps = w.api.GetLine(w.mid, h.value, false);
      if (val(ps?.Name) !== pset) continue;
      for (const p of ps.HasProperties || []) { const pl = w.api.GetLine(w.mid, p.value, false); if (val(pl?.Name) === 'Key' && val(pl.NominalValue) === type.key) return id; }
    }
  }
  const ps = w.pset(pset, [['Key', 'IFCIDENTIFIER', type.key]]);
  const name = w.L(openingTypeName(type));
  const head = [w.guid(), w.ownerHistory(), name, null, null, [w.H(ps)], null, null, name];
  if (w.v4) return door ? w.make(cls, ...head, 'DOOR', type.op[0], false, null) : w.make(cls, ...head, 'WINDOW', type.part, false, null);
  // IFC2X3: IfcDoorStyle / IfcWindowStyle carry the operation; the type classes there are for other things
  return null;
}

export function addOpening(w, { wall, typeKey, offset, width, height, sill, hand = 'a', face = 'left', guid, openingGuid } = {}) {
  const type = openingType(typeKey);
  if (!type) throw Object.assign(new Error('model.err.opentype'), { key: 'model.err.opentype' });
  const host = readWalls(w).find((x) => x.guid === wall);
  if (!host) throw Object.assign(new Error('model.err.nowall'), { key: 'model.err.nowall' });
  const W = width > 0 ? width : type.width, H = height > 0 ? height : type.height, S = type.kind === 'door' ? 0 : (Number.isFinite(sill) ? sill : type.sill);
  const off = fitOffset(host, W, Number.isFinite(offset) ? offset : (host.len - W) / 2);
  if (off === null) throw Object.assign(new Error('model.err.nofit'), { key: 'model.err.nofit' });
  if (S + H > host.height + 1) throw Object.assign(new Error('model.err.tootall'), { key: 'model.err.tootall' });
  const door = type.kind === 'door';
  // the opening, in the wall
  const og = w.gid(openingGuid);
  const olp = w.placeAt(host.lp, off, 0, S - host.baseOffset, 0);
  const opening = w.make('IFCOPENINGELEMENT', og, w.ownerHistory(), w.L(tr('model.opening')), null, null, w.H(olp), null, null, ...(w.v4 ? ['OPENING'] : []));
  w.noteGuid(og, opening);
  w.make('IFCRELVOIDSELEMENT', w.guid(), w.ownerHistory(), null, null, w.H(host.id), w.H(opening));
  // the door or window, in the opening
  const g = w.gid(guid);
  const lp = w.placeAt(olp, 0, 0, 0, 0);
  const name = w.L(openingTypeName(type));
  const tail = door ? (w.v4 ? [round(w.u(H)), round(w.u(W)), 'DOOR', type.op[hand === 'b' ? 1 : 0], null] : [round(w.u(H)), round(w.u(W))])
    : (w.v4 ? [round(w.u(H)), round(w.u(W)), 'WINDOW', type.part, null] : [round(w.u(H)), round(w.u(W))]);
  const id = w.make(door ? 'IFCDOOR' : 'IFCWINDOW', g, w.ownerHistory(), name, null, name, w.H(lp), null, w.T('IFCIDENTIFIER', nextMark(w, door)), ...tail);
  w.noteGuid(g, id);
  w.make('IFCRELFILLSELEMENT', w.guid(), w.ownerHistory(), null, null, w.H(opening), w.H(id));
  w.contain(id, host.storey);
  const tl = typeLine(w, type);
  if (tl) w.relateType(id, tl);
  const op = { kind: type.kind, typeKey, width: W, height: H, sill: S, hand: hand === 'b' ? 'b' : 'a', face: face === 'right' ? 'right' : 'left', offset: off };
  writeOpening(w, { id, opening }, op, host);
  return { id, guid: g, opening: og };
}

/** The next free mark for a door ("D1", "D2" …) or a window ("W1" …): one more than the highest of that letter in the file. */
export function nextMark(w, door) {
  const letter = door ? 'D' : 'W';   // i18n-ignore: door and window marks written into the file (as on drawings)
  let hi = 0;
  for (const id of ids(w.api, w.mid, door ? 'IFCDOOR' : 'IFCWINDOW')) {
    const m = String(val(w.api.GetLine(w.mid, id, false)?.Tag) || '').match(/^([A-Z])(\d+)$/);
    if (m && m[1] === letter) hi = Math.max(hi, Number(m[2]));
  }
  return letter + (hi + 1);
}

/** Shapes, psets and quantities of a door / window and its opening, from its parameters and the host wall. */
function writeOpening(w, { id, opening }, op, host) {
  const type = openingType(op.typeKey) || (op.kind === 'door' ? DOOR_TYPES[0] : WINDOW_TYPES[0]);
  const t = host.t;
  w.setShape(opening, [w.rep('body', 'Body', 'SweptSolid', [w.box(0, -t / 2 - PROUD, 0, op.width, t / 2 + PROUD, op.height)])]);
  const parts = op.kind === 'door' ? doorParts(type, op, t) : windowParts(type, op, t);
  const items = [];
  for (const p of parts) {
    const item = w.box(...p.box);
    w.styleItem(item, w.surfaceStyle(STYLE[p.style].name(), STYLE[p.style].rgb, STYLE[p.style].tr));
    items.push(item);
  }
  w.setShape(id, [w.rep('body', 'Body', 'SweptSolid', items)]);
  const door = op.kind === 'door';
  const external = !!w.readPset(host.id, 'Pset_WallCommon')?.IsExternal;
  w.upsertPset(id, door ? DOOR_PSET : WINDOW_PSET, [['TypeKey', 'IFCIDENTIFIER', op.typeKey], ['Width', 'IFCPOSITIVELENGTHMEASURE', round(w.u(op.width))], ['Height', 'IFCPOSITIVELENGTHMEASURE', round(w.u(op.height))],
    ['Sill', 'IFCLENGTHMEASURE', round(w.u(op.sill))], ['Hand', 'IFCLABEL', op.hand], ['Face', 'IFCLABEL', op.face]]);
  w.upsertPset(id, door ? 'Pset_DoorCommon' : 'Pset_WindowCommon', [['Reference', 'IFCIDENTIFIER', openingTypeName(type)], ['IsExternal', 'IFCBOOLEAN', external]]);
  w.upsertQto(id, door ? 'Qto_DoorBaseQuantities' : 'Qto_WindowBaseQuantities', [['Width', 'length', op.width], ['Height', 'length', op.height], ['Area', 'area', op.width * op.height]]);
  w.upsertPset(opening, OPENING_PSET, [['Width', 'IFCPOSITIVELENGTHMEASURE', round(w.u(op.width))], ['Height', 'IFCPOSITIVELENGTHMEASURE', round(w.u(op.height))]]);
  // the element's own size attributes follow
  const l = w.touch(id);
  l.OverallWidth = round(w.u(op.width)); l.OverallHeight = round(w.u(op.height));
  if (door && w.v4) l.OperationType = type.op[op.hand === 'b' ? 1 : 0];
  w.write(l);
}
const STYLE = {
  frame: { name: () => tr('model.style.frame'), rgb: [0.93, 0.92, 0.89], tr: 0 },
  leaf: { name: () => tr('model.style.leaf'), rgb: [0.72, 0.56, 0.38], tr: 0 },
  glass: { name: () => tr('model.style.glass'), rgb: [0.62, 0.79, 0.89], tr: 0.6 },
  garage: { name: () => tr('model.style.garage'), rgb: [0.86, 0.86, 0.84], tr: 0 },
};

/** A door's boxes in the opening's axes (x along, y across the wall, z up): frame, then leaves (closed) or panels. */
export function doorParts(type, op, t) {
  const W = op.width, H = op.height, f = Math.min(FRAME, W / 6), out = [];
  const box = (x0, y0, z0, x1, y1, z1, style) => out.push({ box: [x0, y0, z0, x1, y1, z1], style });
  const y0 = -t / 2, y1 = t / 2;
  box(0, y0, 0, f, y1, H, 'frame'); box(W - f, y0, 0, W, y1, H, 'frame'); box(f, y0, H - f, W - f, y1, H, 'frame');
  const s = op.face === 'right' ? -1 : 1;   // leaves sit at the face they swing to
  const ly0 = s > 0 ? y1 - LEAF : y0, ly1 = s > 0 ? y1 : y0 + LEAF;
  const inner = W - 2 * f, top = H - f;
  if (type.garage) box(f, -LEAF / 2, 0, W - f, LEAF / 2, top, 'garage');
  else if (type.slide) {
    const half = inner / 2 + 25;
    box(f, -LEAF, 0, f + half, 0, top, 'glass'); box(W - f - half, 0, 0, W - f, LEAF, top, 'glass');
    box(f, -LEAF, 0, f + 40, 0, top, 'leaf'); box(W - f - 40, 0, 0, W - f, LEAF, top, 'leaf');
  } else if (type.pocket) box(f, -LEAF / 2, 0, W - f, LEAF / 2, top, 'leaf');
  else if (type.bifold) box(f, -LEAF / 2, 0, W - f, LEAF / 2, top, 'leaf');   // folded shut in 3D
  else if (type.barn) {
    // a slab hung on the wall face, a little wider than the opening, under its track
    const yb = s > 0 ? y1 : y0 - LEAF, yt = s > 0 ? y1 + LEAF : y0;
    box(-50, yb, 0, W + 50, yt, H + 25, 'leaf');
    box(-50, s > 0 ? yt : yb - 25, H + 40, 2 * W, s > 0 ? yt + 25 : yb, H + 80, 'frame');
  } else if (type.sidelites) {
    const sl = Math.min(type.sidelites, W / 4);
    box(f, -GLASS / 2, 0, f + sl, GLASS / 2, top, 'glass'); box(W - f - sl, -GLASS / 2, 0, W - f, GLASS / 2, top, 'glass');
    box(f + sl, y0, 0, f + sl + f, y1, top, 'frame'); box(W - f - sl - f, y0, 0, W - f - sl, y1, top, 'frame');
    box(f + sl + f, ly0, 0, W - f - sl - f, ly1, top - 3, 'leaf');
  } else if (type.leaves === 2) {
    if (type.glazed) {
      // glass in a rail-and-stile frame on each leaf
      for (const [a, b] of [[f, f + inner / 2 - 2], [f + inner / 2 + 2, W - f]]) {
        const st = 100;
        box(a, ly0, 0, a + st, ly1, top - 3, 'leaf'); box(b - st, ly0, 0, b, ly1, top - 3, 'leaf');
        box(a + st, ly0, 0, b - st, ly1, 200, 'leaf'); box(a + st, ly0, top - 3 - st, b - st, ly1, top - 3, 'leaf');
        box(a + st, (ly0 + ly1) / 2 - GLASS / 2, 200, b - st, (ly0 + ly1) / 2 + GLASS / 2, top - 3 - st, 'glass');
      }
    } else { box(f, ly0, 0, f + inner / 2 - 2, ly1, top - 3, 'leaf'); box(f + inner / 2 + 2, ly0, 0, W - f, ly1, top - 3, 'leaf'); }
  } else box(f, ly0, 0, W - f, ly1, top - 3, 'leaf');
  return out;
}
/** A window's boxes: frame (at the face it sits on), sashes' rails and stiles, glass. */
export function windowParts(type, op, t) {
  const W = op.width, H = op.height, f = Math.min(WFRAME, W / 6, H / 6), out = [];
  const box = (x0, y0, z0, x1, y1, z1, style) => out.push({ box: [x0, y0, z0, x1, y1, z1], style });
  const d = Math.min(WDEPTH, t), s = op.face === 'right' ? -1 : 1;
  const y0 = s > 0 ? t / 2 - d : -t / 2, y1 = y0 + d;
  box(0, y0, 0, f, y1, H, 'frame'); box(W - f, y0, 0, W, y1, H, 'frame');
  box(f, y0, 0, W - f, y1, f, 'frame'); box(f, y0, H - f, W - f, y1, H, 'frame');
  const gy = (y0 + y1) / 2;
  if (type.rail) { const m = H / 2; box(f, y0 + d * 0.15, m - f / 2, W - f, y1 - d * 0.15, m + f / 2, 'frame'); }
  if (type.stile) { const m = W / 2; box(m - f / 2, y0 + d * 0.15, f, m + f / 2, y1 - d * 0.15, H - f, 'frame'); }
  box(f, gy - GLASS / 2, f, W - f, gy + GLASS / 2, H - f, 'glass');
  return out;
}

export function updateOpening(w, guid, ch = {}) {
  const walls = readWalls(w);
  const all = readOpenings(w, null, walls);
  const cur = all.find((x) => x.guid === guid);
  if (!cur) throw Object.assign(new Error('model.err.noopening'), { key: 'model.err.noopening' });
  let host = walls.find((x) => x.id === cur.wall);
  const touched = new Set([host.guid]);
  const op = { ...cur };
  for (const k of ['width', 'height', 'sill']) if (Number.isFinite(ch[k]) && ch[k] >= 0) op[k] = ch[k];
  if (op.kind === 'door') op.sill = 0;
  if (ch.typeKey && openingType(ch.typeKey)?.kind === op.kind) op.typeKey = ch.typeKey;
  if (ch.hand === 'a' || ch.hand === 'b') op.hand = ch.hand;
  if (ch.face === 'left' || ch.face === 'right') op.face = ch.face;
  if (!(op.width >= 100 && op.height >= 100)) throw Object.assign(new Error('model.err.small'), { key: 'model.err.small' });
  // another wall: the opening is related to it, placed relative to it
  if (ch.wall && ch.wall !== host.guid) {
    const to = walls.find((x) => x.guid === ch.wall);
    if (!to) throw Object.assign(new Error('model.err.nowall'), { key: 'model.err.nowall' });
    for (const rid of ids(w.api, w.mid, 'IFCRELVOIDSELEMENT')) {
      const r = w.api.GetLine(w.mid, rid, false);
      if (r.RelatedOpeningElement?.value !== cur.openingId) continue;
      const l = w.touch(rid); l.RelatingBuildingElement = w.H(to.id); w.write(l);
    }
    const pl = w.touch(w.api.GetLine(w.mid, cur.openingId, false).ObjectPlacement.value);
    pl.PlacementRelTo = w.H(to.lp); w.write(pl);
    if (to.storey !== host.storey) { moveContainment(w, cur.id, to.storey); }
    host = to; touched.add(to.guid);
  }
  const off = fitOffset(host, op.width, Number.isFinite(ch.offset) ? ch.offset : op.offset);
  if (off === null) throw Object.assign(new Error('model.err.nofit'), { key: 'model.err.nofit' });
  if (op.sill + op.height > host.height + 1) throw Object.assign(new Error('model.err.tootall'), { key: 'model.err.tootall' });
  op.offset = off;
  const olp = w.api.GetLine(w.mid, cur.openingId, false).ObjectPlacement.value;
  w.movePlacement(olp, off, 0, op.sill - host.baseOffset, 0);
  if (op.typeKey !== cur.typeKey) {
    const type = openingType(op.typeKey);
    const tl = typeLine(w, type);
    for (const rid of ids(w.api, w.mid, 'IFCRELDEFINESBYTYPE')) {
      const r = w.api.GetLine(w.mid, rid, false);
      if (!(r.RelatedObjects || []).some((h) => h.value === cur.id)) continue;
      const rest = r.RelatedObjects.filter((h) => h.value !== cur.id);
      const l = w.touch(rid);
      if (rest.length) { l.RelatedObjects = rest; w.write(l); } else dropRel(w, rid);
    }
    if (tl) w.relateType(cur.id, tl);
    const nm = w.L(openingTypeName(type));
    const l = w.touch(cur.id); l.Name = nm; l.ObjectType = nm; w.write(l);
  }
  writeOpening(w, { id: cur.id, opening: cur.openingId }, op, host);
  return { walls: [...touched] };
}
function moveContainment(w, id, storey) {
  for (const rid of ids(w.api, w.mid, 'IFCRELCONTAINEDINSPATIALSTRUCTURE')) {
    const r = w.api.GetLine(w.mid, rid, false);
    if (!(r.RelatedElements || []).some((h) => h.value === id)) continue;
    const rest = r.RelatedElements.filter((h) => h.value !== id);
    const l = w.touch(rid);
    if (rest.length) { l.RelatedElements = rest; w.write(l); } else dropRel(w, rid);
  }
  w.contain(id, storey);
}

export function deleteOpenings(w, guids) {
  const all = readOpenings(w);
  const list = [], walls = new Set();
  for (const g of guids) {
    const o = all.find((x) => x.guid === g);
    if (!o) continue;
    list.push(o.id, o.openingId);
    walls.add(o.wallGuid);
  }
  if (list.length) w.dropProduct(list);
  return { walls: [...walls] };
}

/** After a wall's thickness (or type) changed: every opening it hosts made again for the new thickness. */
export function regenOpenings(w, wall, all = null) {
  const list = (all || readOpenings(w)).filter((o) => o.wall === wall.id);
  for (const o of list) writeOpening(w, { id: o.id, opening: o.openingId }, o, wall);
  return list.length;
}

/** Hosted openings follow a wall whose start moved along its own line: offsets keep their place in the world. */
export function keepPlaceOnMove(w, wallBefore, aAfter) {
  const L = Math.hypot(wallBefore.b[0] - wallBefore.a[0], wallBefore.b[1] - wallBefore.a[1]) || 1;
  const u = [(wallBefore.b[0] - wallBefore.a[0]) / L, (wallBefore.b[1] - wallBefore.a[1]) / L];
  return (aAfter[0] - wallBefore.a[0]) * u[0] + (aAfter[1] - wallBefore.a[1]) * u[1];
}

// ---------------------------------------------------------------------------------------------------------------- plan symbols
/**
 * The plan symbol of a door or window in `wall` (plan points a → b, thickness t): lines (heavy), arcs and thin lines (swing),
 * dashed (what is overhead or inside the wall), poly (the area a click picks). Plan millimetres.
 */
export function openingSymbol(op, wall) {
  const type = openingType(op.typeKey) || (op.kind === 'door' ? DOOR_TYPES[0] : WINDOW_TYPES[0]);
  const L = Math.hypot(wall.b[0] - wall.a[0], wall.b[1] - wall.a[1]) || 1;
  const u = [(wall.b[0] - wall.a[0]) / L, (wall.b[1] - wall.a[1]) / L], n = [-u[1], u[0]];
  const P = (x, y) => [wall.a[0] + u[0] * x + n[0] * y, wall.a[1] + u[1] * x + n[1] * y];
  const seg = (p, q) => [p[0], p[1], q[0], q[1]];
  const t = wall.t, s = op.face === 'right' ? -1 : 1, x0 = op.offset, x1 = op.offset + op.width, W = op.width;
  const out = { lines: [], arcs: [], thin: [], dashed: [], poly: [P(x0, -t / 2), P(x1, -t / 2), P(x1, t / 2), P(x0, t / 2)] };
  // jambs across the wall at both ends of the opening
  out.lines.push(seg(P(x0, -t / 2), P(x0, t / 2)), seg(P(x1, -t / 2), P(x1, t / 2)));
  const ang = (v) => Math.atan2(v[1], v[0]);
  if (op.kind === 'door') {
    const f = Math.min(FRAME, W / 6), face = s * t / 2;
    if (type.garage) {
      out.dashed.push(seg(P(x0, 0), P(x1, 0)));
      out.thin.push(seg(P(x0 + f, face), P(x1 - f, face)));
    } else if (type.slide) {
      const half = (W - 2 * f) / 2 + 25;
      out.lines.push(seg(P(x0 + f, -LEAF / 2), P(x0 + f + half, -LEAF / 2)), seg(P(x1 - f - half, LEAF / 2), P(x1 - f, LEAF / 2)));
      out.thin.push(seg(P(x0 + f, -LEAF), P(x0 + f + half, -LEAF)), seg(P(x1 - f - half, LEAF), P(x1 - f, LEAF)));
    } else if (type.pocket) {
      const into = op.hand === 'b' ? [x1 - f, x1 + W - 2 * f] : [x0 + f - (W - 2 * f), x0 + f];
      out.lines.push(seg(P(x0 + f, 0), P(x1 - f, 0)));
      out.dashed.push(seg(P(into[0], 0), P(into[1], 0)));
    } else if (type.bifold) {
      // two panels from each jamb, folded out at 30° toward the face it opens to (a closet bifold)
      const pw = (W - 2 * f) / 4, a = Math.PI / 6;
      for (const [hx, dir] of [[x0 + f, 1], [x1 - f, -1]]) {
        const p0 = P(hx, face), p1 = P(hx + dir * pw * Math.cos(a), face + s * pw * Math.sin(a)), p2 = P(hx + dir * 2 * pw * Math.cos(a), face);
        out.lines.push(seg(p0, p1), seg(p1, p2));
      }
      out.poly = [P(x0, -s * t / 2), P(x1, -s * t / 2), P(x1, face + s * pw), P(x0, face + s * pw)];
    } else if (type.barn) {
      // the slab on the wall face, slid open along the wall (dashed: where it goes)
      const yb = face + s * LEAF;
      out.lines.push(seg(P(x0 - 50, yb), P(x1 + 50, yb)));
      const into = op.hand === 'b' ? [x0 - W, x0 - 50] : [x1 + 50, x1 + W];
      out.dashed.push(seg(P(into[0], yb), P(into[1], yb)));
    } else if (type.sidelites) {
      const sl = Math.min(type.sidelites, W / 4), d0 = x0 + f + sl + f, d1 = x1 - f - sl - f, r = d1 - d0;
      out.thin.push(seg(P(x0 + f, 0), P(x0 + f + sl, 0)), seg(P(x1 - f - sl, 0), P(x1 - f, 0)));
      out.lines.push(seg(P(x0 + f + sl, -t / 2), P(x0 + f + sl, t / 2)), seg(P(x1 - f - sl, -t / 2), P(x1 - f - sl, t / 2)));
      const hx = op.hand === 'b' ? d1 : d0, dir = op.hand === 'b' ? -1 : 1;
      const H = P(hx, face), open = P(hx, face + s * r), shut = P(hx + dir * r, face);
      out.lines.push(seg(H, open));
      const a0 = ang([open[0] - H[0], open[1] - H[1]]), a1 = ang([shut[0] - H[0], shut[1] - H[1]]);
      let d = a1 - a0; while (d > Math.PI) d -= 2 * Math.PI; while (d < -Math.PI) d += 2 * Math.PI;
      out.arcs.push({ c: H, r, a0, a1: a0 + d });
      out.poly = [P(x0, -s * t / 2), P(x1, -s * t / 2), P(x1, face + s * r), P(x0, face + s * r)];
    } else {
      const leaves = type.leaves === 2 ? [[x0 + f, 1, (W - 2 * f) / 2], [x1 - f, -1, (W - 2 * f) / 2]] : [[op.hand === 'b' ? x1 - f : x0 + f, op.hand === 'b' ? -1 : 1, W - 2 * f]];
      for (const [hx, dir, r] of leaves) {
        const H = P(hx, face), open = P(hx, face + s * r), shut = P(hx + dir * r, face);
        out.lines.push(seg(H, open));
        const a0 = ang([open[0] - H[0], open[1] - H[1]]), a1 = ang([shut[0] - H[0], shut[1] - H[1]]);
        // the short way round from the open leaf to the closed position
        let d = a1 - a0; while (d > Math.PI) d -= 2 * Math.PI; while (d < -Math.PI) d += 2 * Math.PI;
        out.arcs.push({ c: H, r, a0, a1: a0 + d });
      }
      // the swing area picks too
      const ext = P(x0, face + s * (type.leaves === 2 ? (W - 2 * f) / 2 : W - 2 * f)), ext2 = P(x1, face + s * (type.leaves === 2 ? (W - 2 * f) / 2 : W - 2 * f));
      out.poly = [P(x0, -s * t / 2), P(x1, -s * t / 2), ext2, ext];
    }
  } else {
    // a window: the frame line on its face, the glass line in the middle (two for sashes that pass), the sill on the other face
    const d = Math.min(WDEPTH, t), yf = s > 0 ? t / 2 - d : -t / 2, yb = yf + d, gy = (yf + yb) / 2;
    out.lines.push(seg(P(x0, yf), P(x1, yf)), seg(P(x0, yb), P(x1, yb)));
    if (type.stile) out.thin.push(seg(P(x0, gy - 8), P(x0 + W / 2 + 10, gy - 8)), seg(P(x1 - W / 2 - 10, gy + 8), P(x1, gy + 8)));
    else if (type.rail) out.thin.push(seg(P(x0, gy - 8), P(x1, gy - 8)), seg(P(x0, gy + 8), P(x1, gy + 8)));
    else out.thin.push(seg(P(x0, gy), P(x1, gy)));
    if (type.casement) {
      const hx = op.hand === 'b' ? x1 : x0, dir = op.hand === 'b' ? -1 : 1, r = W, face = s * t / 2;
      const H = P(hx, face), open = P(hx + dir * r * Math.cos(Math.PI / 6), face + s * r * Math.sin(Math.PI / 6));
      out.thin.push(seg(H, open));
    }
  }
  return out;
}

/** Delete a relationship line alone (never what it points at), journaled. */
function dropRel(w, rid) { w.touch(rid); deleteLine(w.api, w.mid, rid); }
