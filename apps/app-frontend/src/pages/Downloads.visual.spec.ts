import { mount } from '@vue/test-utils'
import { expect, it, vi } from 'vitest'
import { nextTick, ref } from 'vue'

import type { InstallJobSnapshot } from '@/helpers/install'
import { preserveMonotonicProgress } from '@/helpers/install-progress'

import Downloads from './Downloads.vue'

const fixture = vi.hoisted(() => ({ manager: {} as unknown }))

// Keep the actual page and ProgressBar; isolate native APIs and unrelated controls.
vi.mock('@/providers/download-manager', () => ({
	injectDownloadManager: () => fixture.manager,
}))
vi.mock('@/providers/content-install', () => ({
	injectContentInstall: () => ({ showCurseForgeManualDownloads: vi.fn() }),
}))
vi.mock('@/helpers/install', () => ({ download_job_support_details: vi.fn() }))
vi.mock('@/helpers/curseforge', () => ({ listPendingCurseForgeManualDownloads: vi.fn() }))
vi.mock('@/components/ui/modal/MissingModpackContentModal.vue', () => ({
	default: { render: () => null },
}))
vi.mock('vue-router', () => ({
	useRoute: () => ({ query: {} }),
	useRouter: () => ({ push: vi.fn() }),
}))
vi.mock('@modrinth/ui', async () => {
	const { defineComponent, h } = await import('vue')
	const { default: ProgressBar } =
		await import('../../../../packages/ui/src/components/base/ProgressBar.vue')
	const control = defineComponent({
		setup:
			(_, { slots }) =>
			() =>
				h('div', slots.default?.()),
	})
	return {
		...Object.fromEntries(
			[
				'Admonition',
				'Badge',
				'BulletDivider',
				'Button',
				'Card',
				'ConfirmModal',
				'DropdownSelect',
				'EmptyState',
				'NavTabs',
				'StyledInput',
				'Table',
				'TagItem',
			].map((name) => [name, control]),
		),
		ProgressBar,
		defineMessages: (messages: unknown) => messages,
		injectNotificationManager: () => ({ handleError: vi.fn() }),
		useFormatBytes: () => (value: number) => String(value),
		useVIntl: () => ({
			formatMessage: (message: { defaultMessage: string }) => message.defaultMessage,
		}),
	}
})

function downloadingJob(provider: InstallJobSnapshot['provider']): InstallJobSnapshot {
	return {
		job_id: 'curseforge-progress-regression',
		instance_deleted: false,
		kind: 'create_modpack_instance',
		status: 'running',
		execution_mode: 'normal',
		provider,
		target: { type: 'new_instance' },
		phase: 'downloading_content',
		progress: { current: 10, total: 10, secondary: { current: 1000, total: 1000 } },
		details: { type: 'modpack', title: 'Progress regression pack' },
		created: '2026-09-26T06:34:06Z',
		modified: '2026-09-26T06:34:06Z',
		summary: {
			files_completed: 10,
			files_total: 10,
			bytes_downloaded: 1000,
			bytes_total: 1000,
			fallback_count: 0,
		},
		items: [],
	}
}

it.each(['curse_forge', 'local'] as const)(
	'%s extraction clears the previous download percentage and accepts new phase progress',
	async (provider) => {
		const jobs = ref([downloadingJob(provider)])
		fixture.manager = { jobs, activeJobs: jobs, historyJobs: ref([]), legacyDownloads: ref([]) }
		const wrapper = mount(Downloads, { global: { directives: { tooltip: () => {} } } })
		const bar = () => wrapper.get('[role=progressbar]')
		try {
			expect(bar().attributes('aria-valuenow')).toBe('100')
			jobs.value = [
				preserveMonotonicProgress(jobs.value[0], {
					...jobs.value[0],
					phase: 'extracting_overrides',
					progress: null,
				}),
			]
			await nextTick()

			// CurseForge emits no counters here; download summary bytes remain populated.
			expect(bar().attributes('aria-label')).toBe('Extracting overrides')
			expect(bar().attributes('aria-valuenow')).toBeUndefined()
			expect(wrapper.text()).not.toMatch(/[0-9]+%/)
			expect(bar().find('.progress-bar--waiting').exists()).toBe(true)

			// A measured phase uses its own counters, even below the old download value.
			jobs.value = [
				preserveMonotonicProgress(jobs.value[0], {
					...jobs.value[0],
					progress: { current: 1, total: 100 },
				}),
			]
			await nextTick()
			expect(bar().attributes('aria-valuenow')).toBe('1')
			expect(wrapper.text()).toContain('1%')
			expect(bar().find('.progress-bar--waiting').exists()).toBe(false)
		} finally {
			wrapper.unmount()
		}
	},
)
