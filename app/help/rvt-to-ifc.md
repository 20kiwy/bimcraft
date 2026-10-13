---
title: Get a project file into 3D through IFC
summary: Export IFC from the program that made a project file, open the IFC in BimCraft, and know what the export keeps.
section: tasks
order: 70
keywords: export, ifc export, rvt to ifc, convert, 3d, geometry, ifc2x3, ifc4, coordination view, reference view
---

## What you'll do

Turn a project file into an IFC file you can orbit, inspect and edit in BimCraft. The export happens in the program that made the file; BimCraft opens the result.

## Steps

1. Open the project in the program that made it. In Revit, that is the release shown on BimCraft's card ("saved in Revit …").
2. Open a 3D view that shows what you want to export. Many exporters export only what that view shows when asked to.
3. Choose **File > Export > IFC**.
4. Pick the IFC version. IFC2X3 Coordination View 2.0 and IFC4 Reference View both open in BimCraft.
5. Choose a folder and file name, and export.
6. In BimCraft, click {{ui:Choose a file}} and choose the new `.ifc` file.

## What you should see

The model in 3D, with storeys taken from the project's levels and the project information in the {{ui:File}} panel. **Exported from** names the exporter.

## What the export keeps, and what it does not

- **Kept**: geometry, storeys, element kinds, names, types, most parameters (as IFC property sets), quantities if you ask the exporter for them, and rooms as spaces.
- **Not kept**: views, sheets, annotation, families as editable families, worksets and design options. An IFC file is a snapshot of the model's elements, not the project.

## Common mistakes

- **Parameters missing in BimCraft.** The exporter decides which parameters go into IFC. Look for options such as "Export Revit property sets", "Export IFC common property sets" and "Export base quantities", and export again.
- **Rooms missing.** Turn on the exporter's option to export rooms or spaces, then click {{ui:Spaces}} in BimCraft to see them.
- **Saving as `.ifczip`.** BimCraft opens plain `.ifc` only. Choose plain IFC, or unzip the `.ifczip`.
- **Model far from the origin.** BimCraft centers the model when it opens, so a model placed far away still appears in the middle of the screen.
- **Expecting edits to flow back.** Edits you save in BimCraft go into a new IFC file, not into the project file.
