# Style Family Catalog

A "style family" is a coherent bundle of font / color / spacing / radius / shadow / motion choices that work together emotionally and functionally. Style families are **starting points** — the user can override anything inside a chosen family.

**These families are project-chosen, never default-imposed.** If you start a `design` session and impose `modern-minimal` without the user asking, you are doing it wrong.

## How to use this catalog

1. In Phase 1 of the interview, listen to the user's references and constraints.
2. In Phase 2, present 2–4 candidate families based on those signals.
3. Once the user picks one, load `<family>.md` for the specifics — that file becomes the starting token set for Phase 3.
4. Each family file contains its own "anti-patterns to avoid" — those are scoped to that family, not global UX rules. Do not quote one family's restrictions when a different family was chosen.

## Catalog

| Family | One-line signature | Reference products | Best for |
|---|---|---|---|
| [`modern-minimal`](./modern-minimal.md) | Spacious, typography-led, restrained color, sharp grid | Linear, Vercel, Notion | Dev tools, SaaS, startup landing |
| [`editorial`](./editorial.md) | Long-form respect, serif headers, generous measure | Medium, Substack, NYT | Blogs, content platforms, knowledge bases |
| [`brutal`](./brutal.md) | Raw, monospace, high-contrast borders, deliberately rough | Vercel templates, brutalist landing pages | Indie projects, designer tools, statement marketing |
| [`playful`](./playful.md) | Rounded, saturated, bouncy motion, illustrative | Duolingo, MailChimp, early Notion | Education, consumer apps, onboarding-heavy |
| [`premium-luxury`](./premium-luxury.md) | Restrained palette, elegant serifs, generous whitespace, subtle motion | Aesop, Hermès, Apple Music | Hospitality, fashion, high-ticket SaaS |
| [`tech-cyberpunk`](./tech-cyberpunk.md) | Dark-first, monospace, neon accents, high info density | GitHub dark, Vercel docs dark, terminal aesthetics | Dev infra, security, AI tooling, data dashboards |
| [`warm-content`](./warm-content.md) | Warm neutrals, comfortable reading, soft surfaces | Are.na, Notion light, Craft | Personal sites, journals, community tools |
| [`brand-driven`](./brand-driven.md) | All tokens derived from existing brand assets | Custom; the project is the source | Established brands, agency work, B2B with brand guidelines |

## When to combine

Users often want hybrids. Common pairs:

- `modern-minimal` color + `editorial` typography → corporate blog
- `tech-cyberpunk` colors + `modern-minimal` layout → developer dashboard
- `premium-luxury` typography + `warm-content` surfaces → boutique e-commerce

Honor the combination. Note it explicitly in `design-spec.md` so future contributors know which family supplies which dimension.

## When none fits

If the user's references span 3+ unrelated families and they can't pick one, the project is probably **brand-driven** by default — start from their brand assets (logo, brand color) and let everything else follow. Use `brand-driven.md` as the framework.
