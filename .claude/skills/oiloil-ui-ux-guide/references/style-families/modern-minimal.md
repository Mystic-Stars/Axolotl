# Style Family: modern-minimal

**Signature**: Spacious, typography-led, restrained color, sharp grid.
**Reference products**: Linear, Vercel, Notion, Stripe, Plaid.
**Best for**: dev tools, SaaS dashboards, startup landing pages, internal enterprise.

## Token defaults

### Color
- **Primary**: a single saturated accent (often blue / teal / violet). Used sparingly.
- **Neutrals**: tinted toward the brand color (e.g. brand `#0066cc` → text `#1a2a3a` rather than `#333`). True grays look sterile in this family.
- **Avoid in this family**: pure black `#000`, pure white `#fff` for large surfaces, generic indigo `#6366f1`, purple-to-blue gradients (Stripe-cliché unless deliberate).

### Typography
- **Heading**: geometric sans — Plus Jakarta Sans, Geist, Manrope, Outfit, DM Sans, Inter (Inter is fine here, the "ban" is a taste preference, not a UX rule).
- **Body**: same family or a paired humanist sans.
- **Mono**: JetBrains Mono, Geist Mono, IBM Plex Mono.

### Radius
- Sharp side: **4 / 8 / 12 px**. Pills only on chips/badges, not on primary buttons.

### Spacing
- 4px base, scale 4 / 8 / 12 / 16 / 24 / 32 / 48.
- Density: balanced or spacious. Compact is uncommon in this family.

### Shadow
- Flat by default. Subtle (1–2px tinted shadow) for elevated surfaces.

### Motion
- Minimal. Fade and small translate+fade for state changes. Scale+fade only for overlays. **Avoid bounce.**

## Anti-patterns *within this family*

These are the things that pull a project out of `modern-minimal`. If the user explicitly wants them, that's fine — but they're stylistically inconsistent with the family.

- All content wrapped in cards with borders + shadow. Use whitespace + typography for hierarchy instead.
- Card-in-card nesting.
- 3-column equal-grid landing pages with stock-photo headers.
- Decorative gradients on every section. Limit to one per page.
- Emoji as section icons. Use Lucide / Phosphor / Heroicons.
- Pure-grey-on-tinted-background body text. Add brand tint to neutrals.

## Typical surfaces

This family does well on:
- Dashboard with sidebar nav + content
- Settings forms
- API documentation
- Marketing landing for B2B SaaS

This family does poorly on:
- Children / education products (too cold)
- Long-form reading (sans body fights long passages — consider `editorial` for blog routes)
- Brand-driven luxury (too utilitarian)

## Notes

The "Modern Minimal" defaults the original SKILL.md treated as universal *belong here* and only here. If the project picked a different family, those defaults do not apply.
