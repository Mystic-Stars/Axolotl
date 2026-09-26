<script setup lang="ts">
import {
	defineMessages,
	type KeyBinding,
	KeybindingChips,
	KeybindingModal,
	Toggle,
	useVIntl,
} from '@modrinth/ui'
import { computed, ref } from 'vue'

import { getNavShortcutEnabled, setNavShortcutEnabled } from '@/helpers/nav-shortcut-state'
import { getQuickScrollEnabled, setQuickScrollEnabled } from '@/helpers/scroll-top-state'
import { NAV_ACTIONS, SCROLL_ACTIONS, type ShortcutAction } from '@/helpers/shortcut-actions'
import {
	buildConflicts,
	clearStoredBinding,
	resolveAllBindings,
	storeBinding,
} from '@/helpers/shortcut-bindings'
import { useTheming } from '@/store/theme'

import SettingsRow from './SettingsRow.vue'
import SettingsSection from './SettingsSection.vue'

const { formatMessage } = useVIntl()
const themeStore = useTheming()

themeStore.quickScrollEnabled = getQuickScrollEnabled()
themeStore.shortcutBindings = resolveAllBindings()
for (const action of NAV_ACTIONS) {
	themeStore[action.enabledField] = getNavShortcutEnabled(action.id)
}

const messages = defineMessages({
	scrollTitle: {
		id: 'app.shortcut-settings.scroll-title',
		defaultMessage: 'Quick scrolling',
	},
	scrollDescription: {
		id: 'app.shortcut-settings.scroll-description',
		defaultMessage: 'Move through long pages with the keyboard.',
	},
	enable: { id: 'app.shortcut-settings.enable', defaultMessage: 'Enable quick scrolling' },
	navTitle: {
		id: 'app.shortcut-settings.nav-title',
		defaultMessage: 'Navigation shortcuts',
	},
	navDescription: {
		id: 'app.shortcut-settings.nav-description',
		defaultMessage: 'Jump to a menu item with its shortcut. Each shortcut is off until enabled.',
	},
	changeShortcut: {
		id: 'app.shortcut-settings.change-shortcut',
		defaultMessage: 'Change the shortcut for {action}',
	},
})

function actionLabel(action: ShortcutAction) {
	return formatMessage(action.label)
}

function actionDescription(action: ShortcutAction) {
	return formatMessage(action.description)
}

function bindingFor(action: ShortcutAction): KeyBinding {
	return themeStore.shortcutBindings[action.id] ?? action.defaultBinding
}

function sameBinding(left: KeyBinding, right: KeyBinding) {
	return (
		left.device === right.device &&
		left.code === right.code &&
		left.mod === right.mod &&
		left.shift === right.shift &&
		left.alt === right.alt
	)
}

const editing = ref<ShortcutAction | null>(null)
const keybindingModal = ref<InstanceType<typeof KeybindingModal>>()

const editingLabel = computed(() => (editing.value ? actionLabel(editing.value) : undefined))
const editingBinding = computed(() => (editing.value ? bindingFor(editing.value) : null))
const editingDefault = computed(() => editing.value?.defaultBinding ?? null)
const editingConflicts = computed(() =>
	editing.value
		? buildConflicts(editing.value.id, (id) => {
				const action = [...SCROLL_ACTIONS, ...NAV_ACTIONS].find((candidate) => candidate.id === id)
				return action ? actionLabel(action) : id
			})
		: [],
)

function openShortcut(action: ShortcutAction) {
	editing.value = action
	keybindingModal.value?.show()
}

function saveShortcut(binding: KeyBinding) {
	const action = editing.value
	if (!action) return

	// A combination matching the default is stored as "not changed", so a later
	// change to the default still reaches this shortcut.
	if (sameBinding(binding, action.defaultBinding)) clearStoredBinding(action.id)
	else storeBinding(action.id, binding)

	themeStore.shortcutBindings = { ...themeStore.shortcutBindings, [action.id]: binding }
}

function toggleQuickScroll(value: unknown) {
	themeStore.quickScrollEnabled = !!value
	setQuickScrollEnabled(themeStore.quickScrollEnabled)
}

function toggleNavShortcut(action: ShortcutAction, value: unknown) {
	themeStore[action.enabledField] = !!value
	setNavShortcutEnabled(action.id, !!value)
}
</script>

<template>
	<div class="flex flex-col gap-6">
		<SettingsSection>
			<template #header>
				<h2 class="m-0 text-lg font-semibold text-[var(--color-text-primary)]">
					{{ formatMessage(messages.scrollTitle) }}
				</h2>
				<p class="m-0 mt-1 text-sm leading-relaxed text-[var(--color-text-tertiary)]">
					{{ formatMessage(messages.scrollDescription) }}
				</p>
			</template>
			<SettingsRow>
				<template #label>
					<span id="settings-target-shortcuts-enable" tabindex="-1">
						{{ formatMessage(messages.enable) }}
					</span>
				</template>
				<template #control>
					<Toggle
						id="quick-scroll-enabled"
						:model-value="themeStore.quickScrollEnabled"
						@update:model-value="toggleQuickScroll"
					/>
				</template>
			</SettingsRow>
			<SettingsRow v-for="action in SCROLL_ACTIONS" :key="action.id">
				<template #label>
					<span :id="`settings-target-${action.id}`" tabindex="-1">{{ actionLabel(action) }}</span>
				</template>
				<template #description>{{ actionDescription(action) }}</template>
				<template #control>
					<button
						type="button"
						class="keybinding-trigger"
						:aria-label="formatMessage(messages.changeShortcut, { action: actionLabel(action) })"
						@click="openShortcut(action)"
					>
						<KeybindingChips :binding="bindingFor(action)" />
					</button>
				</template>
			</SettingsRow>
		</SettingsSection>

		<SettingsSection>
			<template #header>
				<h2
					id="settings-target-shortcuts-nav"
					tabindex="-1"
					class="m-0 text-lg font-semibold text-[var(--color-text-primary)]"
				>
					{{ formatMessage(messages.navTitle) }}
				</h2>
				<p class="m-0 mt-1 text-sm leading-relaxed text-[var(--color-text-tertiary)]">
					{{ formatMessage(messages.navDescription) }}
				</p>
			</template>
			<SettingsRow v-for="action in NAV_ACTIONS" :key="action.id">
				<template #label>{{ actionLabel(action) }}</template>
				<template #description>{{ actionDescription(action) }}</template>
				<template #control>
					<div class="flex flex-wrap items-center justify-end gap-3">
						<button
							type="button"
							class="keybinding-trigger"
							:aria-label="formatMessage(messages.changeShortcut, { action: actionLabel(action) })"
							@click="openShortcut(action)"
						>
							<KeybindingChips :binding="bindingFor(action)" />
						</button>
						<Toggle
							:id="`nav-shortcut-${action.id}`"
							:model-value="themeStore[action.enabledField]"
							@update:model-value="(value) => toggleNavShortcut(action, value)"
						/>
					</div>
				</template>
			</SettingsRow>
		</SettingsSection>

		<KeybindingModal
			ref="keybindingModal"
			:action-label="editingLabel"
			:binding="editingBinding"
			:default-binding="editingDefault"
			:conflicts="editingConflicts"
			@save="saveShortcut"
		/>
	</div>
</template>

<style scoped>
.keybinding-trigger {
	cursor: pointer;
	border-radius: var(--radius-md);
	border: 1px solid transparent;
	background: transparent;
	padding: 0.25rem 0.375rem;
	transition:
		background-color 0.15s ease,
		border-color 0.15s ease;
}

.keybinding-trigger:hover,
.keybinding-trigger:focus-visible {
	border-color: var(--surface-4);
	background: var(--surface-3);
}
</style>
