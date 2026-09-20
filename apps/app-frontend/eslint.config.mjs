import config from '@modrinth/tooling-config/eslint/nuxt.mjs'

export default config
	.append({
		rules: {
			// The shared package exposes a barrel; reaching past it couples the app
			// to internal file layout, which is how 45 deep imports accumulated.
			'no-restricted-imports': [
				'error',
				{
					patterns: [
						{
							group: ['@modrinth/ui/src/*'],
							message:
								'Import from the "@modrinth/ui" barrel instead of reaching into its source. If the symbol is missing from the barrel, export it there.',
						},
					],
				},
			],
		},
	})
	.append({
		// `node --test` resolves `@modrinth/ui` to `node.ts`, a deliberate Node
		// stub that exports only the i18n helpers. Pure-logic modules that a
		// `node --test` file imports therefore cannot use the barrel and name
		// their module directly, which the rule must allow.
		files: ['src/**/*.test.ts', 'src/lab/schematic-preview/instance-files.ts'],
		rules: {
			'no-restricted-imports': 'off',
		},
	})
