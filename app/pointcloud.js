/* SPDX-License-Identifier: Apache-2.0 */
/**
 * pointcloud.js — point clouds in BimCraft and CadCraft (0.5 SG.1, suggestions #507 and #600). No DOM, no three.js:
 * bin/test-pointcloud.mjs runs all of it in Node. Canonical here in bimcraft/dev; CadCraft carries a byte-identical copy
 * (`node tools/pointcloud/sync.mjs [--check]`, which also copies CivilCraft's las.js / terrain.js / crs.js that this reads
 * LAS and LAZ through — CivilCraft's reader and its laz-perf, never a second LAS reader).
 *
 *   sniffCloud(name, head) → 'las' | 'xyz' | null       by the LASF signature, else the extension (.xyz .pts)
 *   await readCloudInfo(file) → info                   header only (LAS/LAZ) or the first 64 KB (text):
 *     { kind 'las'|'laz'|'xyz', name, size, count (exact for LAS/LAZ, an estimate for text), estimate (bool),
 *       min [3] | null, max [3] | null, color (bool), intensity (bool), classes (bool), crs, code, crsNote,
 *       unit ('m'|'ft'|'ftUS'|null — x and y), zUnit, notes[], las (the las.js header) | null, columns (text) }
 *   await loadCloud(file, info, { budget, signal, progress(fraction), lazPerf }) → cloud
 *     every point is read once; at most `budget` are kept, evenly through the file (a scan's own order, so the kept
 *     points thin it evenly). Positions are float32 RELATIVE to `origin` (file units), so state-plane coordinates keep
 *     their millimetres. cloud = { name, kind, info, read, n, origin [3], pos Float32Array(3n), rgb Uint8Array(3n) | null,
 *     inten Uint16Array(n) | null, cls Uint8Array(n) | null, min [3], max [3] (absolute, of every point read),
 *     classCounts Map(class → points read), unit, zUnit, notes[] }
 *   colorize(cloud, mode, { color }) → Uint8Array(3n)   'rgb' (the file's own; height when it has none) | 'height' |
 *                                                       'intensity' | 'class' | 'single'
 *   sliceOf(cloud, { axis 'x'|'y'|'z', at, thick }) → Uint32Array   points within thick/2 of `at` (absolute file units)
 *   pointAt(cloud, i) → [x, y, z]                        absolute file units
 *   new PlanGrid(cloud, indices | null, cell)           a bucket grid of points in plan (x, y), for snapping:
 *     .nearest(x, y, r) → i | -1                        the point nearest (x, y) within r (absolute file units)
 *   nearestOnScreen(cloud, indices | null, toScreen(x, y, z) → [sx, sy] | null, sx, sy, tol) → i | -1
 *   cloudSummary(cloud) → { read, kept, size [dx, dy, dz], classes [{ c, name, n }] }
 *   UNIT_M { m: 1, ft: 0.3048, ftUS: 1200 / 3937 } · CLOUD_EXT · LIMITS
 *
 * LAS / LAZ: header, coordinate system and LAZ decoding are las.js's (readLasHeader, lazPerf). Each record is read here
 * because the viewer needs more than CivilCraft's terrain import does: intensity (bytes 12–13), the class (formats 0–5
 * low 5 bits of byte 15; 6–10 byte 16) and the colour (formats 2, 3, 5, 7, 8, 10; 16-bit by the spec, but many writers
 * store 0–255 — when no value is over 255 the colours are taken as they are). E57 is not read: no permissive reader yet.
 * Text clouds: "x y z", "x y z r g b", "x y z i", "x y z i r g b" (PTS: a first line with the count), separated by spaces,
 * tabs, commas or semicolons; lines that are not numbers are skipped and counted.
 */

import { readLasHeader, lazPerf, className } from './las.js';
import { t } from './i18n.js';

export const UNIT_M = { m: 1, ft: 0.3048, ftUS: 1200 / 3937 };
export const CLOUD_EXT = ['las', 'laz', 'xyz', 'pts'];
export const LIMITS = { fileBytes: 2000 * 1024 * 1024, textBytes: 400 * 1024 * 1024, budget: 20e6 };

const COLOR_AT = { 2: 20, 3: 28, 5: 28, 7: 30, 8: 30, 10: 30 };
const tick = () => new Promise((r) => setTimeout(r, 0));
const aborted = (signal) => { if (signal?.aborted) throw Object.assign(new Error(t('pc.cancelled')), { name: 'AbortError' }); };

export function sniffCloud(name, head) {
  const h = head instanceof Uint8Array ? head : new Uint8Array(head || []);
  if (h.length >= 4 && h[0] === 0x4c && h[1] === 0x41 && h[2] === 0x53 && h[3] === 0x46) return 'las';   // LASF
  const ext = String(name || '').split('.').pop().toLowerCase();
  if (ext === 'las' || ext === 'laz') return 'las';
  if (ext === 'xyz' || ext === 'pts') return 'xyz';
  return null;
}

// --------------------------------------------------------------------- info
export async function readCloudInfo(file) {
  const head = new Uint8Array(await file.slice(0, 4).arrayBuffer());
  const kind = sniffCloud(file.name, head);
  if (!kind) throw new Error(t('pc.not_cloud', { name: file.name }));
  return kind === 'las' ? lasInfo(file) : textInfo(file);
}

async function lasInfo(file) {
  // the coordinate system is matched against crs.js's catalog: load it once (a page fetches crs-db.json; tests set it)
  try { const crs = await import('./crs.js'); if (!crs.catalogLoaded()) await crs.loadCatalog(); } catch (e) { /* no catalog: the file's system goes unnamed, said */ }
  if (file.size > LIMITS.fileBytes) throw new Error(t('pc.too_big', { name: file.name, mb: Math.round(file.size / 1048576), max: LIMITS.fileBytes / 1048576 }));
  // las.js refuses files over CivilCraft's terrain limit; a viewer keeps only what it shows, so it may read larger ones
  const h = await readLasHeader(lenient(file, 200 * 1048576));
  if (h.compressed && file.size > 1024 * 1048576) throw new Error(t('pc.laz_too_big', { name: file.name, mb: Math.round(file.size / 1048576) }));
  const xyUnit = h.crs?.unit && h.crs.unit !== 'deg' ? h.crs.unit : null;
  const notes = [...h.notes];
  if (h.crs?.unit === 'deg') notes.push(t('pc.degrees'));
  return {
    kind: h.compressed ? 'laz' : 'las', name: file.name, size: file.size, count: h.compressed ? h.count : h.available, estimate: false,
    min: h.min, max: h.max, color: h.format in COLOR_AT, intensity: true, classes: true,
    crs: h.crs, code: h.code, crsNote: h.crsNote, unit: xyUnit, zUnit: h.zUnit || xyUnit, notes, las: h, columns: null,
    version: h.version, format: h.format,
  };
}
/**
 * The file as las.js sees it: readLasHeader's first look at `size` is its terrain limit (200 MB, CivilCraft keeps the
 * points it imports); a viewer keeps only a budget of points, so that first answer is held under the limit and every
 * later one is the real size (bin/test-pointcloud.mjs reads a 210 MB LAS to keep this honest).
 */
function lenient(file, limit) {
  let asked = 0;
  return { name: file.name, get size() { return asked++ === 0 ? Math.min(file.size, limit) : file.size; }, slice: (a, b) => file.slice(a, b) };
}

const SEP = /[\s,;]+/;
function numbers(line) {
  const parts = line.trim().split(SEP);
  const out = [];
  for (const p of parts) { if (p === '') continue; const v = Number(p); if (!Number.isFinite(v)) return null; out.push(v); }
  return out;
}

async function textInfo(file) {
  if (file.size > LIMITS.textBytes) throw new Error(t('pc.too_big', { name: file.name, mb: Math.round(file.size / 1048576), max: LIMITS.textBytes / 1048576 }));
  const text = new TextDecoder().decode(await file.slice(0, 65536).arrayBuffer());
  const lines = text.split(/\r?\n/);
  if (file.size > 65536) lines.pop();   // the last line may be cut
  let first = 0, declared = null;
  const one = numbers(lines[0] || '');
  if (one && one.length === 1 && Number.isInteger(one[0])) { declared = one[0]; first = 1; }   // PTS count line
  let cols = 0, rows = 0, bytes = 0;
  const counts = new Map();
  for (let i = first; i < lines.length; i++) {
    const v = numbers(lines[i]);
    if (!v || v.length < 3) continue;
    counts.set(v.length, (counts.get(v.length) || 0) + 1);
    rows++; bytes += lines[i].length + 1;
  }
  if (!rows) throw new Error(t('pc.text.none', { name: file.name }));
  cols = [...counts.entries()].sort((a, b) => b[1] - a[1])[0][0];
  const columns = textColumns(cols);
  const estimate = declared === null;
  const count = declared ?? Math.max(rows, Math.round(file.size / Math.max(8, bytes / rows)));
  return {
    kind: 'xyz', name: file.name, size: file.size, count, estimate, min: null, max: null,
    color: columns.rgb >= 0, intensity: columns.i >= 0, classes: false, crs: null, code: null, crsNote: t('las.crs.none'),
    unit: null, zUnit: null, notes: [], las: null, columns,
  };
}
/** Which column is what, from how many there are. */
export function textColumns(n) {
  if (n >= 7) return { n, i: 3, rgb: 4 };    // x y z i r g b (PTS)
  if (n === 6) return { n, i: -1, rgb: 3 };  // x y z r g b
  if (n >= 4) return { n, i: 3, rgb: -1 };   // x y z i
  return { n, i: -1, rgb: -1 };
}

// --------------------------------------------------------------------- loading
/** Keeps at most `cap` points evenly: when full, every other kept point goes and the keep rate halves. */
class Keeper {
  constructor(cap, rate, { rgb, inten, cls }) {
    this.cap = Math.max(1, Math.floor(cap));
    this.rate = Math.min(1, rate);
    this.acc = 0;
    this.n = 0;
    this.x = new Float64Array(this.cap); this.y = new Float64Array(this.cap); this.z = new Float64Array(this.cap);
    this.rgb = rgb ? new Uint16Array(this.cap * 3) : null;
    this.inten = inten ? new Uint16Array(this.cap) : null;
    this.cls = cls ? new Uint8Array(this.cap) : null;
  }
  want() {
    this.acc += this.rate;
    if (this.acc < 1) return false;
    this.acc -= 1;
    if (this.n === this.cap) this.halve();
    return true;
  }
  halve() {
    let k = 0;
    for (let j = 0; j < this.n; j += 2, k++) {
      this.x[k] = this.x[j]; this.y[k] = this.y[j]; this.z[k] = this.z[j];
      if (this.rgb) { this.rgb[k * 3] = this.rgb[j * 3]; this.rgb[k * 3 + 1] = this.rgb[j * 3 + 1]; this.rgb[k * 3 + 2] = this.rgb[j * 3 + 2]; }
      if (this.inten) this.inten[k] = this.inten[j];
      if (this.cls) this.cls[k] = this.cls[j];
    }
    this.n = k;
    this.rate /= 2;
  }
  put(x, y, z, r, g, b, i, c) {
    const k = this.n++;
    this.x[k] = x; this.y[k] = y; this.z[k] = z;
    if (this.rgb) { this.rgb[k * 3] = r; this.rgb[k * 3 + 1] = g; this.rgb[k * 3 + 2] = b; }
    if (this.inten) this.inten[k] = i;
    if (this.cls) this.cls[k] = c;
  }
}

class Bounds {
  constructor() { this.min = [Infinity, Infinity, Infinity]; this.max = [-Infinity, -Infinity, -Infinity]; this.n = 0; }
  add(x, y, z) {
    this.n++;
    if (x < this.min[0]) this.min[0] = x; if (x > this.max[0]) this.max[0] = x;
    if (y < this.min[1]) this.min[1] = y; if (y > this.max[1]) this.max[1] = y;
    if (z < this.min[2]) this.min[2] = z; if (z > this.max[2]) this.max[2] = z;
  }
}

export async function loadCloud(file, info, { budget = 2e6, signal = null, progress = null, lazPerf: lp = null } = {}) {
  budget = Math.max(1000, Math.min(LIMITS.budget, Math.floor(budget)));
  const rate = info.count > 0 ? budget / info.count : 1;
  const keep = new Keeper(budget, rate, { rgb: info.color, inten: info.intensity, cls: info.classes });
  const box = new Bounds();
  const classCounts = new Map();
  const notes = [...info.notes];
  let read;
  if (info.kind === 'xyz') read = await scanText(file, info, keep, box, notes, signal, progress);
  else read = await scanLas(file, info, keep, box, classCounts, notes, signal, progress, lp);
  if (!keep.n) throw new Error(t('pc.empty', { name: file.name }));
  return finish(file, info, keep, box, classCounts, read, notes);
}

function finish(file, info, keep, box, classCounts, read, notes) {
  const n = keep.n;
  // the origin: the low corner of what was read, rounded to a whole unit, so the float32 offsets stay small
  const origin = box.min.map((v) => Math.floor(v));
  const pos = new Float32Array(n * 3);
  for (let k = 0; k < n; k++) { pos[k * 3] = keep.x[k] - origin[0]; pos[k * 3 + 1] = keep.y[k] - origin[1]; pos[k * 3 + 2] = keep.z[k] - origin[2]; }
  let rgb = null;
  if (keep.rgb) {
    let hi = 0;
    for (let k = 0; k < n * 3; k++) if (keep.rgb[k] > hi) hi = keep.rgb[k];
    if (hi === 0) notes.push(t('pc.no_color'));
    else {
      const shift = hi > 255 ? 8 : 0;
      rgb = new Uint8Array(n * 3);
      for (let k = 0; k < n * 3; k++) rgb[k] = keep.rgb[k] >> shift;
    }
  }
  if (read < info.count && info.kind !== 'xyz') notes.push(t('pc.read_short', { read: read.toLocaleString('en-US'), count: info.count.toLocaleString('en-US') }));
  return {
    name: file.name, kind: info.kind, info, read, n, origin, pos, rgb,
    inten: keep.inten ? keep.inten.slice(0, n) : null, cls: keep.cls ? keep.cls.slice(0, n) : null,
    min: box.min, max: box.max, classCounts, unit: info.unit, zUnit: info.zUnit || info.unit, notes,
  };
}

function recordDecoder(fmt, L, scale, offset) {
  const [sx, sy, sz] = scale, [ox, oy, oz] = offset, ext = fmt >= 6, ca = COLOR_AT[fmt] ?? -1;
  const colorOk = ca >= 0 && ca + 6 <= L;
  return (dv, o, keep, box, classCounts) => {
    const x = dv.getInt32(o, true) * sx + ox, y = dv.getInt32(o + 4, true) * sy + oy, z = dv.getInt32(o + 8, true) * sz + oz;
    const c = ext ? dv.getUint8(o + 16) : dv.getUint8(o + 15) & 31;
    box.add(x, y, z);
    classCounts.set(c, (classCounts.get(c) || 0) + 1);
    if (!keep.want()) return;
    const i = dv.getUint16(o + 12, true);
    if (colorOk) keep.put(x, y, z, dv.getUint16(o + ca, true), dv.getUint16(o + ca + 2, true), dv.getUint16(o + ca + 4, true), i, c);
    else keep.put(x, y, z, 0, 0, 0, i, c);
  };
}

async function scanLas(file, info, keep, box, classCounts, notes, signal, progress, lp) {
  const h = info.las;
  const dec = recordDecoder(h.format, h.recordLength, h.scale, h.offset);
  let read = 0;
  if (!h.compressed) {
    const L = h.recordLength, per = Math.max(1, Math.floor(4 * 1048576 / L)), total = info.count;
    for (let i = 0; i < total; i += per) {
      aborted(signal);
      const n = Math.min(per, total - i), start = h.offsetToPoints + i * L;
      const dv = new DataView(await file.slice(start, start + n * L).arrayBuffer());
      const m = Math.floor(dv.byteLength / L);
      for (let k = 0; k < m; k++) dec(dv, k * L, keep, box, classCounts);
      read += m;
      if (m < n) break;
      if (progress) { progress(read / total); await tick(); }
    }
    return read;
  }
  const M = await (lp || lazPerf());
  const size = file.size, ptr = M._malloc(size);
  if (!ptr) throw new Error(t('las.laz.memory'));
  let laz = null, buf = 0;
  try {
    for (let off = 0; off < size; off += 8 * 1048576) {
      aborted(signal);
      M.HEAPU8.set(new Uint8Array(await file.slice(off, Math.min(size, off + 8 * 1048576)).arrayBuffer()), ptr + off);
      if (progress) progress(0.05 * off / size);
    }
    laz = new M.LASZip();
    try { laz.open(ptr, size); } catch { throw new Error(t('las.bad.laz_open')); }
    const n = Math.min(laz.getCount(), h.count), L = laz.getPointLength();
    if (L < 20) throw new Error(t('las.bad.laz_len'));
    buf = M._malloc(L);
    let heap = M.HEAPU8.buffer, dv = new DataView(heap, buf, L);
    for (let i = 0; i < n; i++) {
      try { laz.getPoint(buf); } catch { notes.push(t('las.laz.stopped', { read: i.toLocaleString('en-US'), n: n.toLocaleString('en-US') })); break; }
      if (M.HEAPU8.buffer !== heap) { heap = M.HEAPU8.buffer; dv = new DataView(heap, buf, L); }
      dec(dv, 0, keep, box, classCounts);
      read++;
      if ((i & 65535) === 65535) { aborted(signal); if (progress) progress(0.05 + 0.95 * i / n); await tick(); }
    }
  } finally {
    if (laz) laz.delete();
    if (buf) M._free(buf);
    M._free(ptr);
  }
  aborted(signal);
  return read;
}

async function scanText(file, info, keep, box, notes, signal, progress) {
  const cols = info.columns, dec = new TextDecoder();
  let rest = '', read = 0, skipped = 0, first = true;
  const CH = 4 * 1048576;
  const line = (s) => {
    if (first) { first = false; const v = numbers(s); if (v && v.length === 1 && Number.isInteger(v[0])) return; }
    const v = numbers(s);
    if (!v || v.length < 3) { if (s.trim()) skipped++; return; }
    const [x, y, z] = v;
    box.add(x, y, z);
    read++;
    if (!keep.want()) return;
    const i = cols.i >= 0 && v.length > cols.i ? Math.max(0, Math.min(65535, Math.round(v[cols.i]))) : 0;
    if (cols.rgb >= 0 && v.length > cols.rgb + 2) keep.put(x, y, z, clamp16(v[cols.rgb]), clamp16(v[cols.rgb + 1]), clamp16(v[cols.rgb + 2]), i, 0);
    else keep.put(x, y, z, 0, 0, 0, i, 0);
  };
  for (let off = 0; off < file.size; off += CH) {
    aborted(signal);
    const text = rest + dec.decode(await file.slice(off, off + CH).arrayBuffer(), { stream: off + CH < file.size });
    const parts = text.split(/\r?\n/);
    rest = parts.pop();
    for (const s of parts) line(s);
    if (progress) { progress(Math.min(1, (off + CH) / file.size)); await tick(); }
  }
  if (rest) line(rest);
  if (skipped) notes.push(t('pc.text.skipped', { n: skipped, count: skipped.toLocaleString('en-US') }));
  return read;
}
const clamp16 = (v) => Math.max(0, Math.min(65535, Math.round(v)));

// --------------------------------------------------------------------- colours
const ramp = [[0.19, 0.29, 0.75], [0.15, 0.62, 0.85], [0.2, 0.7, 0.35], [0.95, 0.8, 0.2], [0.85, 0.25, 0.15]];
function rampAt(f, out, k) {
  const x = Math.max(0, Math.min(1, f)) * (ramp.length - 1), i = Math.min(ramp.length - 2, Math.floor(x)), u = x - i;
  const a = ramp[i], b = ramp[i + 1];
  out[k] = Math.round((a[0] + (b[0] - a[0]) * u) * 255);
  out[k + 1] = Math.round((a[1] + (b[1] - a[1]) * u) * 255);
  out[k + 2] = Math.round((a[2] + (b[2] - a[2]) * u) * 255);
}
/** ASPRS classes as colours (ground earth, vegetation greens, buildings red, water blue, noise grey). */
export const CLASS_COLORS = {
  0: [150, 150, 150], 1: [170, 170, 170], 2: [160, 120, 70], 3: [140, 200, 110], 4: [70, 160, 60], 5: [30, 110, 40],
  6: [215, 70, 50], 7: [110, 110, 110], 8: [240, 200, 40], 9: [50, 110, 220], 10: [120, 80, 160], 11: [90, 90, 100],
  12: [200, 200, 120], 13: [230, 150, 40], 14: [240, 120, 20], 15: [180, 60, 160], 16: [220, 160, 60], 17: [140, 140, 190], 18: [80, 80, 80],
};
/** A value below which `frac` of the values lie (sampled), so a few outliers do not wash the ramp out. */
function quantile(arr, step, offset, frac, n) {
  const pick = [];
  const every = Math.max(1, Math.floor(n / 20000));
  for (let k = 0; k < n; k += every) pick.push(arr[k * step + offset]);
  pick.sort((a, b) => a - b);
  return pick[Math.min(pick.length - 1, Math.max(0, Math.floor(frac * (pick.length - 1))))];
}

export function colorize(cloud, mode = 'rgb', { color = [200, 200, 200] } = {}) {
  const n = cloud.n, out = new Uint8Array(n * 3);
  if (mode === 'rgb' && cloud.rgb) { out.set(cloud.rgb); return out; }
  if (mode === 'single') { for (let k = 0; k < n; k++) out.set(color, k * 3); return out; }
  if (mode === 'class' && cloud.cls) {
    for (let k = 0; k < n; k++) out.set(CLASS_COLORS[cloud.cls[k]] || [200, 200, 200], k * 3);
    return out;
  }
  if (mode === 'intensity' && cloud.inten) {
    const lo = quantile(cloud.inten, 1, 0, 0.02, n), hi = quantile(cloud.inten, 1, 0, 0.98, n) || 1;
    for (let k = 0; k < n; k++) { const g = Math.round(40 + 215 * Math.max(0, Math.min(1, (cloud.inten[k] - lo) / Math.max(1, hi - lo)))); out[k * 3] = g; out[k * 3 + 1] = g; out[k * 3 + 2] = g; }
    return out;
  }
  // height (also rgb without colours, class without classes, intensity without intensity)
  const lo = quantile(cloud.pos, 3, 2, 0.01, n), hi = quantile(cloud.pos, 3, 2, 0.99, n);
  const span = hi - lo || 1;
  for (let k = 0; k < n; k++) rampAt((cloud.pos[k * 3 + 2] - lo) / span, out, k * 3);
  return out;
}
/** The modes a cloud can show (its own colours, intensity and classes only when the file has them). */
export function colorModes(cloud) {
  return ['rgb', 'height', 'intensity', 'class', 'single'].filter((m) => (m === 'rgb' ? !!cloud.rgb : m === 'intensity' ? !!cloud.inten : m === 'class' ? !!cloud.cls && cloud.classCounts.size > 1 : true));
}

// --------------------------------------------------------------------- slices, snapping
const AX = { x: 0, y: 1, z: 2 };
export function sliceOf(cloud, { axis = 'z', at, thick }) {
  const a = AX[axis] ?? 2, c = at - cloud.origin[a], h = Math.abs(thick) / 2;
  const out = [];
  for (let k = 0; k < cloud.n; k++) if (Math.abs(cloud.pos[k * 3 + a] - c) <= h) out.push(k);
  return Uint32Array.from(out);
}
export const pointAt = (cloud, i) => [cloud.pos[i * 3] + cloud.origin[0], cloud.pos[i * 3 + 1] + cloud.origin[1], cloud.pos[i * 3 + 2] + cloud.origin[2]];

/** A bucket grid in plan (x, y) of some of a cloud's points (or all), for the nearest point within a radius. */
export class PlanGrid {
  constructor(cloud, indices = null, cell = 0) {
    this.cloud = cloud;
    const idx = indices || Uint32Array.from({ length: cloud.n }, (_, k) => k);
    const span = Math.max(cloud.max[0] - cloud.min[0], cloud.max[1] - cloud.min[1], 1e-9);
    this.cell = cell > 0 ? cell : Math.max(span / 1024, span / Math.sqrt(Math.max(1, idx.length)) * 2);
    this.map = new Map();
    for (const k of idx) {
      const key = this.key(Math.floor(cloud.pos[k * 3] / this.cell), Math.floor(cloud.pos[k * 3 + 1] / this.cell));
      let b = this.map.get(key);
      if (!b) this.map.set(key, (b = []));
      b.push(k);
    }
    this.size = idx.length;
  }
  key(i, j) { return i * 1048576 + j; }
  nearest(x, y, r) {
    const c = this.cloud, lx = x - c.origin[0], ly = y - c.origin[1];
    const i0 = Math.floor((lx - r) / this.cell), i1 = Math.floor((lx + r) / this.cell);
    const j0 = Math.floor((ly - r) / this.cell), j1 = Math.floor((ly + r) / this.cell);
    if ((i1 - i0 + 1) * (j1 - j0 + 1) > 40000) return -1;   // a radius far bigger than the points: nothing sensible to snap to
    let best = -1, bd = r * r;
    for (let i = i0; i <= i1; i++) for (let j = j0; j <= j1; j++) {
      const b = this.map.get(this.key(i, j));
      if (!b) continue;
      for (const k of b) { const dx = c.pos[k * 3] - lx, dy = c.pos[k * 3 + 1] - ly, d = dx * dx + dy * dy; if (d <= bd) { bd = d; best = k; } }
    }
    return best;
  }
}

export function nearestOnScreen(cloud, indices, toScreen, sx, sy, tol) {
  let best = -1, bd = tol * tol;
  const o = cloud.origin, p = cloud.pos;
  const visit = (k) => {
    const q = toScreen(p[k * 3] + o[0], p[k * 3 + 1] + o[1], p[k * 3 + 2] + o[2]);
    if (!q) return;
    const dx = q[0] - sx, dy = q[1] - sy, d = dx * dx + dy * dy;
    if (d <= bd) { bd = d; best = k; }
  };
  if (indices) for (const k of indices) visit(k); else for (let k = 0; k < cloud.n; k++) visit(k);
  return best;
}

export function cloudSummary(cloud) {
  const classes = [...cloud.classCounts.entries()].sort((a, b) => a[0] - b[0]).map(([c, n]) => ({ c, name: className(c) || t('pc.class.other', { c }), n }));
  return { read: cloud.read, kept: cloud.n, size: [0, 1, 2].map((a) => cloud.max[a] - cloud.min[a]), classes };
}
