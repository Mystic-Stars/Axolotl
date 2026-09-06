# Style Family: tech-cyberpunk

**Signature**: Dark-first, monospace, neon accents, high info density, terminal/IDE aesthetics.
**Reference products**: GitHub dark, Vercel docs dark, Cursor, Warp, Raycast, terminal apps, Cyberpunk 2077 menus.
**Best for**: developer infrastructure, security tools, AI tooling, observability dashboards, command-palette apps.

## Token defaults

### Color
- **Background**: dark by default — `#0a0a0b` to `#18181b` range. Pure black `#000` is acceptable.
- **Primary accent**: neon-leaning — electric blue `#3b82f6`, lime `#84cc16`, magenta `#d946ef`, cyan `#06b6d4`.
- **Text**: off-white with slight cool cast (`#e4e4e7`), not pure white.
- **Borders**: low-opacity white (`rgba(255,255,255,0.08–0.12)`) — barely visible structural lines.
- **Light mode**: optional and secondary. Many products in this family don't ship light mode; if they do, it's clearly not the headline experience.

### Typography
- **Heading**: geometric sans, mono, or hybrid — Inter, Geist, Space Grotesk, JetBrains Mono.
- **Body**: same as heading or paired sans.
- **Mono**: critical and prominent — JetBrains Mono, Geist Mono, Berkeley Mono, IBM Plex Mono. Used for code, IDs, command shortcuts, status text.
- **Mono in chrome**: keyboard hints `⌘K`, status badges, version numbers all live in mono.

### Radius
- **2 / 4 / 6 px**. Sharp but not aggressively so. 8px+ feels too soft for the family.

### Spacing
- 4px base, scale 4 / 8 / 12 / 16 / 24.
- Density: **compact**. The family thrives on info density. Spacious wastes the dark canvas.

### Shadow
- Often replaced by glow / inner shadow. Outer shadows on dark mode read as halos.
- Subtle inner shadow on inputs to imply depth.

### Motion
- Snappy (100–200ms). No bounce. State changes are crisp on/off.
- Selection / focus uses a glow ring or thin colored border.

## Anti-patterns *within this family*

- Light gray text on dark background (`#888` on `#000` fails contrast and looks generic). Use cool off-whites.
- Rounded pill buttons.
- Soft pastel illustrations.
- Stock-photo hero images.
- Whitespace as the design (this family is the opposite of `premium-luxury`).
- Avoiding monospace entirely. Mono is the family's voice.
- Toast notifications with bouncy entry.

## Typical surfaces

This family does well on:
- Developer dashboards (deployments, logs, traces)
- Command palettes
- Code editors and IDE-adjacent surfaces
- Observability and security tooling
- Terminal-inspired marketing pages

This family does poorly on:
- Marketing aimed at non-developer buyers (CFO, HR)
- Long-form reading
- Children / education
- Anything that needs to feel approachable to non-technical users

## Notes

The "AI tooling" wave (2024–) has made this family overused — every dev tool now ships with the same dark + neon accent + mono look. If the project is in this family, lean on a *specific* mono / accent combo that's not literal copies of GitHub or Vercel.
