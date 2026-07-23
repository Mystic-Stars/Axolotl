# Modrinth Monorepo

This is the Modrinth monorepo — it contains all Modrinth projects, both frontend and backend. When entering a project, either to edit or analyse, you should read it's CLAUDE.md.

## Architecture

- **Monorepo tooling:** [Turborepo](https://turbo.build/) (`turbo.jsonc`) + [pnpm workspaces](https://pnpm.io/workspaces) (`pnpm-workspace.yaml`)
- **Frontend:** Vue 3 / Nuxt 3, Tailwind CSS v3
- **Backend:** Rust (Labrinth API), Postgres, Clickhouse
- **Indentation:** Use TAB everywhere, never spaces

### Apps (`apps/`)

| App               | Description                    |
| ----------------- | ------------------------------ |
| `frontend`        | Main Modrinth website (Nuxt 3) |
| `app-frontend`    | Desktop/app frontend (Vue 3)   |
| `app`             | Desktop/app shell (Tauri)      |
| `app-playground`  | Testing playground for app     |
| `labrinth`        | Backend API service            |
| `daedalus_client` | Daedalus client implementation |
| `docs`            | Documentation site (Astro)     |

### Packages (`packages/`)

| Package            | Description                                           |
| ------------------ | ----------------------------------------------------- |
| `ui`               | Shared Vue component library (`@modrinth/ui`)         |
| `assets`           | Styling and auto-generated icons (`@modrinth/assets`) |
| `api-client`       | API client for Nuxt, Tauri, and Node/browser          |
| `app-lib`          | Shared app library                                    |
| `blog`             | Blog system and changelog data                        |
| `utils`            | Shared utility functions (mostly deprecated)          |
| `moderation`       | Moderation utilities                                  |
| `daedalus`         | Daedalus protocol                                     |
| `tooling-config`   | ESLint, Prettier, TypeScript configs                  |
| `ariadne`          | Analytics library                                     |
| `modrinth-log`     | Logging utilities                                     |
| `modrinth-maxmind` | MaxMind GeoIP                                         |
| `modrinth-util`    | General utilities                                     |
| `muralpay`         | Payment processing                                    |
| `path-util`        | Path utilities                                        |
| `sqlx-tracing`     | SQLx query tracing                                    |

## Pre-PR Commands

Run these from the **root** folder before opening a pull request - do not run these after each prompt the user gives you, only run when asked, ask the user a question if they want to run it if the user indicates that they are about to create a pull request.

- **Website:** `pnpm prepr:frontend:web`
- **App frontend:** `pnpm prepr:frontend:app`
- **Frontend libs:** `pnpm prepr:frontend:lib`
- **All frontend (app+web):** `pnpm prepr`
- **Labrinth (backend):** See `apps/labrinth/AGENTS.md`

The website and app `prepr` commands

## Dev Commands

- **Website:** `pnpm web:dev` (copy `.env` template in `apps/frontend/` first)
- **App:** `pnpm app:dev` (copy `.env` template in `packages/app-lib/` first)
- **Storybook (packages/ui):** `pnpm storybook`

## Codex Development Workflow

### Local App Verification

- After completing a development task, start the local development app with `pnpm app:dev` and leave functional verification to the user.
- Do not take screenshots or perform automated, visual, or manual self-testing of the local app.

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
- Before editing the catalog, check the latest launcher version published on the remote through its GitHub release or tag. Do not infer the latest published version from the local package version or the newest catalog entry.
- Use the next patch version after that remote release as the current unreleased announcement. For example, if the latest remote release is `1.4.0`, add changes to `launcher-1.4.1`; if that entry already exists locally, keep updating it until `1.4.1` is published remotely. Only begin `launcher-1.4.2` after the remote publishes `1.4.1`.
- Never append new changes to an announcement that has already been published remotely.
- After every code change or pull request merge, review the current unreleased announcement and add a concise user-facing entry for the change before considering the work complete. This applies to feature work, behavior changes, and bug fixes; do not treat release-note maintenance as optional or defer it to a later release.
- Give every release a new immutable ID in the form `launcher-<version>`, use the exact launcher version and ISO `YYYY-MM-DD` publication date, and place the newest release first. Never reuse an ID, edit a published entry, or change its meaning.
- Use only the Keep a Changelog categories `added`, `changed`, `deprecated`, `removed`, `fixed`, and `security`. Omit empty categories.
- Provide both `en-US` and `zh-CN` text for the title and every change. Other locales intentionally fall back to English; do not copy announcement bodies into every locale JSON file.
- Keep entries concise and user-facing. Describe observable features, behavior changes, removals, fixes, and security impact rather than implementation details.
- `pending_update_toast_for_version` is the persisted trigger for the post-update announcement. Do not add separate per-release startup checks or clear it before the announcement closes.
- Preserve startup dialog priority: initialization errors, first-run onboarding, post-update announcement, then community announcement. A replayed onboarding tour must not consume a pending post-update announcement.
- If the announcement schema, categories, fallback behavior, or dialog priority changes, update the catalog types, both announcement display surfaces, and this section together.
- Keep launcher release notes exclusively in the catalog. Do not create or maintain a separate `UPDATE_LOG.md` file.
- The GitHub release workflow generates its release body from the matching catalog entry with `scripts/axolotl/create-release-notes.mjs`; a release tag without a catalog entry must fail preflight.
- Local development builds expose a preview button in Settings > Updates. Use it to test the real announcement modal without changing onboarding or pending-update state; do not add per-version preview branches.

## Project-Specific Instructions

Each project may have its own file with detailed instructions:

- [`apps/labrinth/AGENTS.md`](apps/labrinth/AGENTS.md) — Backend API
- [`apps/frontend/CLAUDE.md`](apps/frontend/CLAUDE.md) - Frontend Website

## Update Logging Instructions

After making any changes, write the changes as an update log(release note style, in Chinese simplified, using unordered list to append) in UPDATE_LOG.md in the project root directory.
For example: - 修复了xxx问题。
Append the changes to the log, not replacing.

## Code Guidelines

### Comments

- DO NOT use "heading" comments like: `=== Helper methods ===`.
- Use doc comments, but avoid inline comments unless ABSOLUTELY necessary for clarity. Code should aim to be self documenting!

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

Standards available at the @standards/ folder.
