---
title: Measure distances and face areas in 3D
summary: Measure between two points of an IFC model with snapping to corners and edges, read the difference along X, Y and Z, and measure the area of a flat face.
section: tasks
order: 28
keywords: measure, measurement, distance, length, dimension, area, face area, snap, corner, edge, tape, ruler, clearance
---

## What you'll do

Check a clearance, a wall length or the size of a face straight from the model, without opening the authoring program.

## Steps

1. Open an IFC model.
2. Click {{ui:Measure}} in the toolbar, or press {{key:M}}. A bar opens at the bottom of the view.
3. **A distance:** with {{ui:Distance}} chosen, click the first point, then the second. Near a corner the pointer snaps to it, near an edge onto the edge; the small label at the pointer says which. Each pair of clicks makes one measurement.
4. **A face's area:** choose {{ui:Face area}} and click a face. The whole flat face lights up and its area shows.
5. Click {{ui:Clear}} to remove the measurements, or {{ui:Done}} (or {{key:Esc}}) to stop measuring.

## What you should see

- A line between the two points with its length on it, and in the bar the length with how far it goes along X, Y and Z (in the model's axes, Z up), in metres.
- A face's area in square metres, on the face and in the bar.
- Labels stay on their measurements while you orbit, pan and zoom.
- While you measure, clicks do not pick elements. A section plane or box still cuts the model, and you can measure what it shows, cut faces included.

## Common mistakes

- **The point lands on the face, not the corner.** Snapping looks at the element under the pointer. Move closer to the corner, or zoom in.
- **The area is smaller than expected.** Openings are left out, and a face that bends is several faces: click each.
- **{{key:Esc}} stopped measuring.** The first {{key:Esc}} drops a half-made distance; the second stops.

## Next

- [Cut an IFC model with a section plane](topic:section-plane)
- [Hide, isolate and see through elements](topic:hide-and-isolate)
