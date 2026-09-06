# Icons (No Emoji, Modern Minimal)

## Hard rules

- Do not use emoji as icons (or decoration).
- Use one icon family across the product. Do not mix outlined/filled/3D/emoji styles.
- Prefer obvious meanings over clever metaphors. If an icon can be misunderstood, add a text label.

## “Intuitive + refined” checklist

- **Style consistency**: same stroke weight (outline) or same fill style (filled).
- **Sizes**: standardize on 16/20/24 (or your system sizes); avoid random sizes per screen.
- **Optical alignment**: align visually (icon bounding boxes lie; nudge when needed).
- **Touch targets**: icon buttons still need adequate hit area; do not shrink interactive area to the glyph.
- **Labels**: primary actions should be text or text+icon; icon-only is reserved for universally-known actions.
- **Tooltips**: tooltips are support, not the primary way to understand an action.

## Prefer text over icons when

- The action is uncommon in your product.
- The icon is domain-specific (users won’t share the same mental model).
- The action is destructive or high-stakes (use explicit wording).

## Suggested icon sets (pick one; do not mix)

- Lucide / Feather-style outline icons (web-friendly)
- Material Symbols (outlined or rounded; pick one)
- SF Symbols (Apple platforms)

## Common mappings (use cautiously)

- Search: magnifier
- Filter: funnel
- Settings: gear
- More actions: kebab (vertical three dots)
- Close: x
- Back: left arrow
- Info: i in circle (use sparingly; don’t turn UI into a tooltip museum)

If an icon is not instantly clear, prefer a short label instead of inventing a new icon metaphor.

