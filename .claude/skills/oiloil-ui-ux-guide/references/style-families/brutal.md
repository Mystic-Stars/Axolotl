# Style Family: brutal

**Signature**: Raw, monospace, high-contrast borders, deliberately rough, system-font fallbacks visible.
**Reference products**: Many indie maker landing pages, Vercel templates (brutalist variants), Bauhaus-revival sites, Cobalt.tools.
**Best for**: indie projects, designer portfolios, statement marketing, hacker culture products.

## Token defaults

### Color
- **Primary**: high-saturation block colors — pure red, pure yellow, pure blue, pure black.
- **Background**: unapologetic — pure white `#ffffff` or pure black `#000000`. The "no pure black" rule from `modern-minimal` does not apply here; pure black is the point.
- **Borders**: thick (2–4px), pure black, on every container. Borders carry the visual weight that elsewhere is carried by shadows.

### Typography
- **Heading**: monospace, system font, or quirky display — JetBrains Mono, Space Mono, IBM Plex Mono, Departure Mono, Times New Roman (deliberate retro).
- **Body**: monospace or system serif. Geometric sans-serif is *out of family*.
- **Weights**: extreme contrast — 400 vs 800, no in-between.

### Radius
- **0 px**. Sharp corners always. This is the family's most non-negotiable trait.

### Spacing
- 8px base. Scale: 8 / 16 / 24 / 40 / 64.
- Spacing is uneven on purpose — asymmetric layouts are encouraged.

### Shadow
- **Hard shadows only**. `box-shadow: 4px 4px 0 0 #000;` (no blur). No soft elevation.

### Motion
- Snappy or none. No easing curves. Linear or step transitions. Hover states can be aggressive (color invert, hard offset).

## Anti-patterns *within this family*

- Soft shadows.
- Rounded corners.
- Pastels.
- "Design system polish" — uniform spacing, perfect alignment, careful hierarchy. Brutal embraces friction.
- Fade animations.
- Lucide / Phosphor icons. Use ASCII characters, emoji as ironic decoration, or system Unicode glyphs.

## Typical surfaces

This family does well on:
- Single-page indie landing
- Developer tool marketing
- Manifesto / values pages
- Designer portfolios

This family does poorly on:
- Anything where users spend more than ~10 minutes per session — the friction wears
- Enterprise / regulated industries
- Consumer apps with broad demographic reach

## Notes

Brutal is a statement, not a default. If the user picked it without realizing what it implies (no rounded corners, no soft shadows, no neat alignment), pause and double-check before generating tokens.
