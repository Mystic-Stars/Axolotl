#!/usr/bin/env node
/**
 * Fails on Tailwind colour utilities whose key no longer exists.
 *
 * The branch removed the `primary`, `secondary` and `contrast` colour keys, but
 * a class naming a removed key is invisible to eslint, to `tsc` and to the
 * build: Tailwind emits nothing at all and the element silently keeps whatever
 * colour it inherits. Both regressions of this class caught in review were
 * exactly that — `text-primary` in the skin preview, and `text-secondary` /
 * `text-contrast` in the settings pages that came back with an upstream merge.
 *
 * Matching is boundary based: the character before the utility and the one after
 * the key both have to be a space or a quote, so the converged spellings
 * (`--color-text-primary`, `text-[var(--color-text-primary)]`) are not misread
 * as the removed key.
 *
 * Usage: node scripts/axolotl/check-dead-colours.mjs
 */
import { readFileSync, readdirSync, statSync } from 'node:fs'
import { join, relative } from 'node:path'

const ROOTS = ['apps/app-frontend/src', 'packages/ui/src', 'apps/website/src']
const EXTENSIONS = ['.vue', '.ts', '.js', '.scss', '.css']

/** `${file}|${utility}` pairs that may stay, each with the reason why. */
const ALLOWED = []

const PREFIXES =
	'bg|text|border|ring|from|to|via|fill|stroke|shadow|outline|divide|accent|caret|placeholder|decoration'

const REMOVED = new Map([
	['primary', 'the key was removed; use text-[var(--color-text-default)]'],
	['secondary', 'the key was removed; use text-[var(--color-text-tertiary)]'],
	['contrast', 'the key was removed; use text-[var(--color-text-primary)]'],
])

const CANDIDATE = new RegExp(`[ "'\`]((?:${PREFIXES})-(?:primary|secondary|contrast))(?=[ "'\`])`, 'g')

const files = []
const collect = (directory) => {
	for (const entry of readdirSync(directory, { withFileTypes: true })) {
		if (entry.name === 'node_modules' || entry.name.startsWith('.')) continue

		const path = join(directory, entry.name)
		if (entry.isDirectory()) {
			collect(path)
			continue
		}
		if (EXTENSIONS.some((extension) => entry.name.endsWith(extension))) files.push(path)
	}
}

for (const root of ROOTS) {
	try {
		if (statSync(root).isDirectory()) collect(root)
	} catch {
		// A root that is not in this checkout (the website is optional) is fine.
	}
}

const allowed = new Set(ALLOWED.map((entry) => entry.split('|')[0] + '|' + entry.split('|')[1]))
const violations = []

for (const file of files) {
	const source = readFileSync(file, 'utf8')
	const lines = source.split('\n')

	for (const [index, line] of lines.entries()) {
		CANDIDATE.lastIndex = 0
		for (const match of line.matchAll(CANDIDATE)) {
			const utility = match[1]
			const key = utility.slice(utility.indexOf('-') + 1)

			if (allowed.has(`${relative('.', file)}|${utility}`)) continue

			violations.push({
				file: relative('.', file),
				line: index + 1,
				utility,
				reason: REMOVED.get(key),
			})
		}
	}
}

if (violations.length > 0) {
	console.error('Colour utilities that name a removed Tailwind key:\n')
	for (const violation of violations) {
		console.error(`  ${violation.file}:${violation.line}  ${violation.utility}  — ${violation.reason}`)
	}
	console.error(
		`\n${violations.length} class(es) would emit no CSS. Replace them with the converged token,` +
			' or add an entry to ALLOWED naming the file, the utility and the reason it may stay.',
	)
	process.exit(1)
}

console.log('No colour utility names a removed Tailwind key.')
