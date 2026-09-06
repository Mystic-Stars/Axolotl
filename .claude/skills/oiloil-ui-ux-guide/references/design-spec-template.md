# `design-spec.md` Output Template

Use this structure when generating the final `design-spec.md` in the user's project root. Fill every section with concrete values, not placeholders.

```markdown
# [Project Name] Design Specification

> Generated from a `design` consultation with `oiloil-ui-ux-guide`.
> Style family: `<family>` (or `brand-driven`).

## 1. Design direction

- **Product**: [one sentence — what it does, who uses it]
- **Style family**: `<family>`. [If hybrid, e.g. "modern-minimal layout, editorial typography", say so.]
- **References**: [products the user named in Phase 1]
- **Tone**: [3 descriptors the user landed on, or N/A]
- **Hard constraints**: [accessibility, dark mode, mobile-first, i18n, etc.]
- **Locale**: primary `[zh-CN | en | ja | ...]`, secondary `[…]`

## 2. Color

### Brand
- `--color-primary`: `#xxxxxx` — usage notes
- `--color-primary-hover`: `#xxxxxx`
- `--color-primary-subtle`: `#xxxxxx` (background tint, ~5–10% alpha or +90% lightness)
- `--color-secondary`: `#xxxxxx` (omit if not used)

### Neutrals (tinted toward [hue or "true gray"])
- `--color-bg`: `#xxxxxx`
- `--color-surface`: `#xxxxxx`
- `--color-border`: `#xxxxxx`
- `--color-text`: `#xxxxxx`
- `--color-text-secondary`: `#xxxxxx`
- `--color-text-muted`: `#xxxxxx`

### Semantic
- `--color-success`: `#xxxxxx`
- `--color-warning`: `#xxxxxx`
- `--color-error`: `#xxxxxx`
- `--color-info`: `#xxxxxx`

### Dark mode (if shipping)
- Override block for the above neutrals + any color that needs adjustment.

## 3. Typography

| Role | Font | Weights | Source |
|---|---|---|---|
| Heading | [name] | [list] | [Google Fonts / Adobe / self-host] |
| Body | [name] | [list] | [source] |
| Mono | [name] | [list] | [source] |

### Type scale (px)
12 / 14 / 16 / 18 / 20 / 24 / 32 / 40 / 48 / 64 — *or whatever scale the project chose*

### Body measure
- Target: [60–75 chars per line for editorial, 50–65 for app body]
- Line-height: [1.4 for UI, 1.6+ for long-form]

## 4. Spacing

- Base unit: `4px` *(or `8px` if applicable)*
- Allowed scale: `4 / 8 / 12 / 16 / 24 / 32 / 48 / 64`
- Density: `compact` | `balanced` | `spacious`
- Off-scale spacing requires justification in code comments.

## 5. Radius

- `--radius-sm`: `Xpx`
- `--radius-md`: `Xpx`
- `--radius-lg`: `Xpx`
- `--radius-full`: `9999px` (chips, avatars only)

## 6. Elevation / shadow

- `--shadow-sm`: `[box-shadow value]`
- `--shadow-md`: `[box-shadow value]`
- `--shadow-lg`: `[box-shadow value]`
- Or note "flat — use borders only" if family is shadow-less.

## 7. Motion

- Vocabulary: `minimal` | `subtle` | `expressive`
- Default duration: `Xms` for micro, `Xms` for state change, `Xms` for overlays
- Easing: `[curve]`
- Allowed motion patterns: [list — e.g. fade, translate+fade, scale+fade for overlays]
- Forbidden (in this project): [list — e.g. bounce, parallax]

## 7a. Container strategy

How container boundaries are expressed in this project. See `references/extended-dimensions.md` for definitions.

- **Strategy**: `border` | `tinted-surface` | `elevation` | `divider` | `none`
- **Per-surface overrides** (if any):
  - dashboard: `[strategy]`
  - marketing: `[strategy]`
  - form / settings: `[strategy]`
  - long-form content: `[strategy]`
- Notes on implementation specifics — e.g. for `tinted-surface`: surface is `[hex]` against bg `[hex]` (a `[X]%` lightness offset). For `elevation`: `--shadow-md` is the default container shadow, `--shadow-lg` for modals only.

## 7b. Icon system

- **Set**: `lucide` | `phosphor` | `heroicons` | `carbon` | `tabler` | `material` | `brand-custom`
- **Weight**: `thin` | `regular` | `bold` | `filled` (set-dependent)
- **Treatment**: `monochrome` | `two-tone` | `brand-tinted`
- **Sizes**: `16 / 20 / 24 px` baseline; `32 / 48 px` for empty states
- **Primary use color**: `currentColor` (inherits text) | `--color-primary` | `--color-text-muted`
- **Mixing**: do not mix sets within this project. If a needed icon is missing in the set, [decision: substitute closest / commission custom / omit].

## 7c. Decoration

Per-surface decoration policy. Each surface specifies what's allowed; surfaces not listed default to `none / none / none`.

| Surface | Gradients | Textures | Motifs |
|---|---|---|---|
| Marketing hero | `none / subtle / expressive` | `none / noise / dot-grid / paper / scan-lines` | `none / geometric / illustration / photography / data-viz` |
| Dashboard | … | … | … |
| Form / settings | … | … | … |
| Long-form content | … | … | … |

Notes:
- For `gradients = subtle`: max 1 per page, and only on [hero bg / accent button / card highlight].
- For `textures`: implementation note (SVG noise filter, CSS background pattern, image asset).
- For `motifs = illustration`: which library / style. For `photography`: cropping rules, treatment.

## 8. Component conventions

### Buttons
- Primary: `[background, text color, padding, radius]`
- Secondary: `[…]`
- Ghost: `[…]`
- Destructive: `[…]`
- Sizes: `sm / md / lg` with concrete pixel values

### Inputs
- Default state, focus ring, error state, disabled state — concrete values

### Cards
- When to use cards (only for actually-grouped content)
- Padding, radius, border vs shadow choice

### Icons
- See section 7b above. (Section 8 only documents per-component icon usage exceptions, if any.)

## 9. Surfaces (templates)

For each surface the project actually has, define how the tokens apply:

- **Dashboard**: [layout grid, card vs whitespace, density]
- **Marketing landing**: [hero treatment, section rhythm]
- **Form / settings**: [grouping, label position, validation timing]
- **Long-form content**: [measure, figure treatment, pull-quote style] (if applicable)

## 10. Anti-patterns for this project

Specific things to avoid in *this* project, drawn from the chosen style family. Be concrete:

- e.g. "No 3-column equal-grid landing pages with stock photos."
- e.g. "Cards never nested. If you need to group inside a card, use spacing or a divider."
- e.g. "Body copy never on saturated brand background."

## 11. Open questions

If any decision was deferred during the consultation, list it here so the next contributor knows it's open, not omitted.
```

---

## Notes for the AI generating this spec

- Fill every value. If a section doesn't apply, write "N/A — [reason]" rather than removing the section.
- Keep the file < 500 lines. If a section is bloating, extract to a sibling file (e.g. `design-spec-components.md`).
- After writing the file, tell the user the path and offer one follow-up (generate `tokens.css`, run `review` mode against an existing page, etc.).
