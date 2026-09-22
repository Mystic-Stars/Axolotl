<script setup lang="ts">
import {
	ButtonStyled,
	commonMessages,
	defineMessages,
	NewModal,
	useModalStack,
	useVIntl,
} from '@modrinth/ui'
import { renderString } from '@modrinth/utils'
import { getVersion } from '@tauri-apps/api/app'
import { openUrl } from '@tauri-apps/plugin-opener'
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from 'vue'

import {
	announcementKey,
	isAnnouncementActive,
	OPEN_REMOTE_ANNOUNCEMENT_CENTER_EVENT,
	parseAnnouncements,
	REMOTE_ANNOUNCEMENTS_UPDATED_EVENT,
	type RemoteAnnouncement,
	safeAnnouncementUrl,
} from '@/helpers/remote-announcements'
import { getUpdateChannel } from '@/helpers/settings'

const props = defineProps<{ ready: boolean; previewOnly?: boolean }>()
const { formatMessage } = useVIntl()
const { hasModal } = useModalStack()
const messages = defineMessages({
	view: { id: 'app.remote-announcements.view', defaultMessage: 'View announcement' },
	unread: { id: 'app.remote-announcements.unread', defaultMessage: 'Unread' },
	readAll: {
		id: 'app.remote-announcements.read-all',
		defaultMessage: 'Mark all announcements as read',
	},
	centerTitle: { id: 'app.remote-announcements.center-title', defaultMessage: 'Announcements' },
	empty: { id: 'app.remote-announcements.empty', defaultMessage: 'No announcements' },
	previewTitle: {
		id: 'app.remote-announcements.preview-title',
		defaultMessage: 'Announcement style preview',
	},
	previewSummary: {
		id: 'app.remote-announcements.preview-summary',
		defaultMessage:
			'This is a local preview. Select View announcement to preview the full Markdown content.',
	},
	previewContent: {
		id: 'app.remote-announcements.preview-content',
		defaultMessage:
			'## Announcement preview\n\nThis is **sample content**, not a published announcement.\n\n- Supports headings, lists, and links\n- Close buttons are always available\n\n> Previewing does not change real announcement read status.\n\n| Type | Display |\n| --- | --- |\n| Modal | Full Markdown content |\n| Popup | Summary, then full content |\n\n[Visit the website](https://axlmc.org)',
	},
	previewAction: { id: 'app.remote-announcements.preview-action', defaultMessage: 'Visit website' },
})
const modal = ref<InstanceType<typeof NewModal>>()
const selected = ref<RemoteAnnouncement | null>(null)
const active = ref(false)
const centerOpen = ref(false)
const startupNotice = ref<RemoteAnnouncement | null>(null)
const html = computed(() => renderString(selected.value?.content ?? ''))
const stateKey = 'axolotl-remote-announcements-v2'
const read = new Set<string>()
const queuedThisSession = new Set<string>()
const startupNotified = new Set<string>()
let items: RemoteAnnouncement[] = []
let pending: RemoteAnnouncement[] = []
let cacheKey = ''
let endpoint: URL | undefined
let cacheLoaded = false
let inFlight = false
let disposed = false
let lastAttempt = 0
let controller: AbortController | undefined
let interval: ReturnType<typeof setInterval> | undefined
let advanceTimer: ReturnType<typeof setTimeout> | undefined

function persist() {
	if (props.previewOnly) return
	try {
		localStorage.setItem(
			stateKey,
			JSON.stringify({
				read: [...read].slice(-1000),
			}),
		)
	} catch {
		// localStorage may be unavailable (private mode / quota)
	}
}
function emitState() {
	window.dispatchEvent(
		new CustomEvent(REMOTE_ANNOUNCEMENTS_UPDATED_EVENT, {
			detail: {
				items,
				unreadKeys: items.filter((item) => !read.has(announcementKey(item))).map(announcementKey),
			},
		}),
	)
}
function openCenter() {
	if (props.previewOnly || !props.ready || disposed) return
	centerOpen.value = true
	void nextTick(() => modal.value?.show())
}
async function show(item: RemoteAnnouncement) {
	if (
		disposed ||
		!props.ready ||
		!isAnnouncementActive(item) ||
		(hasModal.value && !active.value && !centerOpen.value)
	)
		return
	centerOpen.value = false
	selected.value = item
	active.value = true
	const key = announcementKey(item)
	read.add(key)
	persist()
	emitState()
	pending = pending.filter((entry) => announcementKey(entry) !== key)
	await nextTick()
	if (!disposed) modal.value?.show()
}
function markAllRead() {
	for (const item of items) {
		read.add(announcementKey(item))
	}
	pending = []
	persist()
	emitState()
}
function advance() {
	if (disposed || !props.ready || hasModal.value || active.value) return
	const next = pending.shift()
	if (next) {
		void show(next)
		return
	}
	for (const item of items) {
		const key = announcementKey(item)
		if (
			item.type === 'notification' &&
			!startupNotified.has(key) &&
			!read.has(key) &&
			isAnnouncementActive(item)
		) {
			startupNotice.value = item
			startupNotified.add(key)
			setTimeout(() => {
				if (startupNotice.value === item) startupNotice.value = null
			}, 8000)
			break
		}
	}
}
function closed() {
	active.value = false
	centerOpen.value = false
	if (advanceTimer) clearTimeout(advanceTimer)
	advanceTimer = setTimeout(advance, 350)
}
function sync(next: RemoteAnnouncement[], fresh: boolean) {
	items = next.filter((item) => isAnnouncementActive(item))
	const keys = new Set(items.map(announcementKey))
	pending = pending.filter((item) => keys.has(announcementKey(item)))
	if (selected.value && !keys.has(announcementKey(selected.value))) modal.value?.hide()
	emitState()
	if (fresh) {
		for (const item of items) {
			const key = announcementKey(item)
			if (
				item.type === 'modal' &&
				!queuedThisSession.has(key) &&
				(!read.has(key) || item.priority === 'critical')
			) {
				queuedThisSession.add(key)
				pending.push(item)
			}
		}
		advance()
	}
}
function loadCache() {
	if (cacheLoaded || !cacheKey) return
	cacheLoaded = true
	try {
		const cached = JSON.parse(localStorage.getItem(cacheKey) ?? 'null')
		if (cached && typeof cached.savedAt === 'number' && Date.now() - cached.savedAt < 86400000) {
			const parsed = parseAnnouncements(cached.items)
			if (parsed) sync(parsed, false)
		}
	} catch {
		// Ignore malformed or expired cache payloads
	}
}
async function refresh() {
	if (inFlight || disposed) return
	inFlight = true
	lastAttempt = Date.now()
	const abort = new AbortController()
	controller = abort
	const timeout = setTimeout(() => abort.abort(), 10000)
	try {
		if (!endpoint) {
			const [version, channel] = await Promise.all([getVersion(), getUpdateChannel()])
			endpoint = new URL(
				import.meta.env.VITE_AXO_ANNOUNCEMENTS_URL ||
					'https://admin.axlmc.org/api/public/announcements',
			)
			endpoint.searchParams.set('version', version)
			endpoint.searchParams.set('channel', channel === 'release' ? 'stable' : 'beta')
			cacheKey = stateKey + ':cache:' + endpoint.href
		}
		if (disposed || abort.signal.aborted) return
		loadCache()
		const response = await fetch(endpoint, { signal: abort.signal, credentials: 'omit' })
		if (!response.ok) return
		const text = await response.text()
		if (text.length > 4500000) return
		const result = JSON.parse(text)
		const parsed = parseAnnouncements(result.announcements)
		if (!parsed || disposed) return
		sync(parsed, true)
		try {
			localStorage.setItem(cacheKey, JSON.stringify({ savedAt: Date.now(), items: parsed }))
		} catch {
			// Ignore cache write failures
		}
	} catch {
		// Network/parse failures are non-fatal; next reconnect retries
	} finally {
		clearTimeout(timeout)
		inFlight = false
		controller = undefined
	}
}
async function openLink(value: unknown) {
	const url = safeAnnouncementUrl(value)
	if (!url) return
	try {
		await openUrl(url)
	} catch {
		// Opener may reject unknown schemes
	}
}
function contentClick(event: MouseEvent) {
	const link = event.target instanceof Element ? event.target.closest('a') : null
	if (!link) return
	event.preventDefault()
	event.stopPropagation()
	void openLink(link.getAttribute('href'))
}
function reconnect() {
	if (Date.now() - lastAttempt > 30000) void refresh()
}
function preview(type: RemoteAnnouncement['type'], withAction = false) {
	if (!props.previewOnly || !props.ready || hasModal.value || disposed) return
	const now = new Date().toISOString()
	const item: RemoteAnnouncement = {
		id: 'local-preview-' + type,
		title: formatMessage(messages.previewTitle),
		summary: formatMessage(messages.previewSummary),
		content: formatMessage(messages.previewContent),
		type,
		priority: 'normal',
		starts_at: now,
		ends_at: null,
		published_at: now,
		action_label: withAction ? formatMessage(messages.previewAction) : null,
		action_url: withAction ? 'https://axlmc.org' : null,
	}
	read.clear()
	queuedThisSession.clear()
	sync([item], true)
}
defineExpose({ preview })
onMounted(() => {
	if (props.previewOnly) return
	try {
		const saved = JSON.parse(localStorage.getItem(stateKey) ?? 'null')
		if (saved && Array.isArray(saved.read))
			for (const key of saved.read) if (typeof key === 'string') read.add(key)
	} catch {
		// Ignore malformed local read-state
	}
	window.addEventListener(OPEN_REMOTE_ANNOUNCEMENT_CENTER_EVENT, openCenter)
	const start = () => {
		void refresh()
		interval = setInterval(() => {
			sync(items, false)
			advance()
			if (Date.now() - lastAttempt >= 300000) void refresh()
		}, 15000)
		window.addEventListener('online', reconnect)
	}
	// Defer network polling until after first paint.
	if (typeof requestIdleCallback === 'function') {
		requestIdleCallback(start, { timeout: 2000 })
	} else {
		setTimeout(start, 500)
	}
})
watch([() => props.ready, hasModal], () => {
	if (advanceTimer) clearTimeout(advanceTimer)
	advanceTimer = setTimeout(advance, 350)
})
onUnmounted(() => {
	disposed = true
	controller?.abort()
	if (interval) clearInterval(interval)
	if (advanceTimer) clearTimeout(advanceTimer)
	window.removeEventListener('online', reconnect)
	window.removeEventListener(OPEN_REMOTE_ANNOUNCEMENT_CENTER_EVENT, openCenter)
})
</script>

<template>
	<div
		v-if="startupNotice"
		class="fixed right-4 top-16 z-50 w-[22rem] max-w-[calc(100vw-2rem)] rounded-xl border border-surface-5 bg-surface-3 p-3 shadow-lg"
	>
		<button class="w-full text-left" @click="startupNotice && show(startupNotice)">
			<div class="mb-1 text-xs font-semibold uppercase text-brand">
				{{ formatMessage(messages.unread) }}
			</div>
			<div class="font-semibold text-contrast">{{ startupNotice.title }}</div>
			<div v-if="startupNotice.summary" class="mt-1 line-clamp-2 text-sm text-secondary">
				{{ startupNotice.summary }}
			</div>
		</button>
	</div>
	<NewModal
		ref="modal"
		:header="centerOpen ? formatMessage(messages.centerTitle) : selected?.title"
		:on-hide="closed"
		max-width="640px"
		scrollable
	>
		<div v-if="centerOpen" class="flex flex-col gap-1">
			<div class="mb-2 flex items-center justify-between">
				<span class="font-semibold text-contrast">{{ formatMessage(messages.centerTitle) }}</span>
				<ButtonStyled v-if="items.some((item) => !read.has(announcementKey(item)))">
					<button @click="markAllRead">{{ formatMessage(messages.readAll) }}</button>
				</ButtonStyled>
			</div>
			<div v-if="!items.length" class="py-8 text-center text-sm text-secondary">
				{{ formatMessage(messages.empty) }}
			</div>
			<button
				v-for="item in items"
				:key="announcementKey(item)"
				class="flex items-start gap-2 rounded-lg p-2 text-left hover:bg-button-bg"
				@click="show(item)"
			>
				<span
					class="mt-1.5 size-2 shrink-0 rounded-full"
					:class="read.has(announcementKey(item)) ? 'bg-secondary' : 'bg-red'"
				/>
				<span class="min-w-0 flex-1">
					<span class="block truncate font-medium text-contrast">{{ item.title }}</span>
					<span class="block line-clamp-2 text-xs text-secondary">{{
						item.summary || item.content
					}}</span>
				</span>
			</button>
		</div>
		<div
			v-else
			class="markdown-body break-words"
			@click="contentClick"
			@auxclick="contentClick"
			v-html="html"
		/>
		<template #actions>
			<div class="flex flex-wrap justify-end gap-2">
				<ButtonStyled v-if="!centerOpen"
					><button @click="markAllRead">{{ formatMessage(messages.readAll) }}</button></ButtonStyled
				>
				<ButtonStyled v-if="selected?.action_url && selected.action_label" color="brand">
					<button @click="openLink(selected.action_url)">{{ selected.action_label }}</button>
				</ButtonStyled>
				<ButtonStyled
					><button @click="modal?.hide()">
						{{ formatMessage(commonMessages.closeButton) }}
					</button></ButtonStyled
				>
			</div>
		</template>
	</NewModal>
</template>
