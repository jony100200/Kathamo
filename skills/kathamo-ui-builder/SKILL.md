---
name: kathamo-ui-builder
description: Build consumer application UIs from Kathamo's documented public APIs, reusable widgets, state bindings, custom editor hosts and theme tokens without modifying framework internals.
---

# Building Applications With Kathamo

## Core contract

**Consume the framework; do not turn Kathamo into an application.** A consuming app owns business actions, assets, documents, model runtimes, paint/render engines and data. Kathamo owns layout, windowing, event routing, drawing UI, visual controls, accessibility and editor hosting.

## Before coding

1. Inspect the checked-out public exports, Cargo examples, widget constructors and documentation.
2. Establish which APIs are implemented and which are only proposed. Do not invent a usable SDK facade.
3. Write an application-to-UI mapping: document, source data, operations, editor types, command binding, status and errors.
4. Select existing Kathamo components; if a gap is generic, propose a reusable framework extension rather than a one-off duplicate widget.
5. Use the application's real state or an explicitly labeled mock in a UI demo.

## Suggested component composition

```text
ApplicationWindow
  WorkspaceSwitcher
  DockTree
    EditorArea [document tabs, editor toolbar]
      CustomCanvasOrViewport
    HierarchyPanel [layers, objects or tracks]
    PropertyInspector [typed, bound properties]
    OptionalGraphPanel [nodes, ports, previews]
    OutputPanel [tasks, messages, status]
```

Use an editor registry to attach application-provided editors when the public SDK supports one. The same editor document must not be duplicated on maximize or layout switch.

## Widget recipe

For each control define:

- Stable scoped identity and accessibility role/label.
- Typed inputs and controlled/source-of-truth value.
- Meaningful callback/action and invalid/disabled/loading states.
- Keyboard, pointer and focus semantics; text composition if applicable.
- Theme token classes and responsive layout constraints.
- Undo/redo transaction boundary when changing document data.
- Native interaction verification, including high-DPI and clipping.

## Reusable UI patterns

- **Tree/hierarchy:** stable item IDs, nested levels, reorder, search, selection, visibility and lazy collections.
- **Inspector:** property metadata for numbers, booleans, strings, colors, vectors, enums and assets; validate before applying.
- **Graph:** typed input/output ports, node selection, visible connection state, search-to-add, previews and clear invalid-link feedback.
- **Editor host:** app-owned interactive content region with clipping, resize, focus, input delegation, maximize/restore.
- **Tool/model selector:** capability metadata, availability, disabled reasons and meaningful progress without embedding an execution backend in Kathamo.

## UI format policy

If declarative UI documents or stylesheets become available, follow their verified schemas. Keep actions type-safe and app-owned; do not execute arbitrary script payloads from markup. Never introduce or document hypothetical `.kui`, `.kstyle` or SDK methods as existing.

## Definition of done

An independent application builds and uses the UI from public Kathamo dependencies. Its primary workflow is interactive and reusable, not a static mockup. No framework internals are patched to implement ordinary application-specific behavior.

Save private local layouts, screenshots and generated QA/test artifacts only to `$KS_KATHAMO_PRIVATE` outside Git.
