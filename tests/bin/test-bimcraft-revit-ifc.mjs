#!/usr/bin/env node
/* SPDX-License-Identifier: Apache-2.0 */
/**
 * test-bimcraft-revit-ifc.mjs — 0.5 SG.8 (a): BimCraft opens IFC exported from Revit the way help topic rvt-to-ifc
 * ("From a project file to BimCraft") says: storeys from levels, rooms as spaces, property sets and base quantities,
 * the exporter named in the File panel. Runs on the Revit exports in /var/www/CADCRAFT/testfiles/samples (outside git;
 * skipped with a note when a clean clone has none): an IFC2X3 Coordination View 2.0 export and an IFC4 one.
 *   node bin/test-bimcraft-revit-ifc.mjs [--verbose]
 */
import fs from 'node:fs';
import path from 'node:path';
import { register } from 'node:module';
import { fileURLToPath, pathToFileURL } from 'node:url';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const APP = path.join(ROOT, 'public_html/assets/apps/bimcraft/dev');
const SAMPLES = process.env.BIMCRAFT_TESTFILES ? path.join(process.env.BIMCRAFT_TESTFILES, 'samples') : '/var/www/CADCRAFT/testfiles/samples';
const VERBOSE = process.argv.includes('--verbose');
register('data:text/javascript,' + encodeURIComponent(`export async function resolve(spec, ctx, next) {
  if (spec === 'manifold-3d') return { url: ${JSON.stringify(pathToFileURL(path.join(ROOT, 'public_html/assets/vendor/manifold-3d/current/manifold.js')).href)}, shortCircuit: true };
  return next(spec, ctx);
}`), import.meta.url);
(await import(pathToFileURL(path.join(APP, 'i18n.js')).href)).init({ strings: JSON.parse(fs.readFileSync(path.join(APP, 'lang/en.json'), 'utf8')) });
const WebIFC = await import(pathToFileURL(path.join(APP, 'ifc/api.js')).href);
const { IfcModel } = await import(pathToFileURL(path.join(APP, 'ifc-model.js')).href);
let pass = 0, fail = 0;
const ok = (c, m) => { if (c) { pass++; if (VERBOSE) console.log('  ok   ' + m); } else { fail++; console.log('  FAIL ' + m); } };
const quiet = (fn) => async (...a) => { const l = console.log, w = console.warn; console.log = console.warn = () => {}; try { return await fn(...a); } finally { console.log = l; console.warn = w; } };

const FILES = [
  { file: 'Ifc2x3_Duplex_Architecture.ifc', schema: 'IFC2X3' },
  { file: 'Ifc4_SampleHouse.ifc', schema: 'IFC4' },
];
const present = FILES.filter((f) => fs.existsSync(path.join(SAMPLES, f.file)));
if (!present.length) { console.log('no Revit IFC exports in ' + SAMPLES + ' (a clean clone): skipped'); process.exit(0); }
for (const f of present) {
  const bytes = new Uint8Array(fs.readFileSync(path.join(SAMPLES, f.file)));
  const im = await quiet(IfcModel.create)(WebIFC, null);
  const m = await quiet((b) => im.load(b))(bytes);
  const txt = Buffer.from(bytes.subarray(0, 4000)).toString('latin1');
  ok(/Revit/.test(txt), `${f.file}: written by Revit (its header / application)`);
  ok(m.schema === f.schema, `${f.file}: ${m.schema}`);
  ok(m.storeys.length >= 2, `${f.file}: storeys from the levels (${m.storeys.map((s) => s.name).join(', ')})`);
  ok(m.elements > 20, `${f.file}: ${m.elements} elements drawn`);
  ok(m.spaces > 0, `${f.file}: rooms as spaces (${m.spaces})`);
  ok(!!m.project.application, `${f.file}: the exporter is named (${m.project.application})`);
  // a wall: its property sets and quantities, as the help says the export keeps them
  let wall = null;
  for (const b of m.batches) { for (const id of b.elems.ids) { if (m.classOf.get(id - (m.keyBase || 0)) === 'IfcWall' || m.classOf.get(id - (m.keyBase || 0)) === 'IfcWallStandardCase') { wall = id; break; } } if (wall) break; }
  const p = wall ? await quiet((id) => im.properties(id))(wall) : null;
  const names = (p?.sets || []).map((s) => s.name);
  ok(p && names.some((n) => /^Pset_WallCommon$/.test(n)), `${f.file}: a wall carries Pset_WallCommon (sets: ${names.slice(0, 8).join(', ')})`);
  if (VERBOSE) console.log('  sets on a wall: ' + names.join(', '));
  im.close?.();
}
console.log(`\n${pass} passed, ${fail} failed`);
process.exit(fail ? 1 : 0);
