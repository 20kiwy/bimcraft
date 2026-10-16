/* SPDX-License-Identifier: Apache-2.0 */
/**
 * model-ui.js — the building tools in BimCraft's page (0.5 part 3, track BM): the toolbar's Draw button, the plan view of
 * a level beside the 3D view (model-planview.js), the model bar (level, tools, wall type and height, typed lengths, Plan /
 * Split / 3D), the Levels panel, and the tools themselves. Markup is server-rendered in ui.inc ([data-plan-wrap],
 * [data-modelbar], [data-levels-sec]); bimcraft.js mounts this with a small host object and tells it when a model opens,
 * rebuilds or is picked. Every change goes through IfcEditor.modelChange (model-ops.js): one undo step each, the same
 * Undo / Redo / Save as everything else.
 *
 *   const m = mountModel(host)   host: { root, editor(), model(), ifc(), viewer(), rebuild(keyOrNull), select(keys),
 *                                       status(text, isError), afterEdit(), units() }
 *   m.opened()     a model was opened (or a new building made): Draw goes off, the plan starts again on its levels
 *   m.stage(which) BimCraft shows 'ifc' | 'dwg' | 'rvt' | … — the plan is for IFC models only
 *   m.rebuilding() / m.rebuilt()   around bimcraft.js rebuild(): clicks wait while the model reloads, then the plan is cut again
 *   m.picked()     the 3D view's pick changed: the plan follows
 *   m.show(on)     Draw on / off (a new building turns it on)
 *
 * Changes made while the model is reloading are queued and applied, in order, as soon as it is back (the plan draws
 * them as previews meanwhile), so quick clicking never writes into a model that is being replaced.
 */
import { t } from './i18n.js';
import { PlanView } from './model-planview.js';
import { planCut, chainLoops, viewerToPlan, wallPieces, wallDims } from './model-plan.js';
import { schedules, scheduleCsv } from './model-schedule.js';
import { planPdf } from './model-pdf.js';
import { wallOutlines, WALL_TYPES, wallType, typeThickness, typeName, typeAt } from './model-walls.js';
import { snapPoint, closesChain, loopSide, wallPath } from './model-tools.js';
import { fmtLen, parseLen, parseTyped, fmtAngle, defaultUnits, fmtArea, parsePitch, fmtPitch } from './model-units.js';
import { drawWallPath, drawWalls, changeWall, changeWalls, removeWalls, newLevel, changeLevel, snapshotModel, placeOpening, changeOpening, changeOpenings, removeElements, placeFloor, placeCeiling, placeRoof, changeDerived, loopsOf } from './model-ops.js';
import { loopAt, labelPoint, planLoops } from './model-rooms.js';
import { edgeKinds, roofGeometry, presetEdges, formatEdges, parseEdges, edgeWalls, ROOF_DEFAULTS } from './res-roof.js';   // 0.5 part 4 RC.1 (bim-residential)
import { FLOOR_KINDS } from './model-slabs.js';
import { DOOR_TYPES, WINDOW_TYPES, openingType, openingTypeName, openingSymbol, fitOffset } from './model-openings.js';
import { el } from './ui-util.js';

const CUT = 1200;   // mm above the level: where the plan cuts (4'-0")
const SNAP_PX = 12;
const UNITS_KEY = 'bimcraft.model.units';

export function mountModel(host) {
  const root = host.root;
  const $ = (s) => root.querySelector(s);
  const ui = {
    toggle: $('[data-model-toggle]'), wrap: $('[data-plan-wrap]'), canvas: $('[data-plan-canvas]'), bar: $('[data-modelbar]'),
    level: $('[data-model-level]'), tools: [...root.querySelectorAll('[data-mtool]')], wallOpts: $('[data-mopts="wall"]'),
    wallRef: $('[data-model-wallref]'), wallSide: $('[data-model-wallside]'), wallType: $('[data-model-walltype]'), wallTop: $('[data-model-walltop]'), wallH: $('[data-model-wallh]'), typed: $('[data-model-typed]'),
    layout: [...root.querySelectorAll('[data-model-layout]')], fit: $('[data-model-fit]'), units: $('[data-model-units]'),
    levelsSec: $('[data-levels-sec]'), levels: $('[data-levels]'), stageEl: root.querySelector('.bc-stage'), hint: $('[data-model-hint]'),
    typedRow: $('[data-mtyped-row]'), toolbar: $('[data-toolbar]'),
    oOpts: { door: $('[data-mopts="door"]'), window: $('[data-mopts="window"]') },
    schedSec: $('[data-msched-sec]'), sched: $('[data-msched]'), schedKind: $('[data-msched-kind]'), schedCsv: $('[data-msched-csv]'),
    dims: $('[data-model-dims]'), print: $('[data-model-print]'),
  };
  if (!ui.toggle || !ui.wrap) return null;

  const S = {
    on: false, layout: 'split', level: null, tool: 'select', busy: false, queue: [], again: false,
    data: null, outlines: new Map(), cut: null, cutFor: null, chain: null, last: null, hoverKey: null,
    wall: { reference: 'center', side: 'left', typeKey: 'ext-2x6', top: 'level', height: 2438.4 }, units: readUnits(),
    open: {
      door: { typeKey: DOOR_TYPES[0].key, width: DOOR_TYPES[0].width, height: DOOR_TYPES[0].height, hand: 'a' },
      window: { typeKey: WINDOW_TYPES[1].key, width: WINDOW_TYPES[1].width, height: WINDOW_TYPES[1].height, sill: WINDOW_TYPES[1].sill, hand: 'a' },
    },
    hover: null, drag: null,
    slab: { floor: { mode: 'room', kind: 'wood', thickness: FLOOR_KINDS[0].t }, ceiling: { mode: 'room', height: null, thickness: 12.7 }, roof: { pitch: Math.atan(6 / 12) * 180 / Math.PI, overhang: 304.8, ends: 'hip', fascia: true } },
    loops: null, dims: true, sched: 'doors',
  };
  let pv = null;
  // tools other modules add (0.5 part 4, bim-residential: res-ui.js) — see extend() at the end
  const exts = [];
  const extTool = (name) => exts.find((x) => x.tools?.includes(name)) || null;
  const ed = () => host.editor();
  const kb = () => host.model()?.keyBase || 0;

  // ---- on / off, layout ----
  function canDraw() { return !!(ed() && host.model()); }
  function show(on) {
    S.on = !!on && canDraw();
    ui.toggle.setAttribute('aria-pressed', S.on ? 'true' : 'false');
    ui.wrap.hidden = !S.on;
    ui.levelsSec.hidden = !S.on;
    if (ui.schedSec) ui.schedSec.hidden = !S.on;
    applyLayout();
    if (!S.on) { endChain(); setTool('select'); return; }
    if (!pv) {
      pv = new PlanView(ui.canvas, { move: onMove, click: onClick, dblclick: onDbl, down: onDown, up: onUp, leave: () => { S.last = null; S.hover = null; drawOverlay(); }, contextmenu: () => endChain() });
      pv.padBottom = 40;   // the status line
      window.bimcraftPlan = { view: pv, state: S, click: (p) => onClick(p, {}), move: (p) => onMove(p, {}), typed: (s) => applyTyped(s), refresh };   // for tests
    }
    refresh(true);
  }
  function applyLayout() {
    const st = ui.stageEl;
    st.classList.toggle('is-plan-split', S.on && S.layout === 'split');
    st.classList.toggle('is-plan-only', S.on && S.layout === 'plan');
    for (const b of ui.layout) b.setAttribute('aria-pressed', b.dataset.modelLayout === S.layout ? 'true' : 'false');
    requestAnimationFrame(() => { host.viewer()?.resize?.(); pv?.resize(); });
  }
  ui.toggle.addEventListener('click', () => {
    if (!canDraw()) { host.status(t('model.needs.ifc')); return; }
    show(!S.on);
  });
  for (const b of ui.layout) b.addEventListener('click', () => {
    S.layout = b.dataset.modelLayout;
    if (S.layout === '3d') { S.layout = 'split'; show(false); return; }
    applyLayout();
    requestAnimationFrame(() => fitPlan());   // the pane changed size: the whole level again
  });
  ui.fit?.addEventListener('click', () => fitPlan());
  /** Fit the plan below BimCraft's toolbar and the model bar, which sit over its top. */
  function fitPlan() {
    if (!pv) return;
    pv.resize();   // the pane may just have changed size (Split / Plan)
    const c = ui.canvas.getBoundingClientRect(), b = ui.bar.getBoundingClientRect();
    pv.padTop = Math.max(0, b.bottom - c.top) + 12;
    pv.fit();
  }
  // the model bar goes under the toolbar, however many rows it wraps to
  if (ui.toolbar) new ResizeObserver(() => ui.stageEl.style.setProperty('--bc-tb-h', ui.toolbar.offsetHeight + 'px')).observe(ui.toolbar);

  // ---- data ----
  function refresh(fitToo = false) {
    if (!S.on || !ed()) return;
    S.data = snapshotModel(ed());
    const levels = S.data.levels;
    if (!levels.length) { host.status(t('model.err.nolevels'), true); return; }
    if (!levels.some((l) => l.id === S.level)) S.level = (levels.find((l) => Math.abs(l.elevation) < 1) || levels[0]).id;
    fillLevelSelect();
    renderLevels();
    fillWallOpts();
    fillOpeningOpts();
    fillSlabOpts();
    S.loops = loopsOf(ed(), S.level);
    for (const x of exts) x.refresh?.();
    cutPlan();
    buildScene();
    if (fitToo) fitPlan();
  }
  const level = () => S.data?.levels.find((l) => l.id === S.level) || null;
  const levelAbove = () => { const L = S.data.levels, i = L.findIndex((l) => l.id === S.level); return i >= 0 ? L[i + 1] || null : null; };
  const wallsHere = () => (S.data?.walls || []).filter((w) => w.storey === S.level);
  const wallsOn = (storey) => (S.data?.walls || []).filter((w) => w.storey === storey);
  const openingsHere = () => (S.data?.openings || []).filter((o) => o.storey === S.level);

  /** Cut the rebuilt model's meshes for this level (what the tools did not draw). */
  function cutPlan() {
    const model = host.model(), ifc = host.ifc();
    if (!model || !ifc) { S.cut = null; return; }
    const tag = model.modelID + ':' + S.level + ':' + model.batches.length + ':' + model.triangles;
    if (S.cutFor === tag && S.cut) return;
    const own = ownKeys();
    const w = ed().placer();
    let M;
    try { M = viewerToPlan(ifc.api.GetCoordinationMatrix(model.modelID), storeyWorldMm(w, S.level)); } catch (e) { S.cut = null; return; }
    const c = planCut(model, { M, storey: S.level, cutZ: CUT, skip: own });
    S.cut = { cut: [...c.cut].map(([key, segs]) => ({ key, segs, loops: chainLoops(segs) })), below: [...c.below].map(([key, segs]) => ({ key, segs })), own };
    S.cutFor = tag;
  }
  function storeyWorldMm(w, storey) {
    const st = w.api.GetLine(w.mid, storey, false);
    const M = st?.ObjectPlacement ? [...w.api.GetWorldTransformMatrix(w.mid, st.ObjectPlacement.value)] : [1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1];
    M[12] *= w.unitMm; M[13] *= w.unitMm; M[14] *= w.unitMm;
    return M;
  }
  function ownKeys() {
    const k = kb(), s = new Set();
    for (const w of S.data?.walls || []) s.add(k + w.id);
    for (const o of S.data?.openings || []) s.add(k + o.id);
    for (const r of S.data?.derived?.roofs || []) for (const p of r.parts) s.add(k + p);
    for (const x of exts) x.ownKeys?.(s, k);
    return s;
  }
  /** The plan scene of the level shown (also for the sheets: api.sceneFor). */
  function makeScene() {
    const walls = wallsHere();
    S.outlines = wallOutlines(walls.map((w) => ({ key: w.guid, a: w.a, b: w.b, t: w.t })));
    const k = kb(), own = ownKeys(), ops = openingsHere();
    const byWall = new Map(walls.map((w) => [w.id, w]));
    const scene = {
      walls: walls.map((w) => { const poly = S.outlines.get(w.guid) || []; return { key: k + w.id, guid: w.guid, poly, pieces: poly.length ? wallPieces(w, poly, ops.filter((o) => o.wall === w.id), CUT) : [] }; }),
      symbols: ops.filter((o) => byWall.has(o.wall) && (!S.drag || S.drag.op.guid !== o.guid)).map((o) => ({ key: k + o.id, guid: o.guid, ...openingSymbol(o, byWall.get(o.wall)) })),
      rooms: roomsScene(), dims: [],
      cut: (S.cut?.cut || []).filter((c) => !own.has(c.key)),
      below: (S.cut?.below || []).filter((c) => !own.has(c.key)),
    };
    scene.symbols.push(...roofsScene());
    // the walls' outline as one: their joins (mitres, T ends) are not drawn, as on a drafted plan
    const pieces = scene.walls.flatMap((w) => w.pieces || []);
    if (pieces.length) { const L = planLoops(pieces); scene.wallEdges = [...L.rooms.flatMap((r) => [r.poly, ...r.holes]), ...L.outlines.map((o) => o.poly)]; }
    if (S.dims) scene.dims = wallDims(walls, S.outlines, ops, (mm) => fmtLen(mm, S.units), { exteriorOf: (w) => wallType(w.typeKey)?.exterior ?? true });
    for (const x of exts) x.scene?.(scene);
    return scene;
  }
  function buildScene() {
    if (!pv) return;
    const scene = makeScene();
    pv.gridUnit = S.units === 'mm' ? 1000 : 304.8;
    pv.setScene(scene);
    renderSchedule();
    pv.setSelection(selectedKeys());
    drawOverlay();
  }
  const selectedKeys = () => [...(host.viewer()?.selection || [])];
  /** Room labels: every space of this level drawn over its room (name, area). */
  function roomsScene() {
    const k = kb(), out = [];
    for (const sp of (S.data?.derived?.spaces || []).filter((x) => x.storey === S.level)) {
      const loop = S.loops && loopAt(S.loops.rooms, sp.seed);
      if (!loop) continue;
      out.push({ key: k + sp.id, poly: loop.poly, at: labelPoint(loop.poly), label: [sp.name || sp.number, fmtArea(loop.area, S.units)] });
    }
    return out;
  }
  /** Roofs on this level: the eave (dashed) and the ridges and hips (thin), from the walls under them. */
  function roofsScene() {
    const k = kb(), out = [];
    for (const r of (S.data?.derived?.roofs || []).filter((x) => x.storey === S.level)) {
      const wl = S.data.levels.find((l) => l.guid === r.walls);
      if (!wl) continue;
      const loops = wl.id === S.level ? S.loops : loopsOf(ed(), wl.id);
      const o = loopAt(loops.outlines, r.seed);
      const plan = o && roofGeometry(o.poly, edgeKinds(o.poly, wallsOn(wl.id), r), { thickness: r.thickness, fascia: r.fascia, soffit: r.soffit });
      if (!plan || plan.error) continue;
      const eave = plan.eave.map((p, i) => [p[0], p[1], plan.eave[(i + 1) % plan.eave.length][0], plan.eave[(i + 1) % plan.eave.length][1]]);
      out.push({ key: k + r.id, dashed: eave, thin: plan.ridge, poly: plan.eave, roof: true });
    }
    return out;
  }

  // ---- dimensions, schedules, print (BM.7) ----
  ui.dims?.addEventListener('click', () => { S.dims = !S.dims; ui.dims.setAttribute('aria-pressed', S.dims ? 'true' : 'false'); buildScene(); });
  ui.schedKind?.addEventListener('change', () => { S.sched = ui.schedKind.value; renderSchedule(); });
  function scheduleData() {
    const loops = new Map();
    for (const l of S.data?.levels || []) if ((S.data.derived?.spaces || []).some((x) => x.storey === l.id)) loops.set(l.id, l.id === S.level && S.loops ? S.loops : loopsOf(ed(), l.id));
    const all = schedules(S.data, loops, { fmtLen: (mm) => fmtLen(mm, S.units), fmtArea: (mm2) => fmtArea(mm2, S.units), t });
    for (const x of exts) Object.assign(all, x.schedules?.() || {});   // extensions' schedules (res-ui.js: the lumber cut list)
    return all;
  }
  function renderSchedule() {
    if (!ui.sched || !S.on || !S.data) return;
    const all = scheduleData(), sc = all[S.sched];
    if (!sc) return;
    // a row picks what it lists
    const k = kb();
    const xs = exts.find((x) => x.scheduleKeys && (x.schedules?.() || {})[S.sched]);
    const keysOf = (i) => {
      if (xs) return xs.scheduleKeys(S.sched, i) || [];
      if (S.sched === 'doors' || S.sched === 'windows') { const kind = S.sched === 'doors' ? 'door' : 'window'; const list = S.data.openings.filter((o) => o.kind === kind); const byRow = sc.rows[i][0]; const o = list.find((x) => x.mark === byRow); return o ? [k + o.id] : []; }
      if (S.sched === 'walls') { const name = sc.rows[i][0]; return S.data.walls.filter((w) => (wallType(w.typeKey) ? typeName(wallType(w.typeKey)) : w.typeName) === name).map((w) => k + w.id); }
      const sp = (S.data.derived?.spaces || []).find((x) => x.number === sc.rows[i][0]); return sp ? [k + sp.id] : [];
    };
    const table = el('table', {}, el('thead', {}, el('tr', {}, ...sc.head.map((h) => el('th', {}, h)))),
      el('tbody', {}, ...sc.rows.map((r, i) => { const tr = el('tr', { 'data-msched-row': String(i) }, ...r.map((c) => el('td', {}, c))); tr.addEventListener('click', () => host.select(keysOf(i))); return tr; })),
      sc.total ? el('tfoot', {}, el('tr', {}, ...sc.head.map((_, i) => el('td', {}, sc.total[i] ?? '')))) : null);
    ui.sched.replaceChildren(sc.rows.length ? table : el('p', { className: 'bc-muted' }, t('model.sch.none')));
  }
  ui.schedCsv?.addEventListener('click', () => {
    if (!S.data) return;
    const sc = scheduleData()[S.sched];
    const base = String(host.fileName?.() || 'model').replace(/\.ifc$/i, '');
    host.download(base + ' ' + t('model.sch.' + S.sched).toLowerCase() + '.csv', '\ufeff' + scheduleCsv(sc), 'text/csv');   // i18n-ignore: a file name
  });
  ui.print?.addEventListener('click', () => {
    if (!pv || !S.data) return;
    const sc = { ...pv.scene, dims: (pv.scene.dims || []).map((d) => ({ ...d, offPx: undefined })) };
    const lv = level(), when = new Date().toISOString().slice(0, 10);
    const base = String(host.fileName?.() || 'model').replace(/\.ifc$/i, '');
    const r = planPdf(sc, { title: t('model.print.titleof', { level: lv?.name || '' }), lines: [base, t('model.print.cut', { h: fmtLen(CUT, S.units) }), when], units: S.units, paper: S.units === 'mm' ? 'a4' : 'letter' });
    host.download(base + ' — ' + (lv?.name || '') + '.pdf', r.bytes, 'application/pdf');   // i18n-ignore: a file name
    host.status(t('model.print.done', { scale: r.scale.label }) + (r.missing ? ' ' + t('model.print.missing', { n: r.missing }) : ''));
  });

  // ---- the level select and the Levels panel ----
  function fillLevelSelect() {
    const L = S.data.levels;
    ui.level.replaceChildren(...L.slice().reverse().map((l) => el('option', { value: String(l.id), selected: l.id === S.level }, l.name)));
  }
  ui.level.addEventListener('change', () => { S.level = Number(ui.level.value); endChain(); S.cutFor = null; refresh(); fitPlan(); });
  function renderLevels() {
    if (!ui.levels) return;
    const L = S.data.levels;
    const row = (l) => {
      const name = el('input', { type: 'text', className: 'bc-input', value: l.name, maxLength: 120, 'aria-label': t('model.lv.name'), 'data-lv-name': l.guid });
      const elev = el('input', { type: 'text', className: 'bc-input bc-num', value: fmtLen(l.elevation, S.units), 'aria-label': t('model.lv.elev'), 'data-lv-elev': l.guid });
      const ht = el('input', { type: 'text', className: 'bc-input bc-num', value: fmtLen(l.height, S.units), 'aria-label': t('model.lv.height'), 'data-lv-height': l.guid });
      const pick = el('input', { type: 'radio', name: 'bc-plan-level', checked: l.id === S.level, 'aria-label': t('model.lv.show', { name: l.name }) });
      pick.addEventListener('change', () => { S.level = l.id; endChain(); S.cutFor = null; refresh(); fitPlan(); });
      const commitOn = (input, fn) => {
        const go = () => { const r = fn(input.value); if (r === false) input.value = input.defaultValue; };
        input.addEventListener('change', go);
        input.addEventListener('keydown', (e) => { if (e.key === 'Enter') { e.preventDefault(); input.blur(); } if (e.key === 'Escape') { input.value = input.defaultValue; input.blur(); } });
      };
      commitOn(name, (v) => { if (!v.trim()) return false; commit(changeLevel(l.guid, { name: v.trim() }), { label: l.name, verb: t('model.verb.changed') }); });
      commitOn(elev, (v) => { const mm = parseLen(v, S.units); if (!Number.isFinite(mm)) { host.status(t('model.err.length'), true); return false; } commit(changeLevel(l.guid, { elevation: mm, shiftAbove: true }), { label: l.name, verb: t('model.verb.moved') }); });
      commitOn(ht, (v) => { const mm = parseLen(v, S.units); if (!(mm >= 1000 && mm <= 100000)) { host.status(t('new.err.height'), true); return false; } commit(changeLevel(l.guid, { height: mm, shiftAbove: true }), { label: l.name, verb: t('model.verb.changed') }); });
      return el('li', { className: 'bc-level' + (l.id === S.level ? ' is-current' : ''), 'data-level': l.guid },
        el('label', { className: 'bc-level-pick' }, pick), name, elev, ht);
    };
    const add = el('button', { type: 'button', className: 'btn-line btn-sm-cc', 'data-lv-add': '' }, el('i', { className: 'bi bi-plus-lg', 'aria-hidden': 'true' }), ' ' + t('model.lv.add'));
    add.addEventListener('click', () => {
      const top = L[L.length - 1];
      commit(newLevel({ name: t('new.level', { n: L.length + 1 }), elevation: top ? top.elevation + top.height : 0, height: top ? top.height : 3000 }), { label: t('new.level', { n: L.length + 1 }), verb: t('model.verb.added') });
    });
    ui.levels.replaceChildren(
      el('div', { className: 'bc-level bc-level-head', 'aria-hidden': 'true' }, el('span', { className: 'bc-lv-e' }, t('model.lv.elev')), el('span', { className: 'bc-lv-h' }, t('model.lv.height'))),
      el('ul', { className: 'bc-levels' }, ...L.slice().reverse().map(row)), add);
  }

  // ---- units ----
  function readUnits() { try { const u = localStorage.getItem(UNITS_KEY); if (u === 'mm' || u === 'ftin') return u; } catch (e) { /* none */ } return host.units?.() || defaultUnits(); }
  if (ui.units) {
    ui.units.value = S.units;
    ui.units.addEventListener('change', () => { S.units = ui.units.value === 'mm' ? 'mm' : 'ftin'; try { localStorage.setItem(UNITS_KEY, S.units); } catch (e) { /* this tab */ } refresh(); });
  }
  const round = () => (S.units === 'mm' ? 10 : 12.7);   // free directions come out in 10 mm or 1/2"

  // ---- tools ----
  function setTool(name) {
    S.tool = name;
    for (const b of ui.tools) b.setAttribute('aria-pressed', b.dataset.mtool === name ? 'true' : 'false');
    if (ui.wallOpts) ui.wallOpts.hidden = name !== 'wall';
    for (const [k, row] of Object.entries(ui.oOpts)) if (row) row.hidden = name !== k;
    for (const k of ['floor', 'ceiling', 'roof']) { const row = root.querySelector(`[data-mopts="${k}"]`); if (row) row.hidden = name !== k; }
    for (const row of root.querySelectorAll('[data-mopts]')) if (!['wall', 'door', 'window', 'floor', 'ceiling', 'roof'].includes(row.dataset.mopts)) row.hidden = row.dataset.mopts !== name;
    const xt = extTool(name);
    S.hover = null;
    if (ui.typedRow) ui.typedRow.hidden = name === 'select';
    const typedLabel = ui.typed?.closest('label');
    if (typedLabel) typedLabel.hidden = name === 'floor' || name === 'ceiling' || name === 'roof' || (xt && !xt.typed?.(name));
    if (pv) pv.dragPan = name === 'select';
    ui.canvas.classList.toggle('is-drawing', name !== 'select');
    if (ui.hint) ui.hint.textContent = xt ? t(xt.hint?.(name) || 'res.hint.' + name) : t('model.hint.' + name);
    endChain();
    for (const x of exts) x.toolChanged?.(name);
  }
  for (const b of ui.tools) b.addEventListener('click', () => setTool(b.dataset.mtool));
  function fillWallOpts() {
    if (!ui.wallType || ui.wallType.options.length) { if (ui.wallH && document.activeElement !== ui.wallH) ui.wallH.value = fmtLen(S.wall.height, S.units); return; }
    ui.wallType.replaceChildren(...WALL_TYPES.map((x) => el('option', { value: x.key, selected: x.key === S.wall.typeKey }, typeName(x) + ' (' + fmtLen(typeThickness(x), S.units) + ')')));
    ui.wallType.addEventListener('change', () => { endChain(); S.wall.typeKey = ui.wallType.value; drawOverlay(); });
    for (const [input, key] of [[ui.wallRef, 'reference'], [ui.wallSide, 'side']]) input?.addEventListener('change', () => { endChain(); S.wall[key] = input.value; drawOverlay(); });
    ui.wallTop.addEventListener('change', () => { S.wall.top = ui.wallTop.value; ui.wallH.hidden = S.wall.top !== 'fixed'; });
    ui.wallH.addEventListener('change', () => { const mm = parseLen(ui.wallH.value, S.units); if (mm > 100) S.wall.height = mm; else host.status(t('model.err.length'), true); ui.wallH.value = fmtLen(S.wall.height, S.units); });
    ui.wallH.hidden = S.wall.top !== 'fixed';
    ui.wallH.value = fmtLen(S.wall.height, S.units);
  }

  function fillOpeningOpts() {
    for (const kind of ['door', 'window']) {
      const sel = root.querySelector(`[data-model-otype="${kind}"]`), wIn = root.querySelector(`[data-model-ow="${kind}"]`), hIn = root.querySelector(`[data-model-oh="${kind}"]`);
      if (!sel) continue;
      const o = S.open[kind], sill = kind === 'window' ? root.querySelector('[data-model-osill]') : null;
      const show = () => { if (document.activeElement !== wIn) wIn.value = fmtLen(o.width, S.units); if (document.activeElement !== hIn) hIn.value = fmtLen(o.height, S.units); if (sill && document.activeElement !== sill) sill.value = fmtLen(o.sill, S.units); };
      if (!sel.options.length) {
        sel.replaceChildren(...(kind === 'door' ? DOOR_TYPES : WINDOW_TYPES).map((x) => el('option', { value: x.key, selected: x.key === o.typeKey }, openingTypeName(x))));
        sel.addEventListener('change', () => { const ty = openingType(sel.value); Object.assign(o, { typeKey: ty.key, width: ty.width, height: ty.height }); if (kind === 'window') o.sill = ty.sill; show(); drawOverlay(); });
        const len = (input, key, min) => input.addEventListener('change', () => { const mm = parseLen(input.value, S.units); if (mm >= min) o[key] = mm; else host.status(t('model.err.length'), true); show(); drawOverlay(); });
        len(wIn, 'width', 100); len(hIn, 'height', 100); if (sill) len(sill, 'sill', 0);
      }
      show();
    }
    for (const b of root.querySelectorAll('[data-model-flip]')) if (!b.dataset.bound) { b.dataset.bound = '1'; b.addEventListener('click', () => flipHand()); }
  }
  function fillSlabOpts() {
    const fl = S.slab.floor, ce = S.slab.ceiling, rf = S.slab.roof;
    const bind = (sel, fn) => { const e = root.querySelector(sel); if (e && !e.dataset.bound) { e.dataset.bound = '1'; e.addEventListener('change', () => { fn(e); drawOverlay(); }); } return e; };
    const len = (e, key, obj, min) => { const mm = parseLen(e.value, S.units); if (mm >= min) obj[key] = mm; else host.status(t('model.err.length'), true); e.value = fmtLen(obj[key], S.units); };
    bind('[data-model-flmode="floor"]', (e) => { fl.mode = e.value; });
    bind('[data-model-flmode="ceiling"]', (e) => { ce.mode = e.value; });
    bind('[data-model-flkind]', (e) => { fl.kind = e.value; fl.thickness = FLOOR_KINDS.find((x) => x.key === e.value)?.t || fl.thickness; const th = root.querySelector('[data-model-flthick]'); if (th) th.value = fmtLen(fl.thickness, S.units); });
    const th = bind('[data-model-flthick]', (e) => len(e, 'thickness', fl, 10));
    const ch = bind('[data-model-ceheight]', (e) => len(e, 'height', ce, 500));
    const pi = bind('[data-model-rfpitch]', (e) => { const d = parsePitch(e.value); if (d > 1 && d < 80) rf.pitch = d; else host.status(t('model.err.pitch'), true); e.value = fmtPitch(rf.pitch, S.units); });
    const ov = bind('[data-model-rfover]', (e) => { const mm = parseLen(e.value, S.units); if (mm >= 0) rf.overhang = mm; else host.status(t('model.err.length'), true); e.value = fmtLen(rf.overhang, S.units); });
    bind('[data-model-rfends]', (e) => { rf.ends = e.value; });
    const fa = bind('[data-model-rffascia]', (e) => { rf.fascia = e.checked; });
    if (fa) fa.checked = rf.fascia;
    if (ce.height === null && level()) ce.height = Math.max(2000, (levelAbove() ? levelAbove().elevation - level().elevation : level().height) - 304.8);
    if (th && document.activeElement !== th) th.value = fmtLen(fl.thickness, S.units);
    if (ch && document.activeElement !== ch) ch.value = fmtLen(ce.height ?? 2438.4, S.units);
    if (pi && document.activeElement !== pi) pi.value = fmtPitch(rf.pitch, S.units);
    if (ov && document.activeElement !== ov) ov.value = fmtLen(rf.overhang, S.units);
  }
  /** The Roof tool's per-wall settings for its choice (RC.1 presets: hip, gables on the short / long sides — four-sided
   *  outlines only —, shed with its low side nearest `at`), as stored text, or null when it can't. */
  function roofEdges(poly, ends, at, storey = S.level) {
    const m = presetEdges(poly, wallsOn(storey), ends, at);
    return m ? formatEdges(m) : null;
  }
  const roofOpts = (rf) => ({ thickness: ROOF_DEFAULTS.thickness, fascia: rf.fascia ? ROOF_DEFAULTS.fascia : 0, soffit: !!rf.fascia });
  /** What the Floor, Ceiling or Roof tool would cover at p: the room or the building outline round it. */
  function hoverSlab(p, tool) {
    if (!S.loops) return null;
    const mode = tool === 'roof' ? 'building' : S.slab[tool].mode;
    const loop = loopAt(mode === 'building' ? S.loops.outlines : S.loops.rooms, p);
    return loop ? { loop, mode, seed: p } : null;
  }
  function slabPreview(h, tool, o) {
    o.walls.push(h.loop.poly);
    let text = fmtArea(Math.abs(h.loop.area), S.units);
    if (tool === 'roof') {
      const rf = S.slab.roof, g = roofEdges(h.loop.poly, rf.ends, h.seed);
      const plan = g !== null ? roofGeometry(h.loop.poly, edgeKinds(h.loop.poly, wallsHere(), { pitch: rf.pitch, overhang: rf.overhang, edges: g }), roofOpts(rf)) : { error: 'model.err.gable4' };
      if (plan.error) text = t(plan.error);
      else {
        o.symbols.push({ dashed: plan.eave.map((p, i) => [p[0], p[1], plan.eave[(i + 1) % plan.eave.length][0], plan.eave[(i + 1) % plan.eave.length][1]]), thin: plan.ridge });
        text = fmtPitch(rf.pitch, S.units) + '  ' + fmtArea(plan.area, S.units);
      }
    }
    o.hud = { p: labelPoint(h.loop.poly), text };
  }
  function placeSlab(h, tool) {
    const st = level().guid;
    if (tool === 'floor') { const f = S.slab.floor; commit(placeFloor({ storey: st, seed: h.seed, mode: f.mode, kind: f.kind, thickness: f.thickness }), { label: t('model.floor.' + f.kind), verb: t('model.verb.placed') }); }
    else if (tool === 'ceiling') { const c = S.slab.ceiling; commit(placeCeiling({ storey: st, seed: h.seed, mode: c.mode, height: c.height, thickness: c.thickness }), { label: t('model.ceiling'), verb: t('model.verb.placed') }); }
    else {
      const rf = S.slab.roof, g = roofEdges(h.loop.poly, rf.ends, h.seed);
      if (g === null) { host.status(t('model.err.gable4'), true); return; }
      commit(placeRoof({ storey: st, seed: h.seed, pitch: rf.pitch, overhang: rf.overhang, edges: g, ...roofOpts(rf) }), { label: t('model.roof'), verb: t('model.verb.placed') });
    }
  }
  function flipHand() {
    const o = S.open[S.tool];
    if (!o) return;
    o.hand = o.hand === 'b' ? 'a' : 'b';
    if (S.hover) S.hover.hand = o.hand;
    drawOverlay();
  }

  // ---- doors and windows: which wall, where along it ----
  /**
   * The wall under (or beside) plan point p and where an opening `width` wide would go, centred on p: → { wall, offset,
   * face, ok, ends: [x0, x1] (the face's ends along the wall, for the distances), s (along), side } | null.
   * Doors swing to the side the pointer is on; windows sit on the wall's outside.
   */
  function hoverOpening(p, kind, width) {
    const tol = pv.pxToMm(16);
    let best = null;
    for (const w of wallsHere()) {
      const L = w.len, u = [(w.b[0] - w.a[0]) / L, (w.b[1] - w.a[1]) / L], n = [-u[1], u[0]];
      const rx = p[0] - w.a[0], ry = p[1] - w.a[1], s = rx * u[0] + ry * u[1], d = rx * n[0] + ry * n[1];
      if (s < -tol || s > L + tol) continue;
      const score = Math.max(0, Math.abs(d) - w.t / 2);
      if (score <= tol && (!best || score < best.score)) best = { w, s, d, score };
    }
    if (!best) return null;
    const w = best.w, step = round();
    let offset = best.s - width / 2;
    if (Math.abs(best.s - w.len / 2) <= tol) offset = (w.len - width) / 2;   // the middle of the wall snaps
    else offset = Math.round(offset / step) * step;
    const fit = fitOffset(w, width, offset);
    const face = kind === 'door' ? (best.d >= 0 ? 'left' : 'right') : w.extSide;
    return { wall: w, offset: fit ?? offset, face, ok: fit !== null, ends: faceEnds(w, face), s: best.s };
  }
  /** Where the given face of a wall starts and ends along it (its joined outline), mm from the wall's start. */
  function faceEnds(w, face) {
    const poly = S.outlines.get(w.guid);
    if (!poly) return [0, w.len];
    const u = [(w.b[0] - w.a[0]) / w.len, (w.b[1] - w.a[1]) / w.len], sgn = face === 'right' ? -1 : 1;
    const xs = poly.map(([x, y]) => [(x - w.a[0]) * u[0] + (y - w.a[1]) * u[1], -(x - w.a[0]) * u[1] + (y - w.a[1]) * u[0]]).filter(([, y]) => Math.abs(y - sgn * w.t / 2) < 0.5).map(([x]) => x);
    return xs.length >= 2 ? [Math.min(...xs), Math.max(...xs)] : [0, w.len];
  }
  /** The preview: the symbol where it would go and its distances to both ends of that face. */
  function openingPreview(h, spec, o) {
    const w = h.wall, op = { kind: spec.kind, typeKey: spec.typeKey, width: spec.width, height: spec.height, sill: spec.sill || 0, hand: spec.hand, face: h.face, offset: h.offset };
    o.symbols.push(openingSymbol(op, w));
    const u = [(w.b[0] - w.a[0]) / w.len, (w.b[1] - w.a[1]) / w.len], n = [-u[1], u[0]], sgn = h.face === 'right' ? -1 : 1;
    const P = (x) => [w.a[0] + u[0] * x + n[0] * sgn * w.t / 2, w.a[1] + u[1] * x + n[1] * sgn * w.t / 2];
    const off = sgn * pv.pxToMm(22);
    const [e0, e1] = h.ends;
    if (h.offset - e0 > 0.5) o.dims.push({ a: P(e0), b: P(h.offset), off, text: fmtLen(h.offset - e0, S.units) });
    if (e1 - h.offset - op.width > 0.5) o.dims.push({ a: P(h.offset + op.width), b: P(e1), off, text: fmtLen(e1 - h.offset - op.width, S.units) });
    o.hud = { p: P(h.offset + op.width / 2), text: h.ok ? openingTypeName(openingType(spec.typeKey)) + '  ' + fmtLen(op.width, S.units) + ' × ' + fmtLen(op.height, S.units) : t('model.err.nofit') };
  }
  function placeHere(h, kind) {
    const o = S.open[kind];
    if (!h.ok) { host.status(t('model.err.nofit'), true); return; }
    const type = openingType(o.typeKey);
    commit(placeOpening({ wall: h.wall.guid, typeKey: o.typeKey, offset: h.offset, width: o.width, height: o.height, sill: kind === 'window' ? o.sill : 0, hand: o.hand, face: h.face }),
      { label: openingTypeName(type), verb: t('model.verb.placed') });
  }
  // dragging a door or window along its wall (or onto another one) with Select
  /** The picked drawn wall, when exactly one is picked (its grips show). */
  function gripWall() {
    const keys = selectedKeys();
    if (keys.length !== 1) return null;
    return wallsHere().find((w) => kb() + w.id === keys[0]) || null;
  }
  const gripsOf = (w) => [{ p: w.a, end: 'a' }, { p: w.b, end: 'b' }, { p: [(w.a[0] + w.b[0]) / 2, (w.a[1] + w.b[1]) / 2], end: 'mid' }];
  function onDown(p, e) {
    if (S.tool !== 'select' || !pv) return false;
    const gw = gripWall();
    if (gw) {
      const tol = pv.pxToMm(9);
      const g = gripsOf(gw).find((x) => Math.hypot(p[0] - x.p[0], p[1] - x.p[1]) <= tol);
      if (g) { S.drag = { kind: 'grip', wall: gw, end: g.end, from: p, key: kb() + gw.id }; return true; }
    }
    const k = pv.hit(e.clientX, e.clientY);
    const o = k !== null ? (S.data?.openings || []).find((x) => kb() + x.id === k) : null;
    if (!o) return false;
    S.drag = { kind: 'opening', op: o, key: k, from: p, moved: false };
    return true;
  }
  /** Where a wall grip dragged to `p` puts the wall: { a, b } (snapped; a middle grip moves the whole wall). */
  function gripMove(d, p, e) {
    const w = d.wall;
    const others = { walls: wallsHere().filter((x) => x.guid !== w.guid).map((x) => ({ a: x.a, b: x.b, t: x.t })), points: [] };
    if (d.end === 'mid') {
      const s0 = snapPoint(p, others, { tol: pv.pxToMm(SNAP_PX), from: d.from, ortho: !!e?.shiftKey, step: 90, round: round() });
      const dx = s0.p[0] - d.from[0], dy = s0.p[1] - d.from[1];
      return { a: [w.a[0] + dx, w.a[1] + dy], b: [w.b[0] + dx, w.b[1] + dy], snap: s0 };
    }
    const fixed = d.end === 'a' ? w.b : w.a;
    const s1 = snapPoint(p, others, { tol: pv.pxToMm(SNAP_PX), from: fixed, ortho: !!e?.shiftKey, step: 15, round: round() });
    return d.end === 'a' ? { a: s1.p, b: w.b, snap: s1 } : { a: w.a, b: s1.p, snap: s1 };
  }
  function onUp(p, e, { moved, cancel }) {
    const d = S.drag;
    S.drag = null;
    if (!d) return;
    if (d.kind === 'grip') {
      const m = d.at;
      if (cancel || moved <= 4 || !m) { drawOverlay(); return; }
      if (Math.hypot(m.b[0] - m.a[0], m.b[1] - m.a[1]) < 10) { host.status(t('model.err.short'), true); drawOverlay(); return; }
      commit(changeWall(d.wall.guid, d.end === 'a' ? { a: m.a } : d.end === 'b' ? { b: m.b } : { a: m.a, b: m.b }), { label: t('model.wall'), verb: t('model.verb.moved'), select: [d.key] });
      return;
    }
    if (cancel || moved <= 4 || !S.hover) { S.hover = null; buildScene(); return; }
    const h = S.hover;
    S.hover = null;
    if (!h.ok) { host.status(t('model.err.nofit'), true); buildScene(); return; }
    const ch = { offset: h.offset };
    if (h.wall.guid !== d.op.wallGuid) { ch.wall = h.wall.guid; if (d.op.kind === 'door') ch.face = h.face; }
    host.select([d.key]);
    commit(changeOpening(d.op.guid, ch), { label: openingTypeName(openingType(d.op.typeKey)), verb: t('model.verb.moved'), select: [d.key] });
  }

  // ---- drawing walls ----
  function snapCtx() {
    const walls = wallsHere().map((w) => ({ a: w.a, b: w.b, t: w.t }));
    for (const q of S.queuedSegs()) walls.push({ a: q.a, b: q.b, t: q.t });
    return { walls, points: S.chain ? S.chain.points : [] };
  }
  S.queuedSegs = () => S.queue.flatMap(([, meta]) => meta.preview || []);
  function snapAt(p, e) {
    const from = S.chain?.points[S.chain.points.length - 1] || null;
    return snapPoint(p, snapCtx(), { tol: pv.pxToMm(SNAP_PX), from, ortho: !!e?.shiftKey, step: 15, round: from ? round() : 0, align: S.tool === 'wall' });
  }
  function onMove(p, e) {
    if (S.drag?.kind === 'grip') { S.drag.at = gripMove(S.drag, p, e); drawOverlay(); return; }
    if (S.drag) {
      const o = S.drag.op;
      S.hover = hoverOpening(p, o.kind, o.width);
      if (S.hover && S.hover.wall.guid === o.wallGuid) S.hover.face = o.face;
      if (!S.drag.moved) { S.drag.moved = true; buildScene(); }   // the symbol leaves its place while it is dragged
      drawOverlay();
      return;
    }
    if (extTool(S.tool)) { extTool(S.tool).move?.(p, e); drawOverlay(); return; }
    if (S.tool === 'door' || S.tool === 'window') { S.last = { raw: p }; S.hover = hoverOpening(p, S.tool, S.open[S.tool].width); drawOverlay(); return; }
    if (S.tool === 'floor' || S.tool === 'ceiling' || S.tool === 'roof') { S.hover = hoverSlab(p, S.tool); drawOverlay(); return; }
    if (S.tool === 'select') { const k = pv.hit(e.clientX ?? 0, e.clientY ?? 0); if (k !== pv.hover) { pv.hover = k; pv.render(); } return; }
    S.last = { raw: p, snap: snapAt(p, e), shift: !!e?.shiftKey };
    drawOverlay();
  }
  function onClick(p, e) {
    if (!canDraw()) return;
    if (S.tool === 'select') {
      const k = e && e.clientX !== undefined ? pv.hit(e.clientX, e.clientY) : null;
      const add = e && (e.ctrlKey || e.metaKey || e.shiftKey);
      if (k === null) { if (!add) host.select([]); return; }
      const cur = new Set(selectedKeys());
      if (add) { if (cur.has(k)) cur.delete(k); else cur.add(k); host.select([...cur]); } else host.select([k]);
      return;
    }
    if (extTool(S.tool)) { extTool(S.tool).click?.(p, e); drawOverlay(); return; }
    if (S.tool === 'floor' || S.tool === 'ceiling' || S.tool === 'roof') {
      const h = hoverSlab(p, S.tool);
      if (!h) { host.status(t(S.tool === 'roof' || S.slab[S.tool]?.mode === 'building' ? 'model.err.nobuilding2' : 'model.err.noroom'), true); return; }
      placeSlab(h, S.tool);
      return;
    }
    if (S.tool === 'door' || S.tool === 'window') {
      const h = hoverOpening(p, S.tool, S.open[S.tool].width);
      if (!h) { host.status(t('model.hint.' + S.tool)); return; }
      placeHere(h, S.tool);
      return;
    }
    const s = snapAt(p, e);
    if (S.tool === 'wall') wallPoint(s.p);
  }
  function onDbl(p, e) { if (S.tool === 'wall') endChain(); else extTool(S.tool)?.dbl?.(p, e); }
  function wallPoint(q) {
    if (!S.chain) { S.chain = { points: [q], guids: [] }; drawOverlay(); return; }
    const pts = S.chain.points, a = pts[pts.length - 1];
    if (Math.hypot(q[0] - a[0], q[1] - a[1]) < 1) return;
    const closing = closesChain(pts, q, 1);
    const b = closing ? pts[0] : q;
    const type = wallType(S.wall.typeKey);
    const above = levelAbove();
    const top = S.wall.top === 'level' && above ? { topLevel: above.guid } : { height: S.wall.top === 'fixed' ? S.wall.height : level()?.height || 2438.4 };
    const seg = { a, b, t: typeThickness(type) };
    // closing a room: every wall of the chain gets its outside to the outside of the loop
    const side = S.wall.reference === 'center' ? (closing ? loopSide(pts) : 'left') : S.wall.side;
    const flips = closing ? S.chain.guids.slice() : [];
    pts.push(b);
    const run = S.wall.reference === 'center'
      ? drawWalls({ storey: level().guid, segs: [{ a, b }], typeKey: S.wall.typeKey, extSide: side, ...top })
      : drawWallPath({ storey: level().guid, points: pts.slice(), guids: S.chain.guids, typeKey: S.wall.typeKey, thickness: seg.t, reference: S.wall.reference, extSide: side, ...top });
    const op = closing && flips.length && S.wall.reference === 'center' ? chainClose(run, flips, side) : run;
    commit(op, { label: typeName(type), verb: t('model.verb.drew'), preview: S.wall.reference === 'center' ? [seg] : wallPath([a, b], seg.t, S.wall.reference, side), chain: S.chain });
    if (closing) { endChain(); host.status(t('model.room.closed', { n: flips.length + 1 })); }
    drawOverlay();
  }
  /** The closing wall and the outside of every wall in the chain, as one change. */
  function chainClose(run, guids, side) {
    return (editor) => {
      const r = run(editor);
      const r2 = changeWalls(guids, { extSide: side })(editor);
      return { ...r, journal: mergeJournals(r.journal, r2.journal) };
    };
  }
  function endChain() { if (S.chain) { S.chain = null; drawOverlay(); } }

  function applyTyped(text) {
    if (S.tool === 'select') return false;
    if (extTool(S.tool)) return !!extTool(S.tool).typed?.(S.tool, text);
    if (S.tool === 'door' || S.tool === 'window') {
      // a distance from the nearer end of the wall face the preview shows
      const d = parseLen(text, S.units), h = S.hover, o = S.open[S.tool];
      if (!h || !Number.isFinite(d) || d < 0) { host.status(t(h ? 'model.err.length' : 'model.typed.wall'), true); return false; }
      const [e0, e1] = h.ends, nearStart = h.s < (e0 + e1) / 2;
      const offset = nearStart ? e0 + d : e1 - d - o.width, fit = fitOffset(h.wall, o.width, offset);
      if (fit === null || Math.abs(fit - offset) > 0.5) { host.status(t('model.err.nofit'), true); return false; }
      placeHere({ ...h, offset: fit, ok: true }, S.tool);
      return true;
    }
    const v = parseTyped(text, S.units);
    if (!v) { host.status(t('model.err.typed'), true); return false; }
    const pts = S.chain?.points;
    const last = pts ? pts[pts.length - 1] : null;
    let q = null;
    if (v.kind === 'abs') q = [v.x, v.y];
    else if (!last) { host.status(t('model.typed.first'), true); return false; }
    else if (v.kind === 'rel') q = [last[0] + v.dx, last[1] + v.dy];
    else if (v.kind === 'polar') q = [last[0] + v.len * Math.cos(v.angle * Math.PI / 180), last[1] + v.len * Math.sin(v.angle * Math.PI / 180)];
    else {
      // a length along the direction the pointer shows (its snapped direction), else along X
      const to = S.last?.snap?.p || [last[0] + 1, last[1]];
      const d = Math.hypot(to[0] - last[0], to[1] - last[1]) || 1;
      q = [last[0] + (to[0] - last[0]) / d * v.len, last[1] + (to[1] - last[1]) / d * v.len];
    }
    if (S.tool === 'wall') wallPoint(q);
    return true;
  }
  ui.typed.addEventListener('keydown', (e) => {
    if (e.key === 'Enter') { e.preventDefault(); if (applyTyped(ui.typed.value)) ui.typed.value = ''; }
    else if (e.key === 'Escape') { e.preventDefault(); ui.typed.value = ''; ui.typed.blur(); endChain(); }
  });

  function drawOverlay() {
    if (!pv) return;
    const o = { walls: [], lines: [], dashed: [], points: [], dims: [], symbols: [] };
    for (const q of S.queuedSegs()) o.walls.push(...[outlineOf(q)]);
    const lst = S.last;
    if (S.tool === 'wall' && lst?.snap) { o.snap = lst.snap; o.dashed.push(...(lst.snap.guides || [])); }
    if (S.tool === 'wall' && S.chain) {
      const pts = S.chain.points, a = pts[pts.length - 1];
      for (const q of pts) o.points.push(q);
      if (lst) {
        const b = lst.snap.p, L = Math.hypot(b[0] - a[0], b[1] - a[1]);
        if (L >= 1) {
          const type = wallType(S.wall.typeKey), seg = { a, b, t: typeThickness(type) };
          const path = wallPath([...pts, b], seg.t, S.wall.reference, S.wall.side);
          o.walls.push(previewOutline(path.at(-1) || seg));
          const ang = Math.atan2(b[1] - a[1], b[0] - a[0]) * 180 / Math.PI;
          o.hud = { p: b, text: fmtLen(L, S.units) + '  ' + fmtAngle(ang) + (closesChain(pts, b, 1) ? '\n' + t('model.hud.close') : '') };
          o.dims.push({ a, b, off: seg.t / 2 + pv.pxToMm(18), text: fmtLen(L, S.units) });
        }
      }
    } else if (S.tool === 'wall' && lst) o.hud = { p: lst.snap.p, text: t('model.hud.start') };
    if (S.hover && (S.tool === 'door' || S.tool === 'window')) openingPreview(S.hover, { kind: S.tool, ...S.open[S.tool] }, o);
    if (S.hover && (S.tool === 'floor' || S.tool === 'ceiling' || S.tool === 'roof')) slabPreview(S.hover, S.tool, o);
    if (S.hover && S.drag?.kind === 'opening') openingPreview(S.hover, { ...S.drag.op }, o);
    // the picked wall's grips; while one is dragged, the wall where it would go (joined to the others) and its length
    const gw = S.tool === 'select' ? gripWall() : null;
    if (gw) o.grips = gripsOf(gw).map((g) => ({ p: g.p, kind: g.end === 'mid' ? 'mid' : 'end' }));
    if (S.drag?.kind === 'grip' && S.drag.at) {
      const m = S.drag.at, w = S.drag.wall;
      const walls = wallsHere().filter((x) => x.guid !== w.guid).map((x) => ({ key: x.guid, a: x.a, b: x.b, t: x.t }));
      walls.push({ key: '*', a: m.a, b: m.b, t: w.t });
      const poly = wallOutlines(walls).get('*');
      if (poly) o.walls.push(poly);
      const L = Math.hypot(m.b[0] - m.a[0], m.b[1] - m.a[1]);
      o.dims.push({ a: m.a, b: m.b, off: w.t / 2 + pv.pxToMm(18), text: fmtLen(L, S.units) });
      if (m.snap && m.snap.kind !== 'free') o.snap = { p: S.drag.end === 'mid' ? m.snap.p : (S.drag.end === 'a' ? m.a : m.b), kind: m.snap.kind };
    }
    for (const x of exts) x.overlay?.(o);
    if (S.hiEdge) o.lines.push(S.hiEdge);   // a roof's wall row hovered in Properties (RC.1)
    pv.setOverlay(o);
  }
  /** The preview wall joined to the walls already there (so a corner looks as it will). */
  function previewOutline(seg) {
    const walls = wallsHere().map((w) => ({ key: w.guid, a: w.a, b: w.b, t: w.t }));
    S.queuedSegs().forEach((q, i) => walls.push({ key: 'q' + i, a: q.a, b: q.b, t: q.t }));
    walls.push({ key: '*', ...seg });
    return wallOutlines(walls).get('*') || outlineOf(seg);
  }
  function outlineOf(seg) {
    const L = Math.hypot(seg.b[0] - seg.a[0], seg.b[1] - seg.a[1]) || 1, n = [-(seg.b[1] - seg.a[1]) / L * seg.t / 2, (seg.b[0] - seg.a[0]) / L * seg.t / 2];
    return [[seg.a[0] - n[0], seg.a[1] - n[1]], [seg.b[0] - n[0], seg.b[1] - n[1]], [seg.b[0] + n[0], seg.b[1] + n[1]], [seg.a[0] + n[0], seg.a[1] + n[1]]];
  }

  // ---- changes: applied now, or queued while the model reloads ----
  function commit(run, meta = {}) {
    if (S.busy || (S.reloading && Date.now() - S.reloadT < 30000)) { S.queue.push([run, meta]); drawOverlay(); return; }
    apply(run, meta);
  }
  function apply(run, meta) {
    let r;
    try { r = ed().modelChange(run, { label: meta.label || '', verb: meta.verb || '' }); }
    catch (e) {
      console.error(e);
      const k = e.key || e.message;
      host.status(k && /^model\./.test(k) ? t(k) : t('model.err.failed', { why: e.message || e }), true);
      if (meta.chain && S.chain === meta.chain) S.chain.points.pop();
      refresh(); return;
    }
    if (meta.chain && r.made) for (const g of r.made.keys()) meta.chain.guids.push(g);
    S.keepSel = meta.select || selectedKeys();   // the reload clears the pick: put it back afterwards
    S.reselect = meta.reselect || null;           // …or ask again after it (a roof's parts get new ids: pick it by its GlobalId)
    S.data = snapshotModel(ed());
    S.loops = loopsOf(ed(), S.level);
    for (const x of exts) x.refresh?.();
    renderLevels(); fillLevelSelect();
    buildScene();
    host.afterEdit();
    scheduleRebuild(r.id != null ? kb() + r.id : null);
  }
  async function scheduleRebuild(selectKey) {
    if (S.busy) { S.again = true; return; }
    S.busy = true;
    try { await host.rebuild(null); } catch (e) { console.error(e); host.status(t('model.err.failed', { why: e.message || e }), true); } finally { S.busy = false; S.reloading = false; }
    // what was clicked meanwhile, then once more if it changed anything
    const q = S.queue.splice(0);
    for (const [run, meta] of q) apply0(run, meta);
    if (q.length || S.again) { S.again = false; scheduleRebuild(null); }
  }
  function apply0(run, meta) {
    try {
      const r = ed().modelChange(run, { label: meta.label || '', verb: meta.verb || '' });
      if (meta.chain && r.made) for (const g of r.made.keys()) meta.chain.guids.push(g);
    } catch (e) { console.error(e); host.status(t('model.err.failed', { why: e.message || e }), true); }
    host.afterEdit();
  }

  // ---- keys (capture: the plan's own keys win while Draw is on) ----
  document.addEventListener('keydown', (e) => {
    if (!S.on || e.defaultPrevented || e.altKey || e.ctrlKey || e.metaKey) return;
    const tgt = e.target;
    if (tgt.closest?.('dialog, [data-help-panel], [data-app-settings]') || document.querySelector('dialog[open]')) return;   // i18n-ignore: CSS selectors
    if (tgt === ui.typed) return;
    const inField = tgt.closest?.('input, textarea, select, [contenteditable="true"]');
    if (inField) return;
    const k = e.key;
    if (k === 'Escape') {
      if (exts.some((x) => x.escape?.())) { e.preventDefault(); e.stopImmediatePropagation(); drawOverlay(); return; }
      if (S.chain) { e.preventDefault(); e.stopImmediatePropagation(); endChain(); host.status(t('model.chain.ended')); return; }
      if (S.tool !== 'select') { e.preventDefault(); e.stopImmediatePropagation(); setTool('select'); return; }
      return;
    }
    if (S.tool !== 'select' && /^[0-9@'"<.,\-\/]$/.test(k)) { e.preventDefault(); e.stopImmediatePropagation(); ui.typed.focus(); ui.typed.value += k; return; }
    if (k === 'Enter' && S.chain) { e.preventDefault(); e.stopImmediatePropagation(); endChain(); return; }
    const low = k.toLowerCase();
    if (exts.some((x) => x.key?.(e, low))) { e.preventDefault(); e.stopImmediatePropagation(); return; }
    if (low === 'w' && !e.shiftKey) { e.preventDefault(); e.stopImmediatePropagation(); setTool('wall'); return; }
    if (low === 'v' && !e.shiftKey) { e.preventDefault(); e.stopImmediatePropagation(); setTool('select'); return; }
    if (low === 'd' && !e.shiftKey) { e.preventDefault(); e.stopImmediatePropagation(); setTool('door'); return; }
    if (low === 'n' && !e.shiftKey) { e.preventDefault(); e.stopImmediatePropagation(); setTool('window'); return; }
    if (low === 'h' && !e.shiftKey && (S.tool === 'door' || S.tool === 'window')) { e.preventDefault(); e.stopImmediatePropagation(); flipHand(); return; }
    if (low === 'f' && !e.shiftKey && overPlan) { e.preventDefault(); e.stopImmediatePropagation(); fitPlan(); return; }
    if (k === 'Delete' || k === 'Backspace') {
      const p = picked();
      if (p.count && p.count === selectedKeys().length) { e.preventDefault(); e.stopImmediatePropagation(); deletePicked(p); }
    }
  }, true);
  let overPlan = false;
  ui.canvas.addEventListener('pointerenter', () => { overPlan = true; });
  ui.canvas.addEventListener('pointerleave', () => { overPlan = false; });
  function pickedWalls() {
    const k = kb(), byKey = new Map((S.data?.walls || []).map((w) => [k + w.id, w.guid]));
    return selectedKeys().map((x) => byKey.get(x)).filter(Boolean);
  }
  /** Delete what is picked of the tools' own elements in one change. Rooms come from the walls: they can't be deleted. */
  function deletePicked(p) {
    if (p.derived.some((d) => d.kind === 'room')) { host.status(t('model.room.nodelete'), true); return; }
    host.select([]);
    const base = removeElements({ walls: p.walls, openings: p.openings, derived: p.derived.map((d) => d.guid) });
    const extOps = [...new Set(p.ext.map((q) => q.x))].map((x) => x.remove(p.ext.filter((q) => q.x === x).map((q) => q.it)));
    const run = !extOps.length ? base : (editor) => { let r = (p.walls.length || p.openings.length || p.derived.length) ? base(editor) : null; for (const o of extOps) { const r2 = o(editor); r = r ? { ...r2, journal: mergeJournals(r.journal, r2.journal) } : r2; } return r; };
    commit(run, { label: p.count === 1 ? p.label : t('model.elements', { n: p.count }), verb: t('model.verb.deleted') });
  }
  /** The tools' own elements among the picked: walls, doors and windows (GlobalIds), how many, a label for one. */
  function picked() {
    const k = kb(), walls = [], openings = [], derived = [];
    const keys = selectedKeys();
    if (!keys.length || !ed()) return { walls, openings, derived: [], ext: [], count: 0, label: '' };
    // cheap first: none of the picked is a wall, door or window → nothing of ours (no reading of the model)
    const cls = host.model()?.classOf;
    if (cls && !keys.some((x) => /^Ifc(Wall|WallStandardCase|Door|Window|Space|Slab|Covering|Roof)$/.test(cls.get(x - k) || '') || exts.some((e) => e.classes?.test(cls.get(x - k) || '')) || !cls.has(x - k))) return { walls, openings, derived: [], ext: [], count: 0, label: '' };
    const data = S.on && S.data ? S.data : snapshotModel(ed());   // with Draw off the plan's data may be old
    let label = '';
    const ext = [];
    for (const key of keys) {
      const xi = exts.map((x) => ({ x, it: x.pick?.(key) })).find((q) => q.it);
      if (xi) { if (!ext.some((q) => q.it.guid === xi.it.guid)) ext.push(xi); label = xi.it.label; continue; }
      const w = (data.walls || []).find((x) => k + x.id === key);
      if (w) { walls.push(w.guid); label = t('model.wall'); continue; }
      const o = (data.openings || []).find((x) => k + x.id === key);
      if (o) { openings.push(o.guid); label = openingTypeName(openingType(o.typeKey) || { key: o.typeKey }); continue; }
      const D = data.derived || {};
      const sp = (D.spaces || []).find((x) => k + x.id === key);
      if (sp) { derived.push({ kind: 'room', ...sp }); label = sp.name || sp.number; continue; }
      const fl = (D.floors || []).find((x) => k + x.id === key);
      if (fl) { derived.push({ kind: 'floor', ...fl }); label = t('model.floor.' + fl.kind); continue; }
      const ce = (D.ceilings || []).find((x) => k + x.id === key);
      if (ce) { derived.push({ kind: 'ceiling', ...ce }); label = t('model.ceiling'); continue; }
      const rf = (D.roofs || []).find((x) => k + x.id === key || x.parts.some((p) => k + p === key));
      if (rf) { if (!derived.some((d) => d.guid === rf.guid)) derived.push({ kind: 'roof', ...rf }); label = t('model.roof'); }
    }
    return { walls, openings, derived, ext, count: walls.length + openings.length + derived.length + ext.length, label, picks: keys.length };
  }

  // ---- Properties: change a drawn wall, door or window (BM.5) ----
  /**
   * The block Properties shows above the IFC data when the pick is the tools' own: one wall, one door / window, or several
   * of one kind (then only what they share: type, height, size). Every field is one change on Enter or leaving it.
   */
  function propsBlock() {
    const keys = selectedKeys();
    const p = picked();
    if (!p.count || p.count !== keys.length) return null;
    const data = S.on && S.data ? S.data : snapshotModel(ed());
    const k = kb();
    const walls = data.walls.filter((w) => p.walls.includes(w.guid)), ops = data.openings.filter((o) => p.openings.includes(o.guid));
    const levels = data.levels;
    const L = (mm) => fmtLen(mm, S.units);
    const rows = [];
    const field = (label, input, extra) => rows.push(el('dt', {}, label), el('dd', {}, input, extra || null));
    const lenInput = (value, attr, onLen, { min = 1, allowZero = false, any = false } = {}) => {
      const i = el('input', { type: 'text', className: 'bc-input bc-num', value: Number.isFinite(value) ? L(value) : '', [attr]: '' });
      const go = () => {
        if (i.value === i.defaultValue) return;
        const mm = parseLen(i.value, S.units);
        if (!Number.isFinite(mm) || (!any && (allowZero ? mm < 0 : mm < min))) { host.status(t('model.err.length'), true); i.value = i.defaultValue; return; }
        onLen(mm);
      };
      i.addEventListener('change', go);
      i.addEventListener('keydown', (e) => { if (e.key === 'Enter') { e.preventDefault(); i.blur(); } if (e.key === 'Escape') { i.value = i.defaultValue; i.blur(); } });
      return i;
    };
    const select = (options, value, attr, onPick) => {
      const s2 = el('select', { className: 'bc-mb-select', [attr]: '' }, ...options.map(([v, label]) => el('option', { value: v, selected: v === value }, label)));
      s2.addEventListener('change', () => onPick(s2.value));
      return s2;
    };
    const button = (label, attr, onClick) => { const b = el('button', { type: 'button', className: 'btn-line btn-sm-cc', [attr]: '' }, label); b.addEventListener('click', onClick); return b; };
    const one = (label) => ({ label, verb: t('model.verb.changed'), select: keys });
    let title = '';
    if (p.ext.length) {
      // another module's elements (all of one module): it fills the rows
      const xs = new Set(p.ext.map((q) => q.x));
      if (xs.size !== 1 || p.ext.length !== p.count) return null;
      const r = p.ext[0].x.props?.(p.ext.map((q) => q.it), { rows, field, lenInput, select, button, one, keys, el });
      if (!r) return null;
      title = r.title;
    } else if (walls.length && !ops.length) {
      const w0 = walls[0], many = walls.length > 1, guids = walls.map((w) => w.guid);
      title = many ? t('model.walls', { n: walls.length }) : t('model.wall');
      const sameType = walls.every((w) => w.typeKey === w0.typeKey);
      const typeOpts = WALL_TYPES.map((x) => [x.key, typeName(x)]);
      if (sameType && !WALL_TYPES.some((x) => x.key === w0.typeKey) && wallType(w0.typeKey)) typeOpts.unshift([w0.typeKey, typeName(wallType(w0.typeKey))]);
      if (!sameType) typeOpts.unshift(['', t('model.prop.changetype')]);
      field(t('model.wall.type'), select(typeOpts, sameType ? w0.typeKey : '', 'data-mp-type', (v) => { if (v) commit(many ? changeWalls(guids, { typeKey: v }) : changeWall(w0.guid, { typeKey: v }), one(title)); }));
      const above = (w) => { const i = levels.findIndex((l) => l.id === w.storey); return i >= 0 ? levels[i + 1] || null : null; };
      if (!many) {
        field(t('model.prop.thickness'), lenInput(w0.t, 'data-mp-thick', (mm) => { const key = typeAt(w0.typeKey, mm); if (!key) { host.status(t('model.err.thin'), true); return; } commit(changeWall(w0.guid, { typeKey: key }), one(title)); }, { min: 10 }));
        field(t('model.prop.length'), lenInput(w0.len, 'data-mp-len', (mm) => { const u = [(w0.b[0] - w0.a[0]) / w0.len, (w0.b[1] - w0.a[1]) / w0.len]; commit(changeWall(w0.guid, { b: [w0.a[0] + u[0] * mm, w0.a[1] + u[1] * mm] }), one(title)); }, { min: 10 }));
      }
      const ab = !many ? above(w0) : null, toLevel = walls.every((w) => w.topLevel), fixed = walls.every((w) => !w.topLevel);
      const topOpts = [['level', t('model.wall.top.level')], ['fixed', t('model.wall.top.fixed')]];
      if (!toLevel && !fixed) topOpts.unshift(['', t('model.prop.mixed')]);
      field(t('model.wall.top'), select(topOpts, toLevel ? 'level' : fixed ? 'fixed' : '', 'data-mp-top', (v) => {
        if (v === 'level') {
          const ch = walls.map((w) => ({ w, a: above(w) }));
          if (ch.some((x) => !x.a)) { host.status(t('model.err.notop'), true); renderProps(); return; }
          commit((editor) => { let r = null; for (const x of ch) r = mergeR(r, changeWall(x.w.guid, { topLevel: x.a.guid, topOffset: 0 })(editor)); return r; }, one(title));
        } else if (v === 'fixed') commit(many ? changeWalls(guids, { height: w0.height }) : changeWall(w0.guid, { height: w0.height }), one(title));
      }));
      field(t('model.wall.height'), lenInput(walls.every((w) => Math.abs(w.height - w0.height) < 0.5) ? w0.height : NaN, 'data-mp-height', (mm) => {
        if (toLevel && !many && ab) commit(changeWall(w0.guid, { topOffset: w0.topOffset + (mm - w0.height) }), one(title));
        else commit(many ? changeWalls(guids, { height: mm }) : changeWall(w0.guid, { height: mm }), one(title));
      }, { min: 100 }), toLevel && ab ? el('span', { className: 'bc-sub' }, ' ' + t('model.prop.tolevel', { name: ab.name })) : null);
      if (!many) {
        field(t('model.prop.base'), lenInput(w0.baseOffset, 'data-mp-base', (mm) => commit(changeWall(w0.guid, { baseOffset: mm }), one(title)), { allowZero: true }));
        field(t('model.prop.outside'), el('span', {}, el('span', {}, t(w0.extSide === 'right' ? 'model.prop.right' : 'model.prop.left') + ' '),
          button(t('model.ot.flip'), 'data-mp-flipside', () => commit(changeWall(w0.guid, { extSide: w0.extSide === 'right' ? 'left' : 'right' }), one(title)))));
        rows.push(el('dt', {}, t('model.level')), el('dd', {}, levels.find((l) => l.id === w0.storey)?.name || ''));
      }
    } else if (ops.length && !walls.length && ops.every((o) => o.kind === ops[0].kind)) {
      const o0 = ops[0], many = ops.length > 1, guids = ops.map((o) => o.guid), door = o0.kind === 'door';
      const types = door ? DOOR_TYPES : WINDOW_TYPES;
      title = many ? t('model.elements', { n: ops.length }) : openingTypeName(openingType(o0.typeKey) || types[0]);
      const run = (ch) => commit(many ? changeOpenings(guids, ch) : changeOpening(o0.guid, ch), one(title));
      const same = (f) => ops.every((o) => Math.abs(o[f] - o0[f]) < 0.5);
      const sameType = ops.every((o) => o.typeKey === o0.typeKey);
      const opts = types.map((x) => [x.key, openingTypeName(x)]);
      if (!sameType) opts.unshift(['', t('model.prop.changetype')]);
      field(t('model.ot.type'), select(opts, sameType ? o0.typeKey : '', 'data-mp-otype', (v) => { if (v) run({ typeKey: v }); }));
      field(t('model.ot.width'), lenInput(same('width') ? o0.width : NaN, 'data-mp-ow', (mm) => run({ width: mm }), { min: 100 }));
      field(t('model.ot.height'), lenInput(same('height') ? o0.height : NaN, 'data-mp-oh', (mm) => run({ height: mm }), { min: 100 }));
      if (!door) field(t('model.ot.sill'), lenInput(same('sill') ? o0.sill : NaN, 'data-mp-osill', (mm) => run({ sill: mm }), { allowZero: true }));
      if (!many) {
        const wall = data.walls.find((w) => w.id === o0.wall);
        const [e0, e1] = wall ? faceEndsOf(wall, o0.face) : [0, o0.offset + o0.width];
        field(t('model.prop.fromstart'), lenInput(o0.offset - e0, 'data-mp-ostart', (mm) => run({ offset: e0 + mm }), { allowZero: true }));
        field(t('model.prop.fromend'), lenInput(e1 - o0.offset - o0.width, 'data-mp-oend', (mm) => run({ offset: e1 - mm - o0.width }), { allowZero: true }));
        field(t('model.prop.hinges'), button(t('model.ot.flip'), 'data-mp-hand', () => run({ hand: o0.hand === 'b' ? 'a' : 'b' })));
        field(t(door ? 'model.prop.swing' : 'model.prop.wface'), button(t('model.ot.flip'), 'data-mp-face', () => run({ face: o0.face === 'right' ? 'left' : 'right' })));
      }
    } else if (p.derived.length === 1 && !walls.length && !ops.length) {
      const d = p.derived[0];
      // a roof is picked by one of its parts, which are made again: pick it again by its GlobalId after the reload
      const again = () => { const r = snapshotModel(ed()).derived.roofs.find((x) => x.guid === d.guid); return r?.parts.length ? [kb() + r.parts[0]] : null; };
      const run = (ch) => commit(changeDerived([d.guid], ch), { ...one(title), ...(d.kind === 'roof' ? { reselect: again } : {}) });
      const text = (value, attr, onText) => {
        const i = el('input', { type: 'text', className: 'bc-input', value: value || '', maxLength: 120, [attr]: '' });
        i.addEventListener('change', () => { if (i.value.trim() && i.value !== i.defaultValue) onText(i.value.trim()); else i.value = i.defaultValue; });
        i.addEventListener('keydown', (e) => { if (e.key === 'Enter') { e.preventDefault(); i.blur(); } if (e.key === 'Escape') { i.value = i.defaultValue; i.blur(); } });
        return i;
      };
      if (d.kind === 'room') {
        title = t('model.room');
        const loops = loopsOf(ed(), d.storey), loop = loopAt(loops.rooms, d.seed);
        field(t('model.room.longname'), text(d.name, 'data-mp-rname', (v) => run({ name: v })));
        field(t('model.room.number'), text(d.number, 'data-mp-rnum', (v) => run({ number: v })));
        if (loop) { rows.push(el('dt', {}, t('model.room.area')), el('dd', { 'data-mp-rarea': '' }, fmtArea(loop.area, S.units)), el('dt', {}, t('model.room.perimeter')), el('dd', {}, fmtLen(loop.perimeter, S.units))); }
      } else if (d.kind === 'floor') {
        title = t('model.floor.' + (FLOOR_KINDS.some((k) => k.key === d.kind) ? d.kind : 'wood'));
        field(t('model.fl.kind'), select(FLOOR_KINDS.map((k) => [k.key, t('model.floor.' + k.key)]), d.kind, 'data-mp-flkind', (v) => run({ kind: v })));
        field(t('model.prop.thickness'), lenInput(d.thickness, 'data-mp-flthick', (mm) => run({ thickness: mm }), { min: 10 }));
        field(t('model.fl.offset'), lenInput(d.offset, 'data-mp-floff', (mm) => run({ offset: mm }), { any: true }));
      } else if (d.kind === 'ceiling') {
        title = t('model.ceiling');
        field(t('model.ce.height'), lenInput(d.height, 'data-mp-ceh', (mm) => run({ height: mm }), { min: 500 }));
        field(t('model.prop.thickness'), lenInput(d.thickness, 'data-mp-cet', (mm) => run({ thickness: mm }), { min: 1 }));
      } else {
        title = t('model.roof');
        const pi = el('input', { type: 'text', className: 'bc-input bc-num', value: fmtPitch(d.pitch, S.units), 'data-mp-rfpitch': '' });
        pi.addEventListener('change', () => { const deg = parsePitch(pi.value); if (deg > 1 && deg < 80) run({ pitch: deg }); else { host.status(t('model.err.pitch'), true); pi.value = pi.defaultValue; } });
        pi.addEventListener('keydown', (e) => { if (e.key === 'Enter') { e.preventDefault(); pi.blur(); } });
        field(t('model.rf.pitch'), pi);
        field(t('model.rf.overhang'), lenInput(d.overhang, 'data-mp-rfover', (mm) => run({ overhang: mm }), { allowZero: true }));
        // RC.1: gable overhang, presets incl. shed, plate height, fascia, soffit, and each wall's own setting
        field(t('res.rf.rake'), lenInput(Number.isFinite(d.rake) ? d.rake : d.overhang, 'data-mp-rfrake', (mm) => run({ rake: mm }), { allowZero: true }));
        const wl = data.levels.find((l) => l.guid === d.walls);
        const ws = wl ? (data.walls || []).filter((w) => w.storey === wl.id) : [];
        const o = wl && loopAt(loopsOf(ed(), wl.id).outlines, d.seed);
        const kinds = o ? edgeKinds(o.poly, ws, d) : [];
        const sig = kinds.map((k) => k.kind).join(',');
        const custom = kinds.some((k) => k.kind === 'pitch' && Math.abs(k.pitch - d.pitch) > 1e-6);
        const presetSig = (v) => { const m = o && presetEdges(o.poly, ws, v, null); return m ? edgeKinds(o.poly, ws, { ...d, edges: formatEdges(m), gables: [] }).map((k) => k.kind).join(',') : null; };
        let cur = '';
        if (!custom) for (const v of ['hip', 'gable-short', 'gable-long']) if (presetSig(v) === sig) { cur = v; break; }
        if (!cur && !custom && kinds.filter((k) => k.kind === 'pitch').length === 1 && kinds.every((k) => k.kind !== 'none')) cur = 'shed';
        field(t('model.rf.ends'), select([['', t('res.rf.custom')], ['hip', t('model.rf.hip')], ['gable-short', t('model.rf.gable.short')], ['gable-long', t('model.rf.gable.long')], ['shed', t('res.rf.shed')]], cur, 'data-mp-rfends', (v) => {
          if (!v) return;
          const m = o ? presetEdges(o.poly, ws, v, v === 'shed' ? shedLow(o.poly, kinds) : null) : null;
          if (!m) { host.status(t('model.err.gable4'), true); renderProps(); return; }
          run({ edges: formatEdges(m) || ' ', gables: [] });
        }));
        field(t('res.rf.plate'), lenInput(d.plate > 0 ? d.plate : NaN, 'data-mp-rfplate', (mm) => run({ plate: mm }), { allowZero: true }), el('span', { className: 'bc-sub' }, ' ' + t('res.rf.plate.auto')));
        field(t('res.rf.fascia'), lenInput(d.fascia, 'data-mp-rffascia', (mm) => run({ fascia: mm }), { allowZero: true }));
        const so = el('input', { type: 'checkbox', checked: !!d.soffit, 'data-mp-rfsoffit': '' });
        so.addEventListener('change', () => run({ soffit: so.checked }));
        field(t('res.rf.soffit'), el('label', {}, so, ' ' + t('res.rf.soffit.on')));
        field(t('model.prop.thickness'), lenInput(d.thickness, 'data-mp-rfthick', (mm) => run({ thickness: mm }), { min: 10 }));
        if (o) {
          // one row per wall under the roof: pitched (its own pitch), gable end, or none (stops flush)
          const guids = edgeWalls(o.poly, ws), map = parseEdges(d.edges);
          const keyAt = (i) => guids[i] || '@' + i;
          const setEdge = (i, e) => {
            const m = new Map(map);
            if (!d.edges && d.gables.length) d.gables.forEach((gi) => m.set(keyAt(gi), { kind: 'gable' }));
            if (e) m.set(keyAt(i), e); else m.delete(keyAt(i));
            run({ edges: formatEdges(m) || ' ', gables: [] });
          };
          const list = el('ol', { className: 'bc-rf-edges', 'data-mp-rfedges': '' }, ...kinds.map((k, i) => {
            const a = o.poly[i], b = o.poly[(i + 1) % o.poly.length];
            const kind = select([['pitch', t('res.rf.k.pitch')], ['gable', t('res.rf.k.gable')], ['none', t('res.rf.k.none')]], k.kind, 'data-mp-rfedge', (v) => setEdge(i, v === 'pitch' ? null : { kind: v }));
            const ep = el('input', { type: 'text', className: 'bc-input bc-num', value: k.kind === 'pitch' ? fmtPitch(k.pitch, S.units) : '', size: 6, hidden: k.kind !== 'pitch', 'data-mp-rfedgepitch': String(i), 'aria-label': t('model.rf.pitch') });
            ep.addEventListener('change', () => { const deg = parsePitch(ep.value); if (!(deg > 1 && deg < 80)) { host.status(t('model.err.pitch'), true); ep.value = ep.defaultValue; return; } setEdge(i, Math.abs(deg - d.pitch) < 1e-6 ? null : { kind: 'pitch', pitch: deg }); });
            ep.addEventListener('keydown', (e) => { if (e.key === 'Enter') { e.preventDefault(); ep.blur(); } });
            const li = el('li', { 'data-mp-rfedgerow': String(i) }, el('span', { className: 'bc-sub' }, t('res.rf.edge', { len: fmtLen(Math.hypot(b[0] - a[0], b[1] - a[1]), S.units) })), ' ', kind, ' ', ep);
            const hi = (on) => { S.hiEdge = on && wl.id === S.level ? [a[0], a[1], b[0], b[1]] : null; drawOverlay(); };
            li.addEventListener('mouseenter', () => hi(true)); li.addEventListener('mouseleave', () => hi(false));
            kind.addEventListener('focus', () => hi(true)); kind.addEventListener('blur', () => hi(false));
            return li;
          }));
          rows.push(el('dt', { className: 'bc-mp-wide' }, t('res.rf.walls')), el('dd', { className: 'bc-mp-wide' }, list));
        }
      }
    } else return null;
    return el('section', { className: 'bc-model-props', 'data-model-props': '' },
      el('h3', { className: 'bc-h' }, title), el('dl', { className: 'bc-dl bc-mp-dl' }, ...rows),
      el('p', { className: 'bc-sub' }, t('model.prop.note')));
  }
  /** A point just outside the roof's current low side (its one pitched edge), else its longest edge: where Shed puts the eave. */
  function shedLow(poly, kinds) {
    const n = poly.length, len = poly.map((p, i) => Math.hypot(poly[(i + 1) % n][0] - p[0], poly[(i + 1) % n][1] - p[1]));
    const p1 = kinds.filter((k) => k.kind === 'pitch').length === 1 ? kinds.findIndex((k) => k.kind === 'pitch') : len.indexOf(Math.max(...len));
    const a = poly[p1], b = poly[(p1 + 1) % n];
    return [(a[0] + b[0]) / 2, (a[1] + b[1]) / 2];
  }
  /** The face's ends of a wall along it, from the joined outline of its storey's walls. */
  function faceEndsOf(wall, face) {
    if (!S.outlines.has(wall.guid) || wall.storey !== S.level) {
      const here = (S.data?.walls || snapshotModel(ed()).walls).filter((w) => w.storey === wall.storey);
      const o = wallOutlines(here.map((w) => ({ key: w.guid, a: w.a, b: w.b, t: w.t })));
      const keep = S.outlines; S.outlines = o; const r = faceEnds(wall, face); S.outlines = keep; return r;
    }
    return faceEnds(wall, face);
  }
  const mergeR = (a, b) => (!a ? b : { ...b, journal: mergeJournals(a.journal, b.journal) });
  let propsHost = null;
  function renderProps() { if (propsHost) decorate(propsHost); }
  function decorate(container) {
    propsHost = container;
    container.querySelector('[data-model-props]')?.remove();
    let block = null;
    try { block = propsBlock(); } catch (e) { console.error(e); }
    if (!block) return;
    const after = container.querySelector('.bc-prop-name, [data-pick-count]');
    if (after) after.after(block); else container.prepend(block);
  }

  setTool('select');
  /** What an extension (extend()) may use. */
  const api = {
    S, root, host, t, ed, kb, commit, refresh, buildScene, drawOverlay, setTool, level, levelAbove, wallsHere, wallsOn, snapAt,
    pv: () => pv, units: () => S.units, round, selectedKeys, renderProps: () => renderProps(), loopsOf: (st) => loopsOf(ed(), st), fitPlan, mergeJournals,
    /** The plan scene of any level (the sheets' plans), the level shown left as it was. */
    sceneFor(levelId) {
      const keep = { level: S.level, loops: S.loops, outlines: S.outlines, cut: S.cut, cutFor: S.cutFor };
      try { S.level = levelId; S.loops = loopsOf(ed(), levelId); S.cutFor = null; cutPlan(); return makeScene(); }
      finally { Object.assign(S, keep); }
    },
    schedules: () => scheduleData(),
  };
  return {
    decorate,
    // another file: its 3D view first (Draw off); New building turns Draw on again right after
    opened() { S.cutFor = null; S.level = null; endChain(); S.queue = []; S.empty3d = !(host.viewer()?.allKeys?.().size); if (S.on) show(false); },
    stage(which) { if (which !== 'ifc' && S.on) show(false); },
    rebuilding() { S.reloading = true; S.reloadT = Date.now(); },
    rebuilt() {
      S.reloading = false; S.cutFor = null;
      // a new building's first walls: frame the 3D view once (it had nothing to look at)
      const v = host.viewer(), n = v?.allKeys?.().size || 0;
      if (S.empty3d && n > 0) v.view('iso');
      S.empty3d = n === 0;
      if (S.on) refresh();
      if (S.reselect && !S.queue.length) { try { const ks = S.reselect(); if (ks?.length) S.keepSel = ks; } catch (e) { /* gone */ } S.reselect = null; }
      if (S.keepSel && !S.queue.length) {
        const have = v?.allKeys?.() || new Set(), keys = S.keepSel.filter((k) => have.has(k));
        S.keepSel = null;
        if (keys.length) host.select(keys);
      }
      // a reload this module did not start (Undo, Redo, a placement): what was clicked meanwhile goes in now
      if (!S.busy && S.queue.length) { const q = S.queue.splice(0); for (const [run, meta] of q) apply0(run, meta); scheduleRebuild(null); }
    },
    picked() { if (pv && S.on) { pv.setSelection(selectedKeys()); drawOverlay(); } },
    show,
    get on() { return S.on; },
    /** Add tools from another module: { tools: [names], hint(name), typed(name, text?), move(p, e), click(p, e), dbl, escape(),
     *  key(e, low), overlay(o), scene(scene), ownKeys(set, keyBase), refresh(), toolChanged(name), classes (RegExp of IFC
     *  classes), pick(key) → { guid, label, … } | null, props(items, kit) → { title } (rows pushed into kit.rows),
     *  remove(items) → op }. Returns the plan's API for it. */
    extend(x) {
      exts.push(x);
      for (const b of root.querySelectorAll('[data-mtool]')) if (!ui.tools.includes(b)) { ui.tools.push(b); b.addEventListener('click', () => setTool(b.dataset.mtool)); }
      return api;
    },
    wallsPicked: pickedWalls,
    picked,
    /** The Edit menu on the tools' own elements: Delete re-joins the walls left; Copy / Cut / Duplicate are not built for
     *  them yet (a copied door would leave its wall). → true when handled here. */
    edit(name) {
      const p = picked();
      if (!p.count) return false;
      if (name === 'delete') {
        if (p.count !== selectedKeys().length) return false;
        deletePicked(p);
        return true;
      }
      if (name === 'copy' || name === 'cut' || name === 'duplicate') { host.status(t('model.copy.notyet')); return true; }
      return false;
    },
  };
}

/** One journal from two (the second's first states of lines the first already changed are dropped). */
export function mergeJournals(a, b) {
  if (!a) return b; if (!b) return a;
  const before = new Map(a.before);
  for (const [id, line] of b.before) if (!before.has(id) && !a.created.includes(id)) before.set(id, line);
  return { created: [...a.created, ...b.created], before, retyped: [...(a.retyped || []), ...(b.retyped || [])] };
}
