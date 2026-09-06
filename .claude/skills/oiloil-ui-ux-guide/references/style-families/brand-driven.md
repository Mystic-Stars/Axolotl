# Style Family: brand-driven

**Signature**: Tokens derived from existing brand assets. The project's brand is the source, not a style family.
**Reference products**: Custom — anything where the brand book exists and dictates visual choices.
**Best for**: established companies with brand guidelines, agency work, B2B with strict CI/CD (corporate identity), products that ship across brand-owned surfaces (web + email + print).

## How this family is different

The other 7 families ship with token defaults. `brand-driven` has none — by definition, the brand supplies them. This file is a *framework* for extracting tokens from brand assets, not a token set.

## Inputs you need from the user

In Phase 1, ask explicitly:

1. **Logo** — vector file (SVG / AI / Figma) preferred, otherwise highest-res PNG.
2. **Brand color(s)** — hex codes for primary, optional secondary, semantic colors if defined.
3. **Brand fonts** — font names + weights + license info (Adobe Fonts, self-hosted, Google Fonts). If proprietary, ask about web fallback.
4. **Brand book / style guide** — PDF or web link if one exists.
5. **Voice / tone notes** — affects copy, but also informs density/motion choices.

If the user doesn't have all of these → either
- (a) start from what they have and treat the rest as open in Phase 2 (use neutral defaults), or
- (b) flag that the project might *not* be brand-driven and re-run Phase 2 with a real style family.

## Token derivation rules

When you do have brand assets:

### Color
- Primary = brand primary, exactly as specified.
- Hover / active = derived using OKLCH or HSL lightness shift (typically -8% to -12% lightness for hover, -16% to -20% for active).
- Subtle (background tint) = primary at 5–10% alpha or +90% lightness.
- Neutrals = tinted toward the brand primary's hue (not pure gray) unless the brand book specifies.
- Semantic (success/warning/error/info) = the brand book's versions if defined; otherwise stay close to web defaults but tinted toward the brand temperature.

### Typography
- Headings = brand display font.
- Body = brand body font, or a free fallback if licensing prevents web use. Always provide system-font fallback stack.
- Mono = the brand's mono if specified, otherwise a neutral choice that doesn't fight the brand.

### Spacing / radius / shadow
- The brand book usually does *not* specify these for the web. Pick defaults that match the brand's emotional register:
  - Sharp brand → sharp radius (2–4px), flat shadow.
  - Friendly brand → generous radius (12–16px), soft shadow.
  - Premium brand → 0–2px radius, no shadow, lots of whitespace.

### Motion
- If brand book mentions motion principles, follow them.
- If not, choose motion vocabulary based on brand tone — premium = slow, playful = bouncy, technical = snappy.

## Cross-surface consistency

Brand-driven projects often appear on:
- Web app
- Marketing site
- Email
- Print collateral
- Mobile app

When generating `design-spec.md` for brand-driven, include a **cross-surface notes** section that flags what should stay consistent and what should adapt (e.g. body font might be system on email, web on app).

## Anti-patterns

- Treating the brand color as decoration rather than the primary signal.
- Auto-generating a 9-tier color scale without checking if the brand has its own scale.
- Replacing the brand fonts with "what looks more modern" on the web. The brand's font is the brand's voice; if you don't like it, talk to the user, don't override.
- Ignoring CJK / multi-script considerations when the brand operates internationally.

## Notes

About 30–40% of real-world projects are brand-driven and the consultant's job is largely to *extract and codify* what already exists, not to invent. If the user keeps saying "we already use X for that", the project is probably brand-driven and you should switch families.
