/* SPDX-License-Identifier: Apache-2.0 */
/**
 * model-planview.js — the plan view of one level (0.5 part 3, BM.2): a 2D canvas beside the 3D view, where the building
 * tools draw. Plan millimetres, x right, y up (model-plan.js builds what it shows). Colours from the page's tokens, so
 * paper and model space both work.
 *
 *   const pv = new PlanView(canvas, { move(pt, e), click(pt, e), dblclick(pt, e), leave(), contextmenu(pt, e) })
 *   pv.setScene({ walls, symbols, cut, below, rooms, dims })   (see below)   pv.setOverlay({ … })   pv.setSelection(keys)
 *   pv.fit()  pv.toPlan(clientX, clientY) → [x, y]  pv.toScreen([x, y]) → { x, y } (canvas px)  pv.pxToMm(px)
 *   pv.hit(clientX, clientY) → key | null      what a click would pick (walls, doors / windows, other cut elements)
 *   pv.render()
 *
 * Scene: walls [{ key, pieces: [polygon], poly }], symbols [{ key, lines: [[x1, y1, x2, y2]…], arcs: [{ c, r, a0, a1 }],
 * box: [x0, y0, x1, y1], thin: [[…]] }], cut [{ key, loops: [[x, y, …]], segs }], below [{ key, segs }], rooms [{ key,
 * poly, label: [lines], at }], dims [{ a, b, off, text }]. Overlay: walls [polygon], lines [[x1, y1, x2, y2]], dashed,
 * points [[x, y]], snap { p, kind }, hud { p, text }, symbols (as in the scene).
 *
 * Pointer: wheel zooms about the pointer; middle or right drag, or Space + drag, pans; two fingers pinch and pan; a left
 * click (≤ 4 px of movement) goes to the handlers with the plan point; a left drag with no tool pans.
 */
import { panButton, wheelFactor } from './input-mouse.js';   // 0.5 IP.1: buttons and wheel from the input profile
const TOKENS = ['--ink', '--ink-2', '--line', '--grid', '--grid-minor', '--sheet', '--sheet-2', '--action', '--poche', '--layer-blue', '--layer-red', '--layer-ochre'];


export class PlanView {
  constructor(canvas, handlers = {}) {
    this.canvas = canvas;
    this.h = handlers;
    this.ctx = canvas.getContext('2d');
    this.scene = { walls: [], symbols: [], cut: [], below: [], rooms: [], dims: [] };
    this.overlay = null;
    this.selection = new Set();
    this.hover = null;
    this.cx = 0; this.cy = 0; this.scale = 0.05;   // px per mm
    this.dragPan = false;   // a left drag pans (no tool) — the page sets it
    this.fitted = false;
    this._pointers = new Map();
    this.ro = new ResizeObserver(() => this.resize());
    this.ro.observe(canvas.parentElement);
    this.bind();
    this.resize();
  }

  resize() {
    const r = this.canvas.getBoundingClientRect(), dpr = window.devicePixelRatio || 1;
    const w = Math.max(1, Math.round(r.width * dpr)), h = Math.max(1, Math.round(r.height * dpr));
    if (this.canvas.width !== w || this.canvas.height !== h) { this.canvas.width = w; this.canvas.height = h; }
    this.dpr = dpr; this.w = r.width; this.h2 = r.height;
    if (!this.fitted && r.width > 0) this.fit();
    this.render();
  }

  // ---- coordinates ----
  toScreen([x, y]) { return { x: (x - this.cx) * this.scale + this.w / 2, y: this.h2 / 2 - (y - this.cy) * this.scale }; }
  toPlan(clientX, clientY) { const r = this.canvas.getBoundingClientRect(); return this.fromCanvas(clientX - r.left, clientY - r.top); }
  fromCanvas(px, py) { return [(px - this.w / 2) / this.scale + this.cx, (this.h2 / 2 - py) / this.scale + this.cy]; }
  pxToMm(px) { return px / this.scale; }

  setScene(s) { this.scene = { walls: [], symbols: [], cut: [], below: [], rooms: [], dims: [], ...s }; this.render(); }
  setOverlay(o) { this.overlay = o; this.render(); }
  setSelection(keys) { this.selection = new Set(keys || []); this.render(); }

  bounds() {
    const bb = { x0: Infinity, y0: Infinity, x1: -Infinity, y1: -Infinity };
    const g = (x, y) => { if (x < bb.x0) bb.x0 = x; if (x > bb.x1) bb.x1 = x; if (y < bb.y0) bb.y0 = y; if (y > bb.y1) bb.y1 = y; };
    for (const w of this.scene.walls) for (const q of w.poly || []) g(q[0], q[1]);
    for (const c of this.scene.cut) for (let i = 0; i < c.segs.length; i += 2) g(c.segs[i], c.segs[i + 1]);
    for (const c of this.scene.below) for (let i = 0; i < c.segs.length; i += 2) g(c.segs[i], c.segs[i + 1]);
    for (const s of this.scene.symbols) { if (s.box) { g(s.box[0], s.box[1]); g(s.box[2], s.box[3]); } for (const q of s.poly || []) g(q[0], q[1]); }
    return Number.isFinite(bb.x0) ? bb : null;
  }
  /** Fit the drawing (or a 12 m square round the origin when there is nothing yet). Room is left for the bars on top. */
  fit(bb = this.bounds()) {
    if (!this.w) return;
    const b = bb || { x0: -2000, y0: -2000, x1: 10000, y1: 10000 };
    const top = this.padTop ?? 0, bottom = this.padBottom ?? 0;
    const w = Math.max(1000, b.x1 - b.x0), h = Math.max(1000, b.y1 - b.y0);
    this.scale = Math.min((this.w - 60) / w, (this.h2 - 60 - top - bottom) / h);
    if (!(this.scale > 0)) this.scale = 0.05;
    this.cx = (b.x0 + b.x1) / 2;
    this.cy = (b.y0 + b.y1) / 2 + (top - bottom) / 2 / this.scale;
    this.fitted = true;
    this.render();
  }
  zoomAt(px, py, k) {
    const before = this.fromCanvas(px, py);
    this.scale = Math.min(20, Math.max(0.0005, this.scale * k));
    const after = this.fromCanvas(px, py);
    this.cx += before[0] - after[0]; this.cy += before[1] - after[1];
    this.render();
  }

  // ---- pointer ----
  bind() {
    const c = this.canvas;
    c.addEventListener('wheel', (e) => {
      e.preventDefault();
      const r = c.getBoundingClientRect();
      this.zoomAt(e.clientX - r.left, e.clientY - r.top, Math.exp(-e.deltaY * (e.deltaMode === 1 ? 0.05 : 0.0015) * (e.ctrlKey ? 1 : wheelFactor())));
    }, { passive: false });
    c.addEventListener('contextmenu', (e) => { e.preventDefault(); if (!this._panned) this.h.contextmenu?.(this.toPlan(e.clientX, e.clientY), e); });
    c.addEventListener('pointerdown', (e) => {
      c.setPointerCapture?.(e.pointerId);
      this._pointers.set(e.pointerId, { x: e.clientX, y: e.clientY });
      if (this._pointers.size === 2) { this._pinch = this.pinchState(); this._press = null; return; }
      // a handler that returns true takes the press (dragging a door), so a left drag does not pan
      const claimed = e.button === 0 && !this.space && this.h.down?.(this.toPlan(e.clientX, e.clientY), e) === true;
      // which buttons pan: the input profile's (input-mouse.js; middle and right unless changed); Space or Drag pan: left too
      const pan = (e.button !== 0 && panButton(e.button)) || (e.button === 0 && !claimed && (this.space || this.dragPan || panButton(0)));
      this._press = { x: e.clientX, y: e.clientY, button: e.button, pan, panning: false, cx: this.cx, cy: this.cy, touch: e.pointerType === 'touch', claimed };
      this._panned = false;
    });
    c.addEventListener('pointermove', (e) => {
      if (this._pointers.has(e.pointerId)) this._pointers.set(e.pointerId, { x: e.clientX, y: e.clientY });
      if (this._pinch && this._pointers.size === 2) { this.pinchMove(); return; }
      const p = this._press;
      if (p) {
        const moved = Math.hypot(e.clientX - p.x, e.clientY - p.y);
        if ((p.pan || (p.touch && this.dragPan)) && moved > 3) {
          p.panning = true; this._panned = true;
          this.cx = p.cx - (e.clientX - p.x) / this.scale; this.cy = p.cy + (e.clientY - p.y) / this.scale;
          this.render();
          return;
        }
      }
      this.h.move?.(this.toPlan(e.clientX, e.clientY), e);
    });
    const up = (e) => {
      this._pointers.delete(e.pointerId);
      if (this._pinch) { if (this._pointers.size < 2) this._pinch = null; this._press = null; return; }
      const p = this._press;
      this._press = null;
      if (!p || p.panning) return;
      const moved = Math.hypot(e.clientX - p.x, e.clientY - p.y);
      if (p.claimed) this.h.up?.(this.toPlan(e.clientX, e.clientY), e, { moved, cancel: e.type !== 'pointerup' });
      if (e.type === 'pointerup' && p.button === 0 && Math.hypot(e.clientX - p.x, e.clientY - p.y) <= (p.touch ? 10 : 4)) this.h.click?.(this.toPlan(e.clientX, e.clientY), e);
    };
    c.addEventListener('pointerup', up);
    c.addEventListener('pointercancel', up);
    c.addEventListener('dblclick', (e) => this.h.dblclick?.(this.toPlan(e.clientX, e.clientY), e));
    c.addEventListener('pointerleave', () => this.h.leave?.());
    window.addEventListener('keydown', (e) => { if (e.code === 'Space' && !e.target.closest?.('input, textarea, select, button')) { this.space = true; } });   // i18n-ignore: CSS selectors
    window.addEventListener('keyup', (e) => { if (e.code === 'Space') this.space = false; });
  }
  pinchState() {
    const [a, b] = [...this._pointers.values()];
    return { d: Math.hypot(a.x - b.x, a.y - b.y) || 1, m: { x: (a.x + b.x) / 2, y: (a.y + b.y) / 2 }, scale: this.scale, cx: this.cx, cy: this.cy };
  }
  pinchMove() {
    const [a, b] = [...this._pointers.values()], s = this._pinch;
    const d = Math.hypot(a.x - b.x, a.y - b.y) || 1, m = { x: (a.x + b.x) / 2, y: (a.y + b.y) / 2 };
    const r = this.canvas.getBoundingClientRect();
    // the plan point under the first middle stays under the new middle
    this.scale = s.scale; this.cx = s.cx; this.cy = s.cy;
    const P = this.fromCanvas(s.m.x - r.left, s.m.y - r.top);
    this.scale = Math.min(20, Math.max(0.0005, s.scale * d / s.d));
    const Q = this.fromCanvas(m.x - r.left, m.y - r.top);
    this.cx += P[0] - Q[0]; this.cy += P[1] - Q[1];
    this._panned = true;
    this.render();
  }

  // ---- picking ----
  hit(clientX, clientY, tolPx = 6) {
    const [x, y] = this.toPlan(clientX, clientY), tol = tolPx / this.scale;
    for (const s of this.scene.symbols) {
      if (s.roof) continue;   // a roof's outline covers everything under it: it picks last
      if (s.poly) { if (inPoly(x, y, s.poly) || near(x, y, s.poly, tol)) return s.key; continue; }
      const b = s.box;
      if (b && x >= Math.min(b[0], b[2]) - tol && x <= Math.max(b[0], b[2]) + tol && y >= Math.min(b[1], b[3]) - tol && y <= Math.max(b[1], b[3]) + tol) return s.key;
    }
    for (const w of this.scene.walls) if ((w.pieces || [w.poly]).some((p) => inPoly(x, y, p))) return w.key;
    for (const w of this.scene.walls) if (w.poly && near(x, y, w.poly, tol)) return w.key;
    for (const c of this.scene.cut) {
      if ((c.loops || []).some((l) => inLoop(x, y, l))) return c.key;
      for (let i = 0; i < c.segs.length; i += 4) if (segD(x, y, c.segs[i], c.segs[i + 1], c.segs[i + 2], c.segs[i + 3]) <= tol) return c.key;
    }
    for (const r of this.scene.rooms || []) if (r.poly && inPoly(x, y, r.poly)) return r.key;
    for (const s of this.scene.symbols) if (s.roof && s.poly && inPoly(x, y, s.poly)) return s.key;
    return null;
  }

  // ---- drawing ----
  colors() {
    const cs = getComputedStyle(this.canvas), o = {};
    for (const k of TOKENS) o[k] = cs.getPropertyValue(k).trim() || null;
    return o;
  }
  render() {
    if (this._raf) return;
    this._raf = requestAnimationFrame(() => { this._raf = 0; this.draw(); });
  }
  draw() {
    const g = this.ctx, C = this.colors(), dpr = this.dpr || 1;
    const ink = C['--ink'] || '#111', ink2 = C['--ink-2'] || '#666', line = C['--line'] || '#ccc', sheet = C['--sheet'] || '#fff', action = C['--action'] || '#2563eb';
    const poche = C['--poche'] || C['--line'] || '#bbb';
    g.setTransform(dpr, 0, 0, dpr, 0, 0);
    g.fillStyle = sheet; g.fillRect(0, 0, this.w, this.h2);
    this.drawGrid(g, C['--grid-minor'] || C['--grid'] || line);
    const S = (p) => this.toScreen(p);
    const path = (poly) => { g.beginPath(); poly.forEach((q, i) => { const s = S(q); if (i) g.lineTo(s.x, s.y); else g.moveTo(s.x, s.y); }); g.closePath(); };
    const flat = (l) => { g.beginPath(); for (let i = 0; i < l.length; i += 2) { const s = S([l[i], l[i + 1]]); if (i) g.lineTo(s.x, s.y); else g.moveTo(s.x, s.y); } g.closePath(); };
    const segs = (a) => { g.beginPath(); for (let i = 0; i < a.length; i += 4) { const p = S([a[i], a[i + 1]]), q = S([a[i + 2], a[i + 3]]); g.moveTo(p.x, p.y); g.lineTo(q.x, q.y); } g.stroke(); };
    const sel = (k) => this.selection.has(k), hov = (k) => this.hover === k;
    // what lies below the cut: thin
    g.lineWidth = 1; g.strokeStyle = ink2; g.globalAlpha = 0.7;
    for (const b of this.scene.below) { g.strokeStyle = sel(b.key) ? action : ink2; segs(b.segs); }
    g.globalAlpha = 1;
    // rooms: a faint fill and the label
    for (const r of this.scene.rooms || []) {
      if (r.poly) { path(r.poly); g.fillStyle = sel(r.key) ? action : (C['--layer-blue'] || action); g.globalAlpha = sel(r.key) ? 0.16 : 0.06; g.fill(); g.globalAlpha = 1; }
    }
    // cut elements from other programs (or the library): filled loops, outlines
    for (const c of this.scene.cut) {
      g.fillStyle = sel(c.key) ? action : poche; g.globalAlpha = sel(c.key) ? 0.35 : 0.8;
      if (c.loops?.length) { g.beginPath(); for (const l of c.loops) { for (let i = 0; i < l.length; i += 2) { const s = S([l[i], l[i + 1]]); if (i) g.lineTo(s.x, s.y); else g.moveTo(s.x, s.y); } g.closePath(); } g.fill('evenodd'); }
      g.globalAlpha = 1;
      g.strokeStyle = sel(c.key) || hov(c.key) ? action : ink; g.lineWidth = 1.5; segs(c.segs);
    }
    // drawn walls: poché, and a heavy outline round all of them together (wallEdges: their joins not drawn); a picked or
    // pointed-at wall gets its own outline on top
    const joint = !!this.scene.wallEdges;
    for (const w of this.scene.walls) {
      for (const p of w.pieces || [w.poly]) {
        path(p); g.fillStyle = sel(w.key) ? action : poche; g.globalAlpha = sel(w.key) ? 0.45 : 1; g.fill(); g.globalAlpha = 1;
        if (!joint) { g.strokeStyle = sel(w.key) || hov(w.key) ? action : ink; g.lineWidth = sel(w.key) ? 2.5 : 1.75; g.stroke(); }
      }
    }
    if (joint) {
      g.strokeStyle = ink; g.lineWidth = 1.75;
      for (const p of this.scene.wallEdges) { path(p); g.stroke(); }
      for (const w of this.scene.walls) if (sel(w.key) || hov(w.key)) for (const p of w.pieces || [w.poly]) { path(p); g.strokeStyle = action; g.lineWidth = sel(w.key) ? 2.5 : 1.75; g.stroke(); }
    }
    // doors and windows
    for (const s of this.scene.symbols) this.drawSymbol(g, s, sel(s.key) || hov(s.key) ? action : ink, S);
    // dimensions
    g.font = '600 11px system-ui, sans-serif';
    for (const d of this.scene.dims || []) this.drawDim(g, d, ink2, S);
    // room labels on top
    g.textAlign = 'center'; g.textBaseline = 'middle';
    for (const r of this.scene.rooms || []) {
      if (!r.at || !r.label) continue;
      const s = S(r.at);
      r.label.forEach((t, i) => { g.font = (i ? '500 11px ' : '700 12px ') + 'system-ui, sans-serif'; g.fillStyle = sel(r.key) ? action : ink; g.fillText(t, s.x, s.y + (i - (r.label.length - 1) / 2) * 15); });
    }
    this.drawOverlay(g, { ink, ink2, action, poche }, S, path);
  }
  drawGrid(g, color) {
    // a light grid: 1 m (or 1 ft) lines once they are 12 px apart, 5 m / 10 ft stronger
    const unit = this.gridUnit || 1000, minor = unit * this.scale;
    if (minor < 12) return;
    const [x0, y1] = this.fromCanvas(0, 0), [x1, y0] = this.fromCanvas(this.w, this.h2);
    g.lineWidth = 1; g.strokeStyle = color; g.globalAlpha = 0.5;
    g.beginPath();
    for (let x = Math.ceil(x0 / unit) * unit; x <= x1; x += unit) { const s = this.toScreen([x, 0]).x; g.moveTo(Math.round(s) + 0.5, 0); g.lineTo(Math.round(s) + 0.5, this.h2); }
    for (let y = Math.ceil(y0 / unit) * unit; y <= y1; y += unit) { const s = this.toScreen([0, y]).y; g.moveTo(0, Math.round(s) + 0.5); g.lineTo(this.w, Math.round(s) + 0.5); }
    g.stroke();
    g.globalAlpha = 1;
  }
  drawSymbol(g, s, color, S) {
    g.strokeStyle = color; g.lineWidth = 1.25;
    g.beginPath();
    for (const l of s.lines || []) { const p = S([l[0], l[1]]), q = S([l[2], l[3]]); g.moveTo(p.x, p.y); g.lineTo(q.x, q.y); }
    g.stroke();
    if (s.fill) { for (const p of s.fill) { g.beginPath(); p.forEach((q, i) => { const t = S(q); if (i) g.lineTo(t.x, t.y); else g.moveTo(t.x, t.y); }); g.closePath(); g.fillStyle = this.colors()['--sheet'] || '#fff'; g.fill(); g.stroke(); } }
    g.lineWidth = 0.75;
    g.beginPath();
    for (const a of s.arcs || []) {
      const c = S(a.c);
      // plan y is up, canvas y is down: angles flip
      g.moveTo(c.x + Math.cos(-a.a0) * a.r * this.scale, c.y + Math.sin(-a.a0) * a.r * this.scale);
      g.arc(c.x, c.y, a.r * this.scale, -a.a0, -a.a1, a.a1 > a.a0);
    }
    for (const l of s.thin || []) { const p = S([l[0], l[1]]), q = S([l[2], l[3]]); g.moveTo(p.x, p.y); g.lineTo(q.x, q.y); }
    g.stroke();
    if (s.dashed?.length) {
      g.setLineDash([4, 3]); g.beginPath();
      for (const l of s.dashed) { const p = S([l[0], l[1]]), q = S([l[2], l[3]]); g.moveTo(p.x, p.y); g.lineTo(q.x, q.y); }
      g.stroke(); g.setLineDash([]);
    }
  }
  /** A dimension: a and b (plan), measured along ab, drawn `off` mm to its left, with ticks and the text. */
  drawDim(g, d, color, S) {
    const L = Math.hypot(d.b[0] - d.a[0], d.b[1] - d.a[1]);
    if (L < 1) return;
    const u = [(d.b[0] - d.a[0]) / L, (d.b[1] - d.a[1]) / L], n = [-u[1], u[0]], off = d.offPx !== undefined ? d.offPx / this.scale : d.off || 0;
    const A = [d.a[0] + n[0] * off, d.a[1] + n[1] * off], B = [d.b[0] + n[0] * off, d.b[1] + n[1] * off];
    const a = S(A), b = S(B), ea = S(d.a), eb = S(d.b);
    if (Math.hypot(b.x - a.x, b.y - a.y) < 18) return;
    g.strokeStyle = color; g.fillStyle = color; g.lineWidth = 0.75;
    g.beginPath();
    g.moveTo(a.x, a.y); g.lineTo(b.x, b.y);
    // extension lines (a gap at the object), ticks
    const ext = (e, p) => { const dx = p.x - e.x, dy = p.y - e.y, l = Math.hypot(dx, dy); if (l < 1) return; g.moveTo(e.x + dx / l * 3, e.y + dy / l * 3); g.lineTo(p.x + dx / l * 4, p.y + dy / l * 4); };
    ext(ea, a); ext(eb, b);
    const sx = (b.x - a.x) / Math.hypot(b.x - a.x, b.y - a.y), sy = (b.y - a.y) / Math.hypot(b.x - a.x, b.y - a.y);
    for (const p of [a, b]) { g.moveTo(p.x - (sx - sy) * 4, p.y - (sy + sx) * 4); g.lineTo(p.x + (sx - sy) * 4, p.y + (sy + sx) * 4); }
    g.stroke();
    let ang = Math.atan2(b.y - a.y, b.x - a.x);
    if (ang > Math.PI / 2 || ang <= -Math.PI / 2) ang += Math.PI;
    g.save(); g.translate((a.x + b.x) / 2, (a.y + b.y) / 2); g.rotate(ang);
    g.textAlign = 'center'; g.textBaseline = 'bottom';
    const tw = g.measureText(d.text).width;
    if (tw + 6 < Math.hypot(b.x - a.x, b.y - a.y)) { g.fillStyle = this.colors()['--sheet'] || '#fff'; g.globalAlpha = 0.85; g.fillRect(-tw / 2 - 2, -15, tw + 4, 13); g.globalAlpha = 1; g.fillStyle = color; g.fillText(d.text, 0, -3); }
    g.restore();
  }
  drawOverlay(g, C, S, path) {
    const o = this.overlay;
    if (!o) return;
    for (const p of o.walls || []) { path(p); g.fillStyle = C.action; g.globalAlpha = 0.18; g.fill(); g.globalAlpha = 1; g.strokeStyle = C.action; g.lineWidth = 1.5; g.stroke(); }
    for (const s of o.symbols || []) this.drawSymbol(g, s, C.action, S);
    g.strokeStyle = C.action; g.lineWidth = 1;
    if (o.lines?.length) { g.beginPath(); for (const l of o.lines) { const p = S([l[0], l[1]]), q = S([l[2], l[3]]); g.moveTo(p.x, p.y); g.lineTo(q.x, q.y); } g.stroke(); }
    if (o.dashed?.length) { g.setLineDash([5, 4]); g.beginPath(); for (const l of o.dashed) { const p = S([l[0], l[1]]), q = S([l[2], l[3]]); g.moveTo(p.x, p.y); g.lineTo(q.x, q.y); } g.stroke(); g.setLineDash([]); }
    g.font = '600 11px system-ui, sans-serif';
    for (const d of o.dims || []) this.drawDim(g, d, C.action, S);
    for (const q of o.points || []) { const s = S(q); g.beginPath(); g.arc(s.x, s.y, 3, 0, Math.PI * 2); g.fillStyle = C.action; g.fill(); }
    // grips: filled squares at a wall's ends, a circle at its middle (drag to move)
    for (const gp of o.grips || []) {
      const s = S(gp.p); g.beginPath();
      if (gp.kind === 'mid') g.arc(s.x, s.y, 5, 0, Math.PI * 2); else g.rect(s.x - 5, s.y - 5, 10, 10);
      g.fillStyle = this.colors()['--sheet'] || '#fff'; g.fill(); g.lineWidth = 2; g.strokeStyle = C.action; g.stroke();
    }
    if (o.snap) {
      const s = S(o.snap.p); g.strokeStyle = C.action; g.lineWidth = 1.5; g.beginPath();
      if (o.snap.kind === 'end') g.rect(s.x - 5, s.y - 5, 10, 10);
      else if (o.snap.kind === 'mid') { g.moveTo(s.x, s.y - 6); g.lineTo(s.x + 6, s.y + 5); g.lineTo(s.x - 6, s.y + 5); g.closePath(); }
      else if (o.snap.kind === 'on') { g.moveTo(s.x - 5, s.y - 5); g.lineTo(s.x + 5, s.y + 5); g.moveTo(s.x + 5, s.y - 5); g.lineTo(s.x - 5, s.y + 5); }
      else { g.moveTo(s.x - 7, s.y); g.lineTo(s.x + 7, s.y); g.moveTo(s.x, s.y - 7); g.lineTo(s.x, s.y + 7); }
      g.stroke();
    }
    if (o.hud?.text) {
      const s = S(o.hud.p);
      g.font = '600 12px system-ui, sans-serif';
      const lines = String(o.hud.text).split('\n'), wd = Math.max(...lines.map((l) => g.measureText(l).width)) + 10, ht = lines.length * 16 + 4;
      const x = Math.min(this.w - wd - 4, s.x + 14), y = Math.min(this.h2 - ht - 4, s.y + 14);
      g.fillStyle = C.ink; g.globalAlpha = 0.85; g.fillRect(x, y, wd, ht); g.globalAlpha = 1;
      g.fillStyle = this.colors()['--sheet'] || '#fff'; g.textAlign = 'left'; g.textBaseline = 'top';
      lines.forEach((l, i) => g.fillText(l, x + 5, y + 3 + i * 16));
    }
  }
  /** The plan as a picture (print / PDF): the same drawing at a scale, on a white sheet. → canvas */
  snapshot(widthPx = 2400) {
    const bb = this.bounds() || { x0: 0, y0: 0, x1: 10000, y1: 10000 };
    const keep = { cx: this.cx, cy: this.cy, scale: this.scale, w: this.w, h2: this.h2, dpr: this.dpr, ctx: this.ctx, overlay: this.overlay, sel: this.selection, hover: this.hover };
    const pad = 0.06, W = bb.x1 - bb.x0, H = bb.y1 - bb.y0;
    const c = document.createElement('canvas');
    c.width = widthPx; c.height = Math.round(widthPx * (H + 2 * pad * W) / (W * (1 + 2 * pad)));
    this.ctx = c.getContext('2d'); this.w = c.width; this.h2 = c.height; this.dpr = 1;
    this.scale = Math.min(c.width / (W * (1 + 2 * pad)), c.height / (H + 2 * pad * W));
    this.cx = (bb.x0 + bb.x1) / 2; this.cy = (bb.y0 + bb.y1) / 2;
    this.overlay = null; this.selection = new Set(); this.hover = null;
    const grid = this.gridUnit; this.gridUnit = 1e12;
    this.draw();
    const pxPerMm = this.scale;
    this.gridUnit = grid;
    Object.assign(this, { cx: keep.cx, cy: keep.cy, scale: keep.scale, w: keep.w, h2: keep.h2, dpr: keep.dpr, ctx: keep.ctx, overlay: keep.overlay, selection: keep.sel, hover: keep.hover });
    this.render();
    return { canvas: c, pxPerMm };
  }
  destroy() { this.ro.disconnect(); }
}

function inPoly(x, y, p) {
  let inside = false;
  for (let i = 0, j = p.length - 1; i < p.length; j = i++) {
    const [xi, yi] = p[i], [xj, yj] = p[j];
    if ((yi > y) !== (yj > y) && x < (xj - xi) * (y - yi) / (yj - yi) + xi) inside = !inside;
  }
  return inside;
}
function inLoop(x, y, l) {
  let inside = false;
  for (let i = 0, j = l.length - 2; i < l.length; j = i, i += 2) {
    const xi = l[i], yi = l[i + 1], xj = l[j], yj = l[j + 1];
    if ((yi > y) !== (yj > y) && x < (xj - xi) * (y - yi) / (yj - yi) + xi) inside = !inside;
  }
  return inside;
}
function segD(x, y, x1, y1, x2, y2) {
  const dx = x2 - x1, dy = y2 - y1, l2 = dx * dx + dy * dy;
  const t = l2 ? Math.max(0, Math.min(1, ((x - x1) * dx + (y - y1) * dy) / l2)) : 0;
  return Math.hypot(x - x1 - t * dx, y - y1 - t * dy);
}
function near(x, y, poly, tol) {
  for (let i = 0; i < poly.length; i++) { const a = poly[i], b = poly[(i + 1) % poly.length]; if (segD(x, y, a[0], a[1], b[0], b[1]) <= tol) return true; }
  return false;
}
