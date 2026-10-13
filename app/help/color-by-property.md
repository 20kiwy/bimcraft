---
title: Color elements by a property
summary: Paint every element by the value of one property, such as its class, type, storey, material, fire rating or cost, read the legend, and click a value to show only those elements.
section: tasks
order: 26
keywords: color, colour, color by, legend, smart view, appearance, property, type, material, fire rating, filter, isolate, theme, values
---

## What you'll do

See at a glance how a property is spread over the model: which walls are external, which materials are used where, which elements have no fire rating. Every element takes the color of its value, and the legend says what each color means and how many elements have it.

## Steps

1. Open an IFC model.
2. Click the {{ui:Color}} tab on the right.
3. Choose a property in the list. The first ones belong to every element ({{ui:Class}}, {{ui:Type}}, {{ui:Storey}}, {{ui:Material}}, {{ui:Name}}…); the rest come from the model's property sets and quantity sets, under the set's name. The number after each one is how many elements have it.
4. Read the legend: a color, the value and how many elements have it.
5. Click a value to show only those elements. Hold {{key:Mod}} and click to add more values. Press {{key:Shift+H}} to show everything again.
6. Click {{ui:Off}} to go back to the model's own colors.

## What you should see

- Up to seven values each get their own color, the most common value first. More values than that share one gray, listed as **other values**.
- A number with many different values is split into six equal ranges, from light (low) to dark (high).
- Elements without the property are a pale gray, listed as **No value**, so the colored ones stand out.
- The colors change with Paper and Model, and stay while you edit the model.
- A type's property counts for every element of that type; when an element has its own value for the same property, its own value wins.

## Common mistakes

- **A property is missing from the list.** No element with a shape in the model has it. Properties of storeys and spaces themselves are not listed.
- **Everything is the same color.** Every element has the same value, or none: look at the legend's counts.

## Next

- [Hide, isolate and see through elements](topic:hide-and-isolate)
- [Find your way with the model tree](topic:model-tree)
