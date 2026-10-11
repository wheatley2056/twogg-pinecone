# MC Skin Editor

A free, single-page 3D editor for Minecraft player skins. Paint directly on the 3D model or the 2D texture map, right in your browser — no account or install needed.

> **Try it out at [skincrafter.io](https://skincrafter.io).**

This repository holds the single-page 3D editor for Minecraft player skins, built with Svelte 5, TypeScript, Vite, and three.js. Everything runs client-side — skins, palette, and undo history live in your browser's `localStorage`, no server or account needed.

## Features

- **Resolutions**: Legacy (64×32), Standard (64×64), High (128×128)
- **Models**: Classic (4px arms) and Slim (3px arms), chosen per skin
- **Paint tools**: Pencil (adjustable brush size), Eraser, Eyedropper, Paint bucket (fill)
- **Color picker**: color wheel (via [iro.js](https://iro.js.org/)) plus a hex input, with Darken/Lighten step buttons
- **16-slot palette**: click an empty slot to save the current color, click a filled slot to select it, right-click to overwrite
- **Undo/redo**: up to 50 steps
- **Mirror painting**: mirrors Pencil/Eraser/Fill strokes between left/right arms and legs
- **3D viewport**: orbit/rotate, zoom in/out, paint directly on the model
- **2D texture map**: the full UV-unwrapped skin texture, editable pixel-by-pixel, with a checkerboard behind transparent pixels and a grid overlay
- **Layer & part visibility toggles**: show/hide the Body and Outer Layer (hat/jacket/sleeves/pants) layers, and each of the six body parts, to make editing occluded areas easier
- **Import/export**: import an existing skin PNG (auto-detects resolution from its dimensions) or export the current skin as an uncompressed PNG with alpha
- **Multiple saved skins**: create, rename, duplicate, and delete named skins; the app always auto-saves the current skin as you work
- **Keyboard shortcuts**: `B` pencil, `E` eraser, `I` eyedropper, `G` fill, `M` mirror, `[`/`]` brush size, `Ctrl/Cmd+Z` undo, `Ctrl/Cmd+Shift+Z` or `Ctrl/Cmd+Y` redo

## Getting started

```bash
npm install
npm run dev
```

Then open the printed local URL in a browser.

### Other scripts

```bash
npm run build    # production build to dist/
npm run preview  # preview the production build
npm run check    # type-check (svelte-check + tsc)
```

## How the skin texture maps to the 3D model

The UV layout follows the standard Minecraft player-skin "box UV" template (documented at [minecraft.wiki/w/Skin](https://minecraft.wiki/w/Skin)): each body part is a cuboid whose six faces unwrap onto the texture atlas at fixed offsets, scaled by 1× (64-wide) or 2× (128-wide). See [`src/lib/skin/uv.ts`](src/lib/skin/uv.ts) for the exact math, including how legacy 64×32 skins mirror the right arm/leg onto the left side (matching how the game itself renders that older format) and how the 3D raycasting hit-test converts a click straight back to a texture pixel.

## Project layout

```
src/
  lib/
    skin/        skin data model: types, resolutions, UV layout, 3D part placement
    stores/       Svelte stores: pixel buffer, undo history, palette, tool state,
                  layer/part visibility, saved-skins persistence
    three/        three.js model builder + raycasting-to-pixel helper
    color.ts      hex/RGB/HSL conversion and lighten/darken
    png.ts        PNG import/export helpers
    persist.ts    localStorage read/write wrapper
    paintController.ts  applies the active tool to a pixel (shared by the 2D and 3D views)
  components/     UI: toolbar, color picker, palette, visibility panel, 2D canvas,
                  3D viewport, skin manager bar (new/import/export/reset), dialogs
```

## License

Copyright 2026 Eric Heikes and Charity Heikes.

Licensed under the Apache License, Version 2.0 (the "License"); you may not use this software except in compliance with the License. You may obtain a copy of the License at [http://www.apache.org/licenses/LICENSE-2.0](http://www.apache.org/licenses/LICENSE-2.0).

Unless required by applicable law or agreed to in writing, software distributed under the License is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied. See the License for the specific language governing permissions and limitations under the License.
