---
title: Cut an IFC model with a section plane
summary: Slice the 3D model from the top, front or side, or keep only what is inside a section box, to see inside; move the cut by dragging or typing, flip it, and pick what the cut shows.
section: tasks
order: 25
keywords: section, section plane, section box, clip box, cut, clipping, slice, plan cut, floor plan, elevation, inside, interior, flip, offset
---

## What you'll do

Cut through an IFC model with one plane to see inside it: from the top for a plan, or from the front or side for a section through the building.

## Steps

1. Open an IFC model. See [Open an IFC model and look around in 3D](topic:open-ifc-model).
2. Click {{ui:Section}} in the toolbar, or press {{key:X}}. The model is cut halfway up, and a bar appears under the toolbar.
3. Choose where to cut from: {{ui:Top}}, {{ui:Front}} or {{ui:Side}}. Front and Side also turn the view to look at the cut.
4. Move the cut: drag the **Offset** slider, or type a distance in metres in the box and press {{key:Enter}}. You can type millimetres too, for example `1200 mm`.
5. Click {{ui:Flip}} to keep the other side of the cut.
6. Click an element on the cut to see its properties.
7. Click {{ui:Off}}, click {{ui:Section}} again, or press {{key:X}} to take the cut away.

## A section box

Click {{ui:Box}} in the same bar to keep only what is inside a box. Each of {{ui:X}}, {{ui:Y}} and {{ui:Z}} has two sliders: where the box starts and where it ends, in metres from the model's edge. The values show beside them.

- {{ui:Box round the pick}} fits the box round the elements you picked, with 0.3 m to spare, and zooms to them.
- {{ui:Whole model}} opens the box to the whole model again.
- {{ui:Top}}, {{ui:Front}} or {{ui:Side}} go back to one plane.

## What you should see

- **Top** keeps what is below the cut, so you look down into the rooms. The offset is measured up from the bottom of the model.
- **Front** keeps what is behind the cut, seen from the front; **Side** keeps what is beyond it, seen from the side. The offset is measured from the front or the side of the model.
- A thin colored outline shows where the plane is.
- Where the plane cuts through a wall, slab or other solid, the cut face is filled in a dark color, like a drawn section. {{ui:Fill}} in the bar turns this off and on.
- The view cube in the bottom-right corner turns the model to look at the cut from any side.
- Clicking picks what you can see: anything cut away cannot be picked.

## Common mistakes

- **Nothing changed.** The cut may be above the roof or outside the model. Drag the slider, or type a smaller offset.
- **The offset jumped back.** A distance beyond the model is moved to its end, and the status line says so.
- **The cut is hollow.** {{ui:Fill}} in the bar is off: click it to fill the cut faces of walls, slabs and other solids.
- **A filled cut looks speckled or misses a piece.** Filling needs closed solids; an element whose shape in the file is not closed cannot be filled cleanly.

## Next

- [Show and hide storeys, spaces and edges](topic:storeys-and-spaces)
- [See a DWG or DXF drawing in 3D](topic:dwg-in-3d) — drawings in 3D have the same section plane
