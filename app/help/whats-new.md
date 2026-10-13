---
title: What's new in 0.4
summary: Our own IFC engine, delete and paste any element, hide and isolate, a model tree, several models at once, clash check, find, measure, walk, BCF issues, quantities, compare versions, a 4D schedule, and touch screens.
section: start
order: 5
keywords: whats new, what's new, release notes, changelog, changes, 0.4, version, new features, ifc engine, model tree, hide, isolate, clash, bcf, measure, walk, compare, quantities, 4d, touch, language, spanish
---

This is what changed in BimCraft 0.4, compared with 0.3, the version Try ran before.

## New

- **Delete, cut, copy and paste any element.** Pick walls, doors, furniture or anything else and press {{key:Delete}}, {{key:Mod+X}}, {{key:Mod+C}} or {{key:Mod+V}}. {{ui:Select all}} ({{key:Mod+A}}) picks every element shown. A stair takes its flights and a wall its openings, and deleting a door closes the wall. Each is one step to undo. See [Delete, cut, copy and paste building elements](topic:edit-elements).
- **Hide, isolate and see through.** Pick several elements with {{key:Mod}}+click or {{key:Shift}}+click, then use the {{ui:Show}} menu: {{ui:Hide}} ({{key:H}}), {{ui:Isolate}} ({{key:I}}), {{ui:See-through}} ({{key:T}}) and {{ui:Show all}} ({{key:Shift+H}}). See [Hide, isolate and see through elements](topic:hide-and-isolate).
- **Model tree.** The left panel shows project, site, building, storeys and spaces, with the elements on each grouped by kind. Untick a row to hide it, click to pick, double-click to zoom. See [Find your way with the model tree](topic:model-tree).
- **Several models together.** {{ui:Add a model…}} under {{ui:Models}} opens more IFC files into the same scene, in the first model's coordinates. Hide, move or remove each one. You edit and save one model at a time. See [See several IFC models together](topic:several-models).
- **Color by property.** The {{ui:Color}} tab paints every element by one property, such as class, storey, material or any value in the file, with a legend. Click a value to show only those elements. See [Color elements by a property](topic:color-by-property).
- **Find.** {{ui:Find}} in the left panel picks every element that matches your rules, by class, name, type, storey, material or any property. Save the rules as named search sets. See [Find elements by class, name or property](topic:find).
- **Clash check.** {{ui:Clash check}} finds where two sets of elements run into each other, in one model or across several, with a tolerance. Click a clash to see it, or make an issue of it. See [Check for clashes](topic:clash-check).
- **Issues and BCF.** The {{ui:Issues}} tab saves the current view as an issue: camera, picked and hidden elements, and the section. Save and open BCF 2.1 files to share issues with other coordination tools. See [Issues and BCF files](topic:issues-bcf).
- **Measure in 3D.** {{ui:Measure}} ({{key:M}}): click two points for a distance, with corner and edge snaps, or click a face for its area. See [Measure distances and face areas in 3D](topic:measure).
- **Walk mode.** Click {{ui:Walk}}, then a floor, and walk through the model at eye height with {{key:W}} {{key:A}} {{key:S}} {{key:D}}. Walls stop you. See [Walk through a model](topic:walk).
- **Section box and filled cuts.** {{ui:Box}} in the section bar keeps only what is inside a box. Cut walls and slabs are now filled, like a drawn section; {{ui:Fill}} turns that off. See [Cut an IFC model with a section plane](topic:section-plane).
- **Model checks.** {{ui:Model checks}} lists elements missing a property you require, GlobalIds used twice, elements on no storey, and copies stacked on each other. See [Check a model for common problems](topic:model-checks).
- **Quantity totals.** {{ui:Quantity totals}} counts elements and adds up lengths, areas and volumes, grouped by class, storey or any property. It can also list one row per element, or every space with its area and volume. Each downloads as a CSV. See [Add up quantities, list elements and schedule spaces](topic:quantity-totals).
- **Compare versions.** With two versions of a model open, {{ui:Compare versions}} shows what was added, removed, changed or moved, matched by GlobalId, with a CSV of every change. See [Compare two versions of a model](topic:compare-versions).
- **4D schedule.** Open a task list as a CSV, or start from a template, and drag through the dates to see what is built, in progress and not started. See [Play a construction schedule on the model](topic:4d-schedule).
- **Open in DetailCraft.** An IFC model goes to the detailing app in a new tab, unsaved changes included. Nothing is uploaded. See [Open a file in another app](topic:open-in-another-app).
- **Pick objects in a drawing's 3D view.** Click a line, face or block in a DWG or DXF in {{ui:3D}} and {{ui:Properties}} shows its data. A drawing's 3D view also has {{ui:Section}} ({{key:X}}), and shows text, attributes and wide polylines. See [See a DWG or DXF drawing in 3D](topic:dwg-in-3d).
- **Touch screens.** Bigger buttons, tap to pick, pinch to zoom, long press to add to the pick, a walk pad, and {{ui:Turn 90°}} while placing. See [Use BimCraft on a touch screen or tablet](topic:touch-screens).
- **Recover your work.** If the tab or browser closes before you save an edited model, BimCraft offers it back next time. See [Recover unsaved work](common:recover).
- **Find anything.** Press {{key:Mod+K}}, or click {{ui:Find}} in the bar at the top, to run a tool, jump to a setting or open a help topic. See [Find tools, settings and help](common:find-tools).
- **Recent files.** The start screen lists the files you opened lately (Chrome and Edge). Click one to open it again.
- **Language.** Every word on screen now comes from a language file. {{ui:Settings}}, {{ui:General}}, {{ui:Language}} offers Español (México). It is a machine-made draft that has not been reviewed yet, and help topics are still in English.

## Improved

- **BimCraft's own IFC engine.** IFC files are read by an engine that is part of the app. Big files are read without freezing the page. Curved surfaces are drawn as curves, IFC4X3 road and rail alignments are drawn, and exports from Archicad open with their zones, layered walls, stairs and property sets. The file panel lists anything the engine could not draw yet under **Not drawn yet**. See [What BimCraft reads from each format](topic:file-formats).
- **Smaller offline copy.** {{ui:Save for offline}} keeps about half as much as in 0.3. See [Online and offline](topic:online-offline).
- **Swapping changes the IFC class.** Swap a table for a sink and the element becomes a sanitary terminal, keeping its GlobalId, place and item number. See [Swap a placed item, and update items from the library](topic:swap-and-update-library-items).
- **More panels windows.** Give each panel its own window, up to four, and drag panels between windows. A floating panel resizes by touch or pen too. See [Arrange the panels and use a second monitor](common:arrange-panels).
- **Drawings read in the background.** A DWG is read without holding up the page.
- **Meshes in 2D.** Polyface and polygon meshes now draw in a drawing's 2D view. See [View a DWG or DXF drawing](topic:view-dwg).
- **Project files from Archicad.** Opening a `.pln`, `.pla`, `.tpl` or `.bpn` file says what it is and what to export instead. See [Open .pln and .pla project files](topic:pln-project-files).

## Fixed

- The {{ui:Spaces}} button showed nothing, because spaces were never loaded. It shows a model's rooms now.
- Columns placed on a grid were drawn at the origin. They stand where the file puts them.
- Opening a binary DXF gave a general error. It now says what the file is and how to save one BimCraft reads.
- {{ui:Cut}}, {{ui:Delete}} and {{ui:Select all}} said "not built yet". They work.
- Counts read properly: "1 file", not "1 files".

## Not in this version yet

- Settings has an **Assistants** switch, but assistants connect through CadCraft Studio, which is not available yet.
- Cloud saving and desktop builds are not available yet.
- [What is not built yet](topic:not-built-yet)
