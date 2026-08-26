import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import test from 'node:test'

import type { InstanceUpgradeResult } from '@/helpers/instance-upgrade'

import { shouldExpandUpgradeWarningsByDefault } from '../../../helpers/post-upgrade-notice.ts'
import {
	filterUpgradeWarnings,
	paginateUpgradeWarnings,
	summarizeUpgradeWarnings,
	UPGRADE_WARNING_PAGE_SIZE,
	upgradeResultWarningRows,
	upgradeWarningDisplayName,
	upgradeWarningMessageId,
} from './upgrade-warning.ts'

const base = {
	planId: 'plan',
	sourceInstanceId: 'source',
	targetInstanceId: 'target',
	backupInstanceId: null,
	solution: { kind: 'custom', selections: [], dependencyChanges: [], warnings: [] },
	externalChanges: [],
	skippedDueToExternalConflict: [],
} as InstanceUpgradeResult

test('structured warning maps by stable code', () => {
	const rows = upgradeResultWarningRows({
		...base,
		compatibilityWarnings: [],
		compatibilityWarningDetails: [
			{
				code: 'keep_incompatible',
				relativePath: 'mods/a.jar',
				contentId: 'a',
				provider: 'modrinth',
				projectId: 'project',
				conflictingProjectId: null,
			},
		],
	})
	assert.equal(upgradeWarningMessageId(rows[0].code!), 'instance.upgrade.warning.keep-incompatible')
	const zhCn = JSON.parse(
		readFileSync(new URL('../../../locales/zh-CN/index.json', import.meta.url), 'utf8'),
	) as Record<string, { message: string }>
	const localized = zhCn[upgradeWarningMessageId(rows[0].code!)]?.message
	assert.equal(localized, '{path} 已原样保留，可能与升级后的实例不兼容。')
	assert.doesNotMatch(localized, /will be preserved/i)
})

test('legacy persisted warning falls back to raw message', () => {
	const rows = upgradeResultWarningRows({
		...base,
		compatibilityWarnings: [
			{
				code: 'unidentified',
				message: 'Legacy backend text',
				contentId: null,
				provider: null,
				projectId: null,
				conflictingProjectId: null,
				dependencyRequirements: [],
			},
		],
	})
	assert.equal(rows[0].legacyMessage, 'Legacy backend text')
})

test('300 structured warnings stay summarized with path as secondary data', () => {
	const rows = upgradeResultWarningRows({
		...base,
		compatibilityWarnings: [],
		compatibilityWarningDetails: Array.from({ length: 300 }, (_, index) => ({
			code:
				index < 200 ? 'unidentified' : index < 275 ? 'no_compatible_release' : 'prerelease_only',
			relativePath: `resourcepacks/example-${index}.zip`,
			contentId: `content-${index}`,
			provider: null,
			projectId: null,
			conflictingProjectId: null,
		})),
	})
	assert.deepEqual(summarizeUpgradeWarnings(rows), { local: 200, kept: 75, fallback: 25 })
	assert.equal(upgradeWarningDisplayName(rows[0]), 'example-0.zip')
	assert.equal(shouldExpandUpgradeWarningsByDefault(rows.length), false)
	const zhCn = JSON.parse(
		readFileSync(new URL('../../../locales/zh-CN/index.json', import.meta.url), 'utf8'),
	) as Record<string, { message: string }>
	assert.equal(
		zhCn['instance.upgrade.result.warning-unidentified-headline']?.message,
		'此内容在升级时被原样保留',
	)

	const page1 = paginateUpgradeWarnings(rows, 1)
	const page2 = paginateUpgradeWarnings(rows, 2)
	const lastPage = paginateUpgradeWarnings(rows, 30)
	assert.equal(UPGRADE_WARNING_PAGE_SIZE, 10)
	assert.equal(page1.items.length, 10)
	assert.deepEqual(
		page2.items.map((row) => row.contentId),
		Array.from({ length: 10 }, (_, index) => `content-${index + 10}`),
	)
	assert.deepEqual(
		lastPage.items.map((row) => row.contentId),
		Array.from({ length: 10 }, (_, index) => `content-${index + 290}`),
	)
	const remainderPage = paginateUpgradeWarnings(rows.slice(0, 293), 30)
	assert.deepEqual(
		remainderPage.items.map((row) => row.contentId),
		['content-290', 'content-291', 'content-292'],
	)
	const searched = filterUpgradeWarnings(rows, 'all', 'example-287')
	assert.deepEqual(
		searched.map((row) => row.contentId),
		['content-287'],
	)
	assert.equal(paginateUpgradeWarnings(searched, 1).items.length, 1)
	assert.equal(filterUpgradeWarnings(rows, 'all', '').length, 300)
	assert.equal(filterUpgradeWarnings(rows, 'local', '').length, 200)
	assert.equal(filterUpgradeWarnings(rows, 'kept', '').length, 75)
	assert.equal(filterUpgradeWarnings(rows, 'fallback', '').length, 25)

	const source = readFileSync(new URL('./UpgradeResultCollections.vue', import.meta.url), 'utf8')
	assert.match(source, /v-if="warningsOpen"/)
	assert.match(source, /v-for="warning in warningPage\.items"/)
	assert.doesNotMatch(source, /v-for="warning in warnings"/)
	assert.match(source, /technicalDetails/)
	assert.match(source, /warningHeadline\(warning\)/)
	assert.match(
		source,
		/watch\(\[warningSearch, warningFilter\],[\s\S]*?warningPageNumber\.value = 1/,
	)

	const buttonSlotStart = source.indexOf('<template #button="{ open }">')
	const buttonSlotEnd = source.indexOf('</template>', buttonSlotStart)
	const summaryPosition = source.indexOf('warningSummary.local', buttonSlotStart)
	assert.ok(
		buttonSlotStart >= 0 && summaryPosition > buttonSlotStart && summaryPosition < buttonSlotEnd,
	)
	assert.match(source, /class="block w-full"/)
	assert.match(source, /button-class="[^"]*w-full[^"]*focus-visible:ring-4/)

	const accordionSource = readFileSync(
		new URL('../../../../../../packages/ui/src/components/base/Accordion.vue', import.meta.url),
		'utf8',
	)
	const accordionButtonSlot = accordionSource.lastIndexOf('<slot name="button"')
	const accordionButtonStart = accordionSource.lastIndexOf('<button', accordionButtonSlot)
	const accordionButtonEnd = accordionSource.indexOf('</button>', accordionButtonSlot)
	assert.ok(
		accordionButtonStart >= 0 &&
			accordionButtonSlot > accordionButtonStart &&
			accordionButtonSlot < accordionButtonEnd,
	)
})
