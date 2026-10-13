---
title: What is not built yet
summary: What this BimCraft build does not do yet, and what to use instead today.
section: trouble
order: 20
keywords: missing, limits, limitations, not supported, roadmap, authoring, modeling, sections, measure, clash, bcf, cloud, desktop, export
---

BimCraft today is a reader, checker and IFC data editor. This is the honest list of what it does not do yet, with what to do instead.

## Modeling and authoring

| Not yet | Instead, today |
|---|---|
| Drawing or placing walls, doors, floors or any element | Use your authoring program, then open its IFC export here |
| Changing an element's shape or size | Your authoring program. Delete, copy, cut and paste work here: see [Delete, cut, copy and paste building elements](topic:edit-elements) |
| Deleting storeys, spaces, buildings or the site | Delete the elements in them |
| Pasting building elements into another model | Copy and paste within one model; placed library items paste into any model |
| Adding new properties or property sets | Fill in existing values here, or add the parameter in your authoring program and export again |
| Editing quantities, lists, enumerations | Your authoring program |
| Saving project or family files | BimCraft saves IFC only |
| Families: editing or creating | Your authoring program |

## Viewing models

| Not yet | Instead, today |
|---|---|
| 3D from project files | Export IFC and open it. See [Get a project file into 3D through IFC](topic:rvt-to-ifc) |
| Floor plans, elevations, several section planes at once | One {{ui:Section}} plane or a section box at a time; see [Cut an IFC model with a section plane](topic:section-plane). Untick storeys in {{ui:Storeys}} |
| Sharing search sets with others, sets inside the IFC file | Sets stay in this browser; see [Find elements by class, name or property](topic:find) |
| Measuring angles, or along a path of several points | Measure each part with {{ui:Measure}}: see [Measure distances and face areas in 3D](topic:measure) |
| Saved views, shadows | Save a view as an issue: see [Issues and BCF files](topic:issues-bcf). Walk mode: see [Walk through a model](topic:walk) |
| Clearance (soft) clashes, clash groups, saving clash results | Run {{ui:Clash check}} and make issues of the ones that matter: see [Check for clashes](topic:clash-check) |
| Saving several models as one set, or editing two models at once | Edit one model at a time; each saves its own file |
| Assigning issues, due dates, labels, several viewpoints per issue, editing an issue | One view per issue; see [Issues and BCF files](topic:issues-bcf) |
| `.ifczip` and `.ifcxml` | Unzip or export plain `.ifc` |
| Opening `.pln`, `.pla`, `.tpl` and `.bpn` project files | Export IFC and DWG from them: see [Open .pln and .pla project files](topic:pln-project-files) |
| Opening the {{ui:Elements}} list to select by kind | Click a kind's group in the {{ui:Model tree}}: see [Find your way with the model tree](topic:model-tree) |

## Drawings

| Not yet | Instead, today |
|---|---|
| Editing, measuring or saving DWG and DXF | The CadCraft drafting app ({{ui:Edit in CadCraft}} on the 2D view) |
| Printing or exporting PDF | Print from the program that made the drawing; check it first with {{ui:Print view}} |
| Plot styles and pen tables | {{ui:Print view}} shows plain black with line weights |
| DWG files older than R13, and binary DXF (BimCraft says so when you open one) | Save the drawing again as a newer DWG, or as ASCII DXF |
| Drawings linked inside linked drawings | Open them directly |
| TIFF pictures | Save a PNG or JPEG copy with the same name and add it |
| DGN drawings | Listed in folder reports only |
| Objects listed under **Not drawn yet** | View the drawing in the program that made it |
| ACIS solids, bodies, regions and surfaces, and subdivision meshes in 3D | Export IFC or 3D faces from the program that made the drawing |
| Pictures, multileaders and sheets in the 3D view of a drawing | See them on the {{ui:Model}} tab and the sheets |
| Picking objects in a drawing's 2D view | Switch to {{ui:3D}} and click the object there: see [See a DWG or DXF drawing in 3D](topic:dwg-in-3d) |
| Equipment schedules from drawings without `KCL-ITEM` tags | Not available |
| Equipment schedule as XLSX or PDF | {{ui:Download CSV}} and open it in a spreadsheet |

## Files and platforms

| Not yet | Instead, today |
|---|---|
| Cloud saving | Save copies on your computer or office server. Cloud saving is not available yet |
| Desktop builds | Use BimCraft in a browser. Desktop builds are not available yet |
| Remembering folders in Firefox and Safari | Choose the folder again, or drag it in |
| Undo after saving | Open the original file again to start over |
| Filling in the missing part of a cut-off project file | Copy the original again; in a folder, BimCraft reads the newest complete backup |
| Reopening the files you had open | Unsaved changes are offered back as "Recover your work?" when you return; open the files again from the recent list |
