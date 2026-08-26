import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import test from 'node:test'

import { resolveBreadcrumbLabel } from '../../helpers/breadcrumb-label.ts'

const localeFiles = {
	'en-US': '../../locales/en-US/index.json',
	'zh-CN': '../../locales/zh-CN/index.json',
	'zh-TW': '../../locales/zh-TW/index.json',
} as const

const localeMessages = Object.fromEntries(
	Object.entries(localeFiles).map(([locale, path]) => [
		locale,
		JSON.parse(readFileSync(new URL(path, import.meta.url), 'utf8')) as Record<
			string,
			{ message: string }
		>,
	]),
) as Record<keyof typeof localeFiles, Record<string, { message: string }>>

test('upgrade breadcrumb reuses localized Upgrade instance message at runtime', () => {
	let locale: keyof typeof localeFiles = 'en-US'
	const labels = { Upgrade: 'app.instance.upgrade-instance' }
	const resolve = () =>
		resolveBreadcrumbLabel(
			'Upgrade',
			() => '',
			labels,
			(messageId) => localeMessages[locale][messageId].message,
		)

	assert.equal(resolve(), 'Upgrade instance')
	locale = 'zh-CN'
	assert.equal(resolve(), '升级实例')
	locale = 'zh-TW'
	assert.equal(resolve(), '升級實例')
})

test('upgrade route paths, internal names, and breadcrumb depth stay unchanged', () => {
	const routes = readFileSync(new URL('../../routes.js', import.meta.url), 'utf8')
	assert.match(routes, /useRootContext: true,[\s\S]*?breadcrumb: \[\{ name: 'Upgrade' \}\]/)
	assert.doesNotMatch(routes, /breadcrumb: \[\{ name: '\?Instance'[^\]]*\{ name: 'Upgrade' \}\]/)
	for (const [path, name] of [
		['', 'InstanceUpgrade'],
		['compatibility', 'InstanceUpgradeCompatibility'],
		['customize', 'InstanceUpgradeCustomize'],
		['confirm', 'InstanceUpgradeConfirm'],
		['progress', 'InstanceUpgradeProgress'],
		['result', 'InstanceUpgradeResult'],
	] as const) {
		assert.match(routes, new RegExp(`path: '${path}',\\s+name: '${name}'`))
	}
})

test('breadcrumb component resolves Upgrade through formatMessage on each render', () => {
	const source = readFileSync(new URL('./Breadcrumbs.vue', import.meta.url), 'utf8')
	assert.match(source, /Upgrade: messages\.upgradeInstance/)
	assert.match(source, /Upgrade: ArrowBigUpDashIcon/)
	assert.match(source, /id: 'app\.instance\.upgrade-instance'/)
	assert.match(source, /resolveBreadcrumbLabel\([\s\S]*?\(message\) => formatMessage\(message\)/)
})

test('breadcrumb separators render only between visible breadcrumb items', () => {
	const source = readFileSync(new URL('./Breadcrumbs.vue', import.meta.url), 'utf8')
	assert.match(source, /v-for="\(breadcrumb, index\) in breadcrumbs"/)
	assert.match(source, /v-if="index < breadcrumbs\.length - 1"/)
	assert.doesNotMatch(source, /ChevronRightIcon v-if="breadcrumb\.link"/)
})
