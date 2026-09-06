# Business Mockup Contract (Phase 4b)

## What this is

In Phase 4b of the `design` flow, the skill generates a **standalone HTML file** that renders the project's actual core surface using the chosen tokens. This file is the user's **final review artifact** before the spec is locked. The user looks at *their own product*, with *their own copy*, in *their own language*, rendered in the candidate design system, and decides whether to ship the spec or iterate.

This file is *not* the same as `design-preview-template.html`:

| | `design-preview-template.html` | business mockup |
|---|---|---|
| Purpose | Quick try-on during exploration | Final ship/iterate decision |
| Content | Generic surfaces (dashboard / marketing / form / ...) | The user's actual product surface |
| Copy | Generic placeholder ("Active users 2,847") | User's domain copy ("待审核投放计划 12 条") |
| Language | English placeholder | Project's primary locale |
| Iteration | Rewrite JSON config; refresh | Regenerate the HTML when tokens change |
| Lifetime | Disposable | Disposable (but kept around per iteration for comparison) |

## Where it lives

`/tmp/business-mockup-<n>.html` where `<n>` increments per iteration. Keeping previous iterations lets the user compare side-by-side.

## The contract — what the generated file MUST satisfy

### 1. Single self-contained file

- One `.html` file. No external assets except Google Fonts CDN.
- All CSS inline in `<style>`.
- All icons inline as SVG (or via a single CDN sprite link if the icon set has one).
- All images either omitted or inline as data URIs / placeholder SVG.

### 2. Token application contract

**Every visual decision in the file must be traceable to a token.** No ad-hoc values. Concretely:

- All colors come from `--color-*` CSS variables defined at the top of the `<style>` block.
- All fonts come from `--font-*` variables and a single `@import` at the top of `<style>`.
- All radii come from `--radius-*`.
- All spacing values come from the project's spacing scale (4 / 8 / 12 / 16 / 24 / 32 / 48 — or whatever scale was chosen). Off-scale spacing is forbidden in the mockup; if you find yourself wanting `padding: 13px;` you've left the contract.
- All shadows come from `--shadow-*` (or are absent if `containerStrategy` is `border` / `divider` / `none`).
- All motion (hover transitions etc.) follows the chosen motion vocabulary — `minimal` ≈ no transitions or 100ms; `subtle` ≈ 200ms ease; `expressive` ≈ 300ms+ with custom easing.

### 3. `containerStrategy` honored

The generated file must implement the chosen container strategy globally:

- `border` — `border: 1px solid var(--color-border)` on cards / panels / list-rows. No `box-shadow` for separation.
- `tinted-surface` — cards use `background: var(--color-surface)` against page `background: var(--color-bg)`, where surface is offset (lighter on light themes, lighter on dark themes by ~3-5% lightness). No border, no shadow.
- `elevation` — `box-shadow: var(--shadow-md)` on cards. No border. Page bg and surface can be the same color.
- `divider` — no enclosing borders. Sections separated by `border-bottom: 1px solid var(--color-border)`.
- `none` — pure spacing. No borders, no shadows. Section separation by margin only.

A mixed implementation (some cards use border, some use elevation) is a contract violation unless the spec explicitly defined per-surface overrides.

### 4. `iconSystem` honored

- Use only icons from the chosen `iconSystem.set`.
- Apply the chosen `weight` (e.g. `phosphor` `bold`).
- Apply the chosen `treatment`:
  - `monochrome` — single fill, usually `currentColor`.
  - `two-tone` — primary + secondary fill (use `--color-primary` + `--color-text-muted` or per-icon two-tone if the set ships with it).
  - `brand-tinted` — `--color-primary` at 0.6-0.8 alpha.
- Don't mix icon sets within the file. If you need an icon the set doesn't have, use the closest match or omit.

### 5. `decoration` honored

If the spec sets `decoration.gradients = "expressive"`, the mockup *should* use multiple gradients (hero bg, accent buttons, decorative shapes). If `none`, the mockup *must* not use gradients anywhere.

If `decoration.textures = "noise"`, apply a noise overlay via SVG filter or data-URI background-image. If `none`, no texture.

If `decoration.motifs = "illustration"`, include 1-2 inline SVG illustrations or geometric figures appropriate to the family. If `none`, no decorative imagery.

### 6. `locale` honored

- All visible copy in `locale.primary`.
- Demo data should look real for that locale: Chinese names for `zh-CN` users (王小明), Japanese for `ja` (田中太郎), realistic numbers / currency formatting (¥ for JP, ¥ or 元 for CN).
- Body line-height >=1.7 for CJK locales, 1.5 for Latin.
- Heading letter-spacing 0 for CJK, may be -0.01em to -0.02em for Latin display sizes.

### 7. Real business surface, not generic

This is the most important requirement. The mockup must show **the user's actual product**, derived from what they said in Phase 1.

If the user said "广告投放计划管理后台":
- The mockup is a list of 投放计划 with state (审核中 / 投放中 / 已暂停 / 已结束), not "Active users 2,847".
- Real-feeling demo data: "618 大促主推 - 美妆护肤" "黑五前置 - 数码周边".

If the user said "病历归档":
- The mockup shows patient records with diagnostic codes, dates, attending physician.
- Not "User", "Owner", "Status" — but "患者", "主治医生", "归档状态".

If the user said "亲子任务卡片":
- The mockup shows task cards with cute icons / illustrations, completion stickers, parent/child role split.
- Not "Project", "Owner" — but "今天的任务", "完成 ✓", with appropriately playful copy and visuals.

The skill picks one or two **core pages** for the mockup, not a full app. Picking the right page matters: choose the user's primary daily-use surface, not the rarely-visited settings page.

### 8. Two-page minimum, three-page maximum

- Page 1: the primary daily-use surface (list / dashboard / main canvas).
- Page 2 (optional): a key flow surface (detail view / form / confirmation).
- Page 3 (optional): if the project has a marketing surface as part of the design system, include a small marketing block; otherwise skip.

Pages live in the same file as `<section>` blocks separated by visual gap and a small in-page nav (or vertical scroll).

### 9. No fake interactivity

The mockup is a static visual. No working JS. Buttons don't navigate. Forms don't submit. Nav links are inert. This keeps the file simple and the focus on visual decisions.

### 10. Header annotation

At the top of the mockup, render a small fixed banner:

```
[Project Name] · business mockup · iteration N
[family] · [containerStrategy] · [iconSystem.set] · [locale.primary]
```

So the user always knows which version they're looking at. Style the banner discretely (high contrast against bg, but small — 12-13px, top-right corner).

---

## How to generate the file

This is the workflow, not output:

1. **Re-read the user's Phase 1 inputs** — what does the product do, who uses it, what surfaces matter.
2. **Pick 1-2 core surfaces** based on (1). If unclear, ask the user one focused question: "Of all the screens in this product, which one would you say users spend the most time on?".
3. **Draft a content outline first** — what real entities go on each surface, what real fields, what real states. Don't start writing HTML yet.
4. **Write the HTML** following the contract. Use the spec's chosen tokens.
5. **Save to `/tmp/business-mockup-<n>.html`** and tell the user where it is.
6. **Open it for the user** (`open /tmp/business-mockup-<n>.html` on macOS).
7. **Ask one open question**: "How does it feel? Anything you'd want to change before we lock the spec?"
8. **Iterate** — if the user wants changes, decide whether they're token changes (re-run Phase 3 to update tokens, then regenerate) or content/copy changes (regenerate with same tokens).

## When to skip Phase 4b

- The user explicitly says "skip the business mockup, just write the spec" → honor it.
- The project is not a single product but a multi-product design system (no single business surface) → use the static template only.
- The user is reviewing for a hypothetical / not-yet-defined product → ask whether to invent a plausible business or stay generic.

## Why the contract matters

Without it, the AI tends to:

- Generate prettier-than-real demo content that doesn't stress the tokens (everything fits because the AI made the data fit).
- Sneak in off-scale spacing because "it looks better here" — at which point the spec is no longer trusted.
- Use a different icon set than agreed for "variety", which is the worst possible thing for a system review.

The contract makes the mockup *useful as evidence* — what the user sees in the mockup is what they get if they apply the spec faithfully. If the mockup looks bad and the contract was honored, the spec is wrong; iterate the spec, not the mockup.
