import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import test from 'node:test'

import {
	postUpgradeWarningForContent,
	shouldExpandUpgradeWarningsByDefault,
} from './post-upgrade-notice.ts'

const warning = {
	code: 'keep_incompatible' as const,
	contentId: 'content-a',
	relativePath: 'mods/example.jar',
}

test('matches post-upgrade warnings by exact content id or normalized relative path', () => {
	assert.equal(postUpgradeWarningForContent([warning], 'content-a', null), warning)
	assert.equal(postUpgradeWarningForContent([warning], null, 'mods\\example.jar'), warning)
	assert.equal(postUpgradeWarningForContent([warning], null, '././mods/example.jar'), warning)
	assert.equal(postUpgradeWarningForContent([warning], null, 'mods/other.jar'), null)
})

test('matches local preserved resource packs using only their exact normalized path', () => {
	const localWarning = {
		code: 'keep_incompatible' as const,
		contentId: null,
		relativePath: 'resourcepacks/foo.zip',
	}
	assert.equal(
		postUpgradeWarningForContent([localWarning], null, 'resourcepacks/foo.zip'),
		localWarning,
	)
	assert.equal(
		postUpgradeWarningForContent([localWarning], null, 'resourcepacks\\foo.zip'),
		localWarning,
	)
})

test('content id match takes priority over an earlier path-only match', () => {
	const pathMatch = { ...warning, contentId: null }
	const contentMatch = {
		...warning,
		contentId: 'content-a',
		relativePath: 'resourcepacks/foo.zip',
	}
	assert.equal(
		postUpgradeWarningForContent([pathMatch, contentMatch], 'content-a', 'mods/example.jar'),
		contentMatch,
	)
})

test('large warning collections default to collapsed', () => {
	assert.equal(shouldExpandUpgradeWarningsByDefault(5), true)
	assert.equal(shouldExpandUpgradeWarningsByDefault(30), false)
})

test('instance header and content page consume persisted target notice', () => {
	const indexSource = readFileSync(new URL('../pages/instance/Index.vue', import.meta.url), 'utf8')
	const modsSource = readFileSync(new URL('../pages/instance/Mods.vue', import.meta.url), 'utf8')
	assert.match(indexSource, /v-if="postUpgradeNotice"/)
	assert.match(indexSource, /postUpgradeNotice\.targetGameVersion/)
	assert.match(indexSource, /usePostUpgradeNotice\(\(\) => instance\.value\?\.id \?\? props\.id\)/)
	assert.match(modsSource, /usePostUpgradeNotice\(\(\) => props\.instance\.id\)/)
	assert.match(modsSource, /item\.instanceEntryId,[\s\S]*item\.file_path/)
	assert.match(modsSource, /postUpgradeWarningTooltip/)
})

test('notice query uses the target instance value and registered Tauri command', () => {
	const querySource = readFileSync(
		new URL('../composables/usePostUpgradeNotice.ts', import.meta.url),
		'utf8',
	)
	const helperSource = readFileSync(new URL('./instance.ts', import.meta.url), 'utf8')
	const tauriSource = readFileSync(
		new URL('../../../app/src/api/instance.rs', import.meta.url),
		'utf8',
	)
	assert.match(querySource, /\['post-upgrade-notice', instanceId\]/)
	assert.match(querySource, /enabled: computed\(\(\) => toValue\(instanceId\)\.length > 0\)/)
	assert.match(querySource, /throw error/)
	assert.match(helperSource, /plugin:instance\|instance_get_post_upgrade_notice/)
	assert.match(tauriSource, /tauri::generate_handler!\[[\s\S]*instance_get_post_upgrade_notice/)
})

test('result uses Modrinth Card and Accordion for collapsed compatibility warnings', () => {
	const source = readFileSync(
		new URL('../pages/instance/upgrade/UpgradeResultDetails.vue', import.meta.url),
		'utf8',
	)
	assert.match(source, /<Card v-if="warningRows\.length"/)
	assert.match(source, /<Accordion[\s\S]*:open-by-default="warningsExpandedByDefault"/)
	assert.match(source, /<TriangleAlertIcon/)
})

test('runtime icon references use names exported by the assets package', () => {
	const downloadsSource = readFileSync(new URL('../pages/Downloads.vue', import.meta.url), 'utf8')
	const resultSource = readFileSync(
		new URL('../pages/instance/upgrade/UpgradeResultDetails.vue', import.meta.url),
		'utf8',
	)
	const assetsSource = readFileSync(
		new URL('../../../../packages/assets/generated-icons.ts', import.meta.url),
		'utf8',
	)
	assert.match(downloadsSource, /upgrade_unmanaged_instance' \? RefreshCwIcon/)
	assert.doesNotMatch(downloadsSource, /UpdatedIcon/)
	assert.match(resultSource, /import \{[^}]*TriangleAlertIcon[^}]*\} from '@modrinth\/assets'/)
	assert.doesNotMatch(resultSource, /WarningIcon/)
	assert.match(assetsSource, /export const RefreshCwIcon = _RefreshCwIcon/)
	assert.match(assetsSource, /export const TriangleAlertIcon = _TriangleAlertIcon/)
})
