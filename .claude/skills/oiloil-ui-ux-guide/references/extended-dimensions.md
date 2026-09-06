# Extended Token Dimensions

The original token model (`color` / `font` / `radius` / `spacing` / `shadow` / `motion`) is enough to describe a *sterile* design system. To capture the things that actually distinguish style families from each other in real product UI, the spec also tracks four additional dimensions:

1. `containerStrategy` — how container boundaries are expressed
2. `iconSystem` — which icon set, weight, and treatment
3. `decoration` — gradients, textures, motifs (mostly for marketing and hero surfaces)
4. `locale` — primary language + secondary supported languages

Each of these is a real visual decision. Skipping them is the reason the original preview template made `modern-minimal` and `warm-content` look identical when rendered.

---

## 1. `containerStrategy`

How do containers (cards, panels, list rows, sections) visually separate from each other?

| Value | What it means | Implications |
|---|---|---|
| `border` | 1px borders around containers, often with a slight surface tint | The default of most SaaS UI. Reads as "designed system". |
| `tinted-surface` | Container surface color is offset from page bg (lighter or darker), no border | iOS 14+ Health, many fintech apps. Feels softer, more depth. |
| `elevation` | No border. Soft shadow does the work. | Material, macOS. Reads as physical, "objects on a desk". |
| `divider` | Containers don't have visible boundaries; horizontal/vertical dividers separate sections within | Editorial, list-heavy interfaces. Quiet. |
| `none` | Pure spacing and typography do the grouping work | Brutalist, manifesto pages, `premium-luxury` heroes. |

A project may pick one strategy globally and override per-surface (e.g. `tinted-surface` everywhere except dashboard tables which use `divider`).

## 2. `iconSystem`

```yaml
iconSystem:
  set: lucide | phosphor | heroicons | carbon | tabler | material | brand-custom
  weight: thin | regular | bold | filled         # set-dependent
  treatment: monochrome | two-tone | brand-tinted
```

Notes:

- `lucide` — the modern default; clean, monoline, regular weight. Works in almost any family.
- `phosphor` — has 6 weights; great for `playful` (bold/filled) and `editorial` (thin/regular).
- `heroicons` — Tailwind ecosystem; outline + solid. Solid is heavier; pick on purpose.
- `carbon` — IBM design system. Reads enterprise.
- `tabler` — close to lucide visually but with more variety. Good for dense `tech-cyberpunk` UI.
- `material` — Google. Carries Material's voice; usually only correct for Android-leaning products.
- `brand-custom` — when the brand has a proprietary set; default for established brands.

The `treatment` matters more than people think:
- `monochrome` — single color (usually text color or muted), recedes.
- `two-tone` — accent + neutral, draws attention. Often `playful` / `warm-content`.
- `brand-tinted` — uses primary at a lower alpha. Works in `modern-minimal` and `tech-cyberpunk`.

## 3. `decoration`

```yaml
decoration:
  gradients: none | subtle | expressive
  textures:  none | noise | dot-grid | paper | scan-lines
  motifs:    none | geometric | illustration | photography | data-viz
```

Definitions:

- `gradients`
  - `none` — no decorative gradients. Solid color blocks only.
  - `subtle` — at most one gradient per page (typically hero background or a single accent button).
  - `expressive` — gradients are part of the visual language (multiple per page, animated where appropriate).
- `textures`
  - `noise` — film grain / noise overlay; widely used in `playful` and `brutal`.
  - `dot-grid` — engineering / blueprint feel; common in `tech-cyberpunk`.
  - `paper` — warm canvas texture; `warm-content`, `editorial`, `premium-luxury`.
  - `scan-lines` — CRT / terminal aesthetic; only `tech-cyberpunk` and very deliberate brutalism.
- `motifs` — what's the secondary visual language alongside copy?
  - `geometric` — abstract shapes (circles, blobs, strokes) used as decoration.
  - `illustration` — characters / scenes; in family for `playful` and `warm-content`.
  - `photography` — used heavily in `premium-luxury`, `editorial`, `warm-content`.
  - `data-viz` — charts / sparklines as decoration; `tech-cyberpunk`, `modern-minimal` SaaS.

Decoration is **per-surface**: marketing pages may go `expressive + illustration`, while the dashboard stays `none / none / data-viz`.

## 4. `locale`

```yaml
locale:
  primary: zh-CN | zh-TW | en | ja | ko | ...
  secondary: [list of supported locales]
```

Why this is a token decision, not a runtime concern:

- **Font shortlist depends on language.** Plus Jakarta Sans is great for Latin; for `zh-CN` you need a CJK-capable font (Noto Sans SC, Source Han Sans, PingFang fallback). For `ja` add Noto Sans JP / Hiragino. The skill should propose locale-appropriate fonts in Phase 3, not generic Latin-only candidates.
- **Line-height and font-size baselines differ.** CJK body needs larger line-height (1.7+) to be comfortable; Latin can do 1.5. Hard-coding 1.5 leaves CJK text feeling cramped.
- **Letter-spacing.** Negative letter-spacing on display headings looks great in Latin and bad in CJK.
- **Demo content.** A user looking at an English `Active users 2,847` dashboard cannot judge how their actual Chinese product will feel. The preview must render in the project's primary language.

When `secondary` is non-empty, the chosen body font must support all listed scripts (or the spec must define a font fallback stack).

---

## Per-family defaults

Each style family has a default *starting point* across the four extended dimensions. The user can always override.

### `modern-minimal`
| Dim | Default | Compatible alternatives |
|---|---|---|
| containerStrategy | `border` | `tinted-surface`, `divider` |
| iconSystem | `lucide` regular monochrome | `tabler`, `heroicons` |
| decoration.gradients | `none` | `subtle` (hero only) |
| decoration.textures | `none` | `dot-grid` (engineering brand) |
| decoration.motifs | `data-viz` (dashboard) / `geometric` (marketing) | — |
| locale | as project | — |

### `editorial`
| Dim | Default | Compatible alternatives |
|---|---|---|
| containerStrategy | `divider` or `none` | `border` (only for code blocks / sidebars) |
| iconSystem | `phosphor` thin or regular monochrome | `lucide` |
| decoration.gradients | `none` | — |
| decoration.textures | `paper` | `none` |
| decoration.motifs | `photography` | `illustration` (for cover art) |
| locale | as project; serif body fonts must support locale | — |

### `brutal`
| Dim | Default | Compatible alternatives |
|---|---|---|
| containerStrategy | `border` (thick, 2-4px) | `none` |
| iconSystem | `brand-custom` ASCII / Unicode glyphs | `tabler` bold, system emoji ironically |
| decoration.gradients | `none` (use color blocks) | — |
| decoration.textures | `noise` or `paper` | `scan-lines` |
| decoration.motifs | `geometric` (raw shapes) | — |
| locale | locale-agnostic; mono fonts must support locale | — |

### `playful`
| Dim | Default | Compatible alternatives |
|---|---|---|
| containerStrategy | `tinted-surface` or `elevation` | `border` (thin, soft color) |
| iconSystem | `phosphor` bold or filled, `two-tone` | `heroicons` solid |
| decoration.gradients | `expressive` | `subtle` |
| decoration.textures | `noise` | `paper` |
| decoration.motifs | `illustration` | `geometric` |
| locale | leans toward rounded fonts in any locale (Nunito ZH, Noto Sans Rounded, etc.) | — |

### `premium-luxury`
| Dim | Default | Compatible alternatives |
|---|---|---|
| containerStrategy | `none` or `divider` | — |
| iconSystem | optional; `phosphor` thin monochrome if used; `brand-custom` preferred | — |
| decoration.gradients | `none` | `subtle` (very dark/very light) |
| decoration.textures | `paper` or `none` | metallic foil (rare) |
| decoration.motifs | `photography` (full-bleed editorial) | `none` |
| locale | serif body; for CJK, prefer Source Han Serif / Songti family | — |

### `tech-cyberpunk`
| Dim | Default | Compatible alternatives |
|---|---|---|
| containerStrategy | `tinted-surface` (dark) | `elevation` (with glow not shadow) |
| iconSystem | `tabler` or `carbon` monochrome | `lucide` |
| decoration.gradients | `subtle` (mesh / neon glow) | `expressive` (deliberate, neon) |
| decoration.textures | `dot-grid` or `scan-lines` | `noise` (film-grain) |
| decoration.motifs | `data-viz` | `geometric` (terminal glyphs) |
| locale | mono fonts must support locale; for CJK: JetBrains Mono + Noto Sans Mono CJK | — |

### `warm-content`
| Dim | Default | Compatible alternatives |
|---|---|---|
| containerStrategy | `tinted-surface` | `border` (warm gray, very low contrast) |
| iconSystem | `phosphor` regular `two-tone` | `lucide` |
| decoration.gradients | `none` | — |
| decoration.textures | `paper` | `none` |
| decoration.motifs | `illustration` | `photography` |
| locale | warm serif body; CJK: Source Han Serif / Noto Serif CJK | — |

### `brand-driven`
All four dimensions derive from brand assets:
- `containerStrategy` — read from existing brand UI; if brand has cards-with-shadows, use `elevation`; if brand never uses borders, use `none`. Look at the brand site's UI patterns.
- `iconSystem` — `brand-custom` if the brand has a set; otherwise pick a neutral one (`lucide`) and document the choice.
- `decoration` — extract from brand collateral. If brand uses noise overlay everywhere, use `noise`. If brand never uses gradients, set `none`.
- `locale` — primary = brand's primary market language. Secondary = whatever the brand publishes in.

If the brand has not made these decisions explicitly, **the consultant's job is to make them and write them into `design-spec.md` so the brand book is extended, not replaced**.

---

## How these dimensions feed into the preview template

When `design-config.js` is written for the preview template:

```js
window.__DESIGN_CONFIG__ = {
  mode: "full",
  // ... existing fields ...
  containerStrategy: "tinted-surface",
  iconSystem: { set: "phosphor", weight: "regular", treatment: "two-tone" },
  decoration: {
    gradients: "subtle",
    textures: "paper",
    motifs: "illustration"
  },
  locale: { primary: "zh-CN", secondary: [] }
};
```

The template (Commit 2) provides switchers for each so the user can A/B inside one config without rewriting it.

## How these dimensions feed into the business mockup

The Phase 4b business mockup MUST honor every one of these dimensions. See `references/business-mockup-contract.md`.
