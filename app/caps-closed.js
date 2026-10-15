// SPDX-License-Identifier: Apache-2.0
/**
 * caps-closed.js — which elements of a batch are closed solids, for the section caps (viewer3d.js updateCaps).
 *
 * The caps count, along each view ray, how often it crosses an element's surface behind the section plane: odd = inside a
 * solid, so the cut face paints there. That only holds for closed surfaces. An open one (tree foliage, a terrain sheet,
 * a single-sided panel) gives odd counts over its whole outline, and the cut colour painted over it — the "ghosted"
 * elements of a Discord report (PokeJohn, 2026-10-14), worse or better depending on the view angle. Such elements are
 * left out of the count: they have no inside to fill.
 *
 * Closed = every edge of the element's triangles is used an even number of times, with vertices compared by position
 * (exporters split vertices at hard edges, so indices alone would call every box open). DOM-free; no three.js.
 *
 *   openElements(positions, indices, triIds) → Set of element ids whose surface is open
 *   stencilIndex(index, nSolid, triIds, open) → Uint32Array: the shown (group 0) triangles of closed elements
 */

const Q = 1e4;   // positions are compared at 0.1 mm (model metres)

/** Element ids (triIds[t] for triangle t of `indices`) whose triangles do not make a closed surface. */
export function openElements(positions, indices, triIds) {
  const nV = positions.length / 3;
  // One id per distinct position, so split vertices meet again.
  const canon = new Int32Array(nV);
  const seen = new Map();
  for (let v = 0; v < nV; v++) {
    const k = Math.round(positions[v * 3] * Q) + ',' + Math.round(positions[v * 3 + 1] * Q) + ',' + Math.round(positions[v * 3 + 2] * Q);
    let id = seen.get(k);
    if (id === undefined) { id = seen.size; seen.set(k, id); }
    canon[v] = id;
  }
  const nP = seen.size;
  // Triangles by element.
  const byEl = new Map();
  const nT = indices.length / 3;
  for (let t = 0; t < nT; t++) {
    const e = triIds[t];
    let l = byEl.get(e);
    if (!l) { l = []; byEl.set(e, l); }
    l.push(t);
  }
  const open = new Set();
  const edges = new Map();
  for (const [e, tris] of byEl) {
    edges.clear();
    for (const t of tris) {
      const a = canon[indices[t * 3]], b = canon[indices[t * 3 + 1]], c = canon[indices[t * 3 + 2]];
      for (const [p, q] of [[a, b], [b, c], [c, a]]) {
        if (p === q) continue;   // a degenerate edge adds nothing
        const key = p < q ? p * nP + q : q * nP + p;
        edges.set(key, (edges.get(key) || 0) + 1);
      }
    }
    for (const n of edges.values()) if (n % 2) { open.add(e); break; }
  }
  return open;
}

/** The index for a stencil pass: the first nSolid triangles (group 0, shown) whose element is closed. */
export function stencilIndex(index, nSolid, triIds, open) {
  if (!open.size) return index.subarray(0, nSolid * 3);
  const out = new Uint32Array(nSolid * 3);
  let n = 0;
  for (let t = 0; t < nSolid; t++) {
    if (open.has(triIds[t])) continue;
    out[n++] = index[t * 3]; out[n++] = index[t * 3 + 1]; out[n++] = index[t * 3 + 2];
  }
  return out.subarray(0, n);
}
