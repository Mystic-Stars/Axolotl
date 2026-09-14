import { fixupPluginRules } from '@eslint/compat'
import { createConfigForNuxt } from '@nuxt/eslint-config/flat'
import turboPlugin from 'eslint-plugin-turbo'
import common from './common.mjs'

export const configurationNuxtToAppend = [
	...common,
	{
		name: 'turbo',
		plugins: {
			turbo: fixupPluginRules(turboPlugin),
		},
		rules: {
			'turbo/no-undeclared-env-vars': 'error',
		},
	},
	{
		name: 'modrinth',
		rules: {
			'vue/html-self-closing': 'off',
			'vue/multi-word-component-names': 'off',
			'vue/no-undef-components': [
				'error',
				{
					ignorePatterns: [
						'NuxtPage',
						'NuxtLayout',
						'NuxtLink',
						'NuxtRouteAnnouncer',
						'ClientOnly',
						'Teleport',
						'Transition',
						'TransitionGroup',
						'Head',
						'Title',
						'router-link',
						'RouterView',
						'RouterLink',
						'nuxt-link',
					],
				},
			],
			'vue/no-undef-properties': 'warn',
			// Optional props are common across the design system; defaults would
			// only silence lints without changing runtime behavior for omitted props.
			'vue/require-default-prop': 'off',
			// Markdown/rich-text surfaces intentionally render sanitized HTML.
			'vue/no-v-html': 'off',
		},
		languageOptions: {
			parserOptions: {
				warnOnUnsupportedTypeScriptVersion: false,
			},
		},
	},
]

export default createConfigForNuxt().append(configurationNuxtToAppend)
