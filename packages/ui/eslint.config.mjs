import baseConfig from '@modrinth/tooling-config/eslint/nuxt.mjs'

export default baseConfig
	.append({
		// Vitest's cache directory (`cacheDir: '.vite'` in vite.config.ts) holds
		// vendored dependency bundles. It is a build artifact, not source.
		ignores: ['.vite/**'],
	})
	.append({
		// TresJS components are resolved by the TresJS Vite plugin and are never
		// imported by name, so the rule cannot see them.
		files: ['src/components/skin/**/*.vue'],
		rules: {
			'vue/no-undef-components': [
				'error',
				{
					ignorePatterns: ['Group', 'primitive', 'Tres[A-Z].*'],
				},
			],
		},
	})
