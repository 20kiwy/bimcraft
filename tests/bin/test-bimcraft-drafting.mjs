/* SPDX-License-Identifier: Apache-2.0 */
import assert from 'node:assert/strict';
import path from 'node:path';
import fs from 'node:fs';
import { fileURLToPath, pathToFileURL } from 'node:url';
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
const app = fs.existsSync(path.join(root, 'app/model-tools.js')) ? path.join(root, 'app') : path.join(root, 'public_html/assets/apps/bimcraft/dev');
const { snapPoint, wallPath } = await import(pathToFileURL(path.join(app, 'model-tools.js')));
const { openingType, windowParts, openingSymbol, windowWell } = await import(pathToFileURL(path.join(app, 'model-openings.js')));
const close = (p, q) => p.every((x, i) => Math.abs(x - q[i]) < 1e-6);
const ctx = { walls: [{ a: [4000, 5000], b: [6000, 5000], t: 200 }] };
let s = snapPoint([4020, 2000], ctx, { tol: 50, align: true });
assert.equal(s.kind, 'align'); assert.deepEqual(s.p, [4000, 2000]);
assert.deepEqual(s.guides, [[4000, 5000, 4000, 2000]]);
assert.equal(snapPoint([4020, 2000], ctx, { tol: 50 }).kind, 'free');
assert.equal(snapPoint([4060, 2000], ctx, { tol: 50, align: true }).kind, 'free');
s = snapPoint([4020, 4990], ctx, { tol: 50, align: true }); assert.equal(s.kind, 'end');
s = snapPoint([5000, 5010], ctx, { tol: 50, align: true }); assert.equal(s.kind, 'mid');
s = snapPoint([4020, 20], ctx, { tol: 50, from: [0, 0], ortho: true, round: 127, align: true });
assert.equal(s.kind, 'align'); assert(close(s.p, [4000, 0]));
s = snapPoint([25, 5010], ctx, { tol: 50, from: [0, 0], ortho: true, align: true });
assert.equal(s.kind, 'align'); assert(close(s.p, [0, 5000]));
s = snapPoint([5010, 5000], { points: [[0, 5000]] }, { tol: 50, from: [0, 0], step: 15, align: true });
assert.equal(s.kind, 'align'); assert(close(s.p, [5000, 5000]));
s = snapPoint([1000, 3010], { points: [[0, 3000]] }, { tol: 50, align: true });
assert.deepEqual(s.p, [1000, 3000]);
const pts = [[0, 0], [4000, 0], [4000, 3000], [0, 3000], [0, 0]];
for (const side of ['left', 'right']) for (const reference of ['center', 'inside', 'outside']) {
  const segs = wallPath(pts, 200, reference, side);
  assert.equal(segs.length, 4);
  for (let i = 0; i < 4; i++) assert(close(segs[i].b, segs[(i + 1) % 4].a));
  const off = (reference === 'center' ? 0 : reference === 'outside' ? -100 : 100) * (side === 'left' ? 1 : -1);
  assert(Math.abs(segs[0].a[1] - off) < 1e-6); assert.equal(segs[1].b[0], 4000 - off);
}
assert.deepEqual(wallPath([[0, 0], [0, 0]], 200), []);
const diag = wallPath([[0, 0], [1000, 1000]], 200, 'outside', 'left')[0];
assert(Math.abs(Math.hypot(...diag.a) - 100) < 1e-6);
assert(wallPath([[0, 0], [1000, 0], [1, 1]], 200, 'outside').flatMap((x) => [...x.a, ...x.b]).every(Number.isFinite));
const type = openingType('win-basement-egress');
assert(type.well && type.casement);
for (const face of ['left', 'right']) {
  const op = { ...type, typeKey: type.key, face, offset: 1000, hand: 'a' };
  const parts = windowParts(type, op, 200), well = parts.slice(-3);
  assert.equal(well.length, 3);
  assert(parts.every(({ box: b }) => b[0] < b[3] && b[1] < b[4] && b[2] < b[5]));
  assert(well.every(({ box: b }) => face === 'left' ? b[1] >= 100 : b[4] <= -100));
  const sym = openingSymbol(op, { a: [0, 0], b: [6000, 0], t: 200 });
  assert.equal(sym.dashed.length, 3);
  assert.equal(windowWell(op, 200).far - windowWell(op, 200).near, 914.4);
}
assert.equal(windowWell({ width: 2000, height: 1000 }, 200).x1 - windowWell({ width: 2000, height: 1000 }, 200).x0, 2304.8);
console.log('Drafting regression checks passed: alignment, snap priority, face offsets, corners, egress well geometry and symbols.');
