# Axolotl Launcher Monorepo

This repository contains the Axolotl Launcher desktop application and its official website. Read the applicable project instructions before editing either surface.

## Architecture

- **Monorepo tooling:** [Turborepo](https://turbo.build/) (`turbo.jsonc`) + [pnpm workspaces](https://pnpm.io/workspaces) (`pnpm-workspace.yaml`)
- **Frontend:** Vue 3 / Nuxt 3, Tailwind CSS v3
- **Desktop:** Rust / Tauri
- **Indentation:** Use TAB everywhere, never spaces

### Apps (`apps/`)

| App            | Description                          |
| -------------- | ------------------------------------ |
| `app-frontend` | Desktop application frontend (Vue 3) |
| `app`          | Desktop application shell (Tauri)    |
| `website`      | Official Axolotl website (Nuxt 3)    |

### Packages (`packages/`)

| Package                       | Description                              |
| ----------------------------- | ---------------------------------------- |
| `ui`, `assets`, `utils`       | Shared Vue components, assets, utilities |
| `api-client`                  | Public content-service API client        |
| `app-lib`                     | Shared desktop application library       |
| `daedalus`                    | Minecraft metadata protocol              |
| `ariadne`                     | Social and tunnel protocol types         |
| `async-minecraft-ping`        | Minecraft server ping client             |
| `modrinth-content-management` | Content installation model               |
| `path-util`, `tooling-config` | Shared path and tooling configuration    |

## Pre-PR Commands

Run these from the **root** folder before opening a pull request - do not run these after each prompt the user gives you, only run when asked, ask the user a question if they want to run it if the user indicates that they are about to create a pull request.

- **Website:** `pnpm prepr:website`
- **App frontend:** `pnpm prepr:frontend:app`
- **Frontend libs:** `pnpm prepr:frontend:lib`
- **All frontend:** `pnpm prepr`

The website and app `prepr` commands

## Dev Commands

- **Website:** `pnpm website:dev`
- **App:** `pnpm app:dev` (copy `.env` template in `packages/app-lib/` first)

## Codex Development Workflow

### Local App Verification

- Do not take screenshots or perform automated, visual, or manual self-testing of the local app.

### Website Verification

- Do not use browser automation tools to perform excessive testing of website frontend changes (interaction flows, theme toggling, responsive checks, screenshots) unless the user explicitly requests it. The user verifies visual and interaction changes themselves.

### Remote Commits

- Before pushing a remote commit, inspect its changed paths. If it does not change the desktop app (`apps/app/`, `apps/app-frontend/`) or its app-specific dependencies, prevent unnecessary GitHub Actions usage by including `[skip ci]` in the commit message.
- Never use `[skip ci]` for commits that affect the desktop app or its build, packaging, or runtime dependencies.

### Desktop Onboarding Maintenance

When adding or materially changing a desktop app page, route, navigation entry, large user-facing component, core workflow, settings section, or content-management feature under `apps/app-frontend`, assess the Axolotl onboarding experience.

- Update the onboarding when the feature is relevant to a new user's first-use journey or changes an existing guided workflow.
- Define tours and localized message descriptors in `apps/app-frontend/src/components/ui/onboarding/onboardingConfig.ts`. Keep `OnboardingOverlay` presentational and put reusable runtime behavior in `useOnboardingTour`; do not add step-ID-specific branches to either file.
- Add one stable, semantic `data-onboarding-id` to the component that owns each guided target, then reference that ID through `targetId` in the step configuration. Remove targets when their steps are removed.
- Choose the interaction deliberately: `navigate` must use the real control and wait for its route, `activate` must execute the control's original behavior, and `inspect` must explain a region while any non-onboarding click advances without activating the underlying UI.
- Express workflow branches with `branchByTarget` and `nextByCreationPath` in the step configuration instead of hard-coded conditionals. Missing optional targets must time out and skip rather than trap the tour or show placeholder copy.
- Do not add a navigation step for the page that first-run or replay mode already opens. Establish prerequisite route or modal state before starting the tour, then begin with useful page content.
- Keep the step definition, target, interaction, expected route, branch behavior, and English and Simplified Chinese FormatJS copy synchronized with the feature.
- For a major feature that should not appear in the first-run tour, add an appropriate contextual/replayable tour or document why onboarding is not needed.
- Verify first-run and replay modes, target-missing behavior, narrow windows, and guided modals after changing any target. Guided modals must reserve space for the bottom dialogue instead of rendering beneath it.
- Add future mascot assets only through `OnboardingMascotStage`; do not scatter mascot asset references across onboarding steps.

### Desktop Update Announcement Maintenance

Launcher release announcements are bundled with `apps/app-frontend` and shown after a completed app update and in Settings > Updates.

- Add ordinary release announcements only to `apps/app-frontend/src/announcements/catalog.ts`; adding an entry must not require changes to `App.vue`, the updater, or the announcement components.
- Do not edit the announcement catalog unless the user explicitly asks for an update log. After completing each round of changes, ask whether the user wants an update log for that round.
- Ask the user for the exact launcher version before writing the announcement unless they already provided it. Use that version exactly; do not query the remote or infer a version from local metadata to choose one automatically.
- A catalog entry remains editable until its matching release tag has been published to `origin`. Before changing an existing entry, query both `refs/tags/v<version>` and `refs/tags/<version>` with `git ls-remote --tags --refs origin`; when either exists, the release is published and the entry must not change.
- Give every release an ID in the form `launcher-<version>`, use the exact launcher version and ISO `YYYY-MM-DD` publication date, and place the newest release first. An unpublished entry may be amended to include its release changes. Once its release tag is published, never reuse its ID, edit the entry, or change its meaning.
- Use only the Keep a Changelog categories `added`, `changed`, `deprecated`, `removed`, `fixed`, and `security`. Omit empty categories.
- Provide both `en-US` and `zh-CN` text for the title and every change. Other locales intentionally fall back to English; do not copy announcement bodies into every locale JSON file.
- Keep entries concise and user-facing. Describe observable features, behavior changes, removals, fixes, and security impact rather than implementation details.
- `pending_update_toast_for_version` is the persisted trigger for the post-update announcement. Do not add separate per-release startup checks or clear it before the announcement closes.
- Preserve startup dialog priority: initialization errors, first-run onboarding, post-update announcement, then community announcement. A replayed onboarding tour must not consume a pending post-update announcement.
- If the announcement schema, categories, fallback behavior, or dialog priority changes, update the catalog types, both announcement display surfaces, and this section together.
- Keep launcher release notes exclusively in the catalog. Do not create or maintain a separate `UPDATE_LOG.md` file.
- The GitHub release workflow generates its release body from the matching catalog entry with `scripts/axolotl/create-release-notes.mjs`; a release tag without a catalog entry must fail preflight.
- Local development builds expose a preview button in Settings > Updates. Use it to test the real announcement modal without changing onboarding or pending-update state; do not add per-version preview branches.

## Update Logging Instructions

- Do not write or append an update log unless the user explicitly requests one.
- After completing each round of changes, ask whether the user wants an update log for that round. If they do and have not already specified the version, ask for the exact version before writing.
- Do not query a remote release, infer a version from repository metadata, or choose a version automatically. Use the version supplied by the user.
- Follow the project's canonical update-log location and format. Launcher release notes belong in `apps/app-frontend/src/announcements/catalog.ts`; do not create or maintain a separate `UPDATE_LOG.md` file for them.

## Database Migration Safety

- Treat every migration as immutable as soon as it has been applied to any database, including a local development database. Never edit, rename, reorder, or delete an applied migration.
- Before changing an existing migration, query the active development database's `_sqlx_migrations` table for its version. If a successful row exists, stop and create a new migration with a higher version instead.
- Fix migration mistakes only with a new forward migration. Do not make an old migration appear compatible by directly changing `_sqlx_migrations` checksums.
- A historical checksum may be reconciled in application code only when the checksum is explicitly allowlisted, the existing schema is validated structurally, and a tested forward migration brings that schema to the canonical state. Unknown checksums must still fail.
- SQLite table renames do not free global index names. Rebuild migrations must drop or rename legacy indexes before creating replacement indexes.
- Migration tests must cover both a fresh database and an upgrade database containing the exact previous tables, indexes, foreign keys, representative data, malformed provider data, and missing optional data.
- After a migration test, run `PRAGMA foreign_key_check`, verify that legacy tables and indexes are gone, and verify that provider-qualified data did not change provider identity.
- Migrations must not access the network. Cache data used during migration is untrusted and must be validated or ignored without blocking the upgrade.
- Do not start a migration-watching development process while migration files are still being edited. Finish the migration and its upgrade tests first, then start the app once to apply it.

## Code Guidelines

### Temporary Test Files

- Delete test files created only for task-local verification after the tests complete. Do not leave temporary test files in the worktree unless the user explicitly asks to keep them.

### Comments

- DO NOT use "heading" comments like: `=== Helper methods ===`.
- Use doc comments, but avoid inline comments unless ABSOLUTELY necessary for clarity. Code should aim to be self documenting!

### Frontend Styling: Prefer Tailwind Utilities

- Express UI styling with Tailwind utility classes directly on elements. Never add a custom CSS class that only aliases one or a few declarations a utility can express verbatim. BAD: `.foo { margin-top: 0.5rem }` plus `class="foo"`. GOOD: `class="mt-2"`.
- Use the shared tokens and nothing else: the Tailwind preset lives at `packages/tooling-config/tailwind/tailwind-preset.ts`, theme variables in `packages/assets/styles/variables.scss`. Standard mappings: `--gap-xs/sm/md/lg/xl` → `gap-1/2/3/4/6`, `var(--color-secondary)` → `text-secondary`, `var(--color-contrast)` → `text-contrast`, `var(--surface-N)` → `bg-surface-N` / `border-surface-N`, `var(--color-button-bg)` → `bg-button-bg`, `var(--color-divider)` → `border-divider`, `var(--radius-*)` → `rounded-[var(--radius-*)]`. Never reference CSS variables that are not defined in the shared theme (e.g. `--color-text-secondary` is undefined in the desktop app) — an undefined variable silently disables the declaration.
- Custom classes are acceptable only where utilities cannot express the styling:
  - Anchors for `:deep`/child-element/pseudo-element selectors and `[data-size]`-style compound state rules that must keep working;
  - Animated states (`@keyframes`, transitions), `::before`/`::after` with `content`, scrollbar pseudo-elements;
  - Platform-conditional rules (`.mac`/`.windows`), non-standard-breakpoint media queries, and third-party/rendered content (markdown, terminals, canvas, editors);
  - Genuinely reusable visual variants composed with `@apply` (prefer `@apply` over hand-written declarations when composing utilities into a component class).
- In scoped `<style>` blocks, a rule whose declarations each map 1:1 to utilities is a violation: put the utilities on the element and delete the rule. Do not define utility-alias classes even when scoped — they get duplicated across components and drift.
- Do not copy a style block from one component into another; extract a shared component, an `@apply`-based class, or use utilities inline.

## Bash Guidelines

### Output handling

- DO NOT pipe output through `head`, `tail`, `less`, or `more`
- NEVER use `| head -n X` or `| tail -n X` to truncate output
- IMPORTANT: Run commands directly without pipes when possible
- IMPORTANT: If you need to limit output, use command-specific flags (e.g. `git log -n 10` instead of `git log | head -10`)
- ALWAYS read the full output — never pipe through filters

### General

- Do not create new non-source code files (e.g. Bash scripts, SQL scripts) unless explicitly prompted to
- For Frontend, when doing lint checks, only use the `prepr` commands, do not use `typecheck` or `tsc` etc.
- Types in `@modrinth/utils` are considered highly outdated, if a component needs them, check if you can switch said component to use types from `packages/api-client`
- When provided problems, do not say "I didn't introduce these problems" (shifting the blame/effort) - just fix them.

## Edit Tool - Whitespace Handling (CLAUDE ONLY)

The Read tool uses `→` to mark where line numbers end and file content begins.

**Rule:** Copy the EXACT whitespace that appears after the `→` marker.

- Whatever appears between `→` and the code text is what's actually in the file
- That whitespace must be used EXACTLY in Edit tool's old_string
- Don't count arrows, don't interpret - just copy what's after the `→`

**Example:**
14→ private byte tag;
For Edit, use: `		private byte tag;` (copy everything after →, including the two tabs)

**If Edit fails:** Stop and explain the problem. Do not attempt sed/awk/bash workarounds.

**IMPORTANT**: Trust the Read tool output. Copy what's after `→` into Edit immediately. DO NOT verify with sed/od/grep first - that's wasting time and the instructions already tell you to stop if Edit fails, not to pre-verify.

## Standards

Standards available at the @standards/ folder. Read the relevant standard before working on related code:

- @standards/frontend/SURFACE_SYSTEM.md — UI elevation/separator tokens; consult on any UI work using surfaces or strokes
- @standards/frontend/MODALS.md — regular and multistage modal APIs; consult before building or changing any modal
- @standards/frontend/ADDING_API_MODULES.md — adding a new module to packages/api-client
- @standards/frontend/DEPENDENCY_INJECTION.md — the provide/inject DI layer for platform abstraction and page-level context
- @standards/frontend/INTERNATIONALIZATION.md — translatable strings, ICU message format, and i18n conventions
- @standards/frontend/FIGMA_MCP_USAGE.md — translating Figma designs into Vue components via the Figma MCP server
- @standards/maintaining/CHANGELOG.md — changelog entry style; consult when writing or reviewing changelog entries
