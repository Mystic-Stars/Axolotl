# Surface System

Use `surface-*` variables to describe UI elevation and separation. The scale is ordered from the page base up through stronger raised surfaces and strokes.

## Layers

| Token       | Use                                                                 |
| ----------- | ------------------------------------------------------------------- |
| `surface-1` | Page background.                                                    |
| `surface-2` | Default raised surfaces, table rows, and standard card backgrounds. |
| `surface-3` | Header bands, inputs, dropdown surfaces, and card hover states.     |
| `surface-4` | Standard strokes and outlines, including table outlines.            |
| `surface-5` | Strong strokes for surfaces that need extra separation.             |

## Panels and the cards on them

A full-page region that owns chrome — the settings shell's sidebar and content
column are the reference case — is a **panel** and uses `surface-2`. Content
cards sitting on a panel use **`surface-3`**, one rung above it: at the same
level a card is invisible against its own panel. This is also the level the
shared `.base-card` uses.

Light and dark express that separation differently, and both have to work:

- In the **light** theme the `surface-2` → `surface-3` steps differ by only
  ~1.03:1, so the **stroke** carries it. Panels and cards on them keep a
  `surface-4` border.
- In **dark** themes the stroke is nearly invisible and shadows are dark on
  dark, so the **luminance step** carries it.

A component only moves with the custom-background opacity settings if it reads
a `--surface-*` token. Reading a legacy `--color-*` alias does not work: custom
properties resolve where they are declared, so the aliases on `:root` capture
the opaque values before the translucent palette is applied on `body`.

## Strokes

Use `surface-4` for normal outlines and dividers. Tables should use `surface-4` for their outer border and row separators.

Reserve `surface-5` for stronger outlines, such as modal frames, high-emphasis separators, or hover states on elements that already sit on `surface-4`.

## Backgrounds

Use `surface-1` for page backgrounds and `surface-2` for ordinary raised content. Use `surface-3` for header strips, inputs, and temporary elevation such as hover states. Use `surface-4` sparingly as a stronger raised background, usually for controls or badges that need to sit above nearby content.

Avoid using legacy aliased background variables for new UI. Prefer explicit `bg-surface-*` and `border-surface-*` utilities so the layer intent is visible in the component.
