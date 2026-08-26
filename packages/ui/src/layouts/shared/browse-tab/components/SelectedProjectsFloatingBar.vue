<template>
	<FloatingActionBar
		:shown="shown"
		:aria-label="formatMessage(messages.ariaLabel)"
		allow-overflow
		hide-when-modal-open
	>
		<div class="flex min-w-0 items-center gap-0.5">
			<div
				v-if="selectedCount > 0"
				class="relative h-8 shrink-0"
				@mouseenter="openProjectPreview"
				@mouseleave="scheduleProjectPreviewClose"
			>
				<button
					type="button"
					class="project-stack-trigger relative h-8 cursor-pointer rounded-lg p-0 focus-visible:outline-none focus-visible:ring-4 focus-visible:ring-brand-shadow"
					:style="{ width: `${iconStackWidth}px` }"
					:aria-controls="projectPreviewId"
					:aria-expanded="projectPreviewOpen"
					:aria-label="
						formatMessage(projectPreviewOpen ? messages.hideProjects : messages.showProjects, {
							count: selectedCount,
						})
					"
					@focus="openProjectPreview"
					@blur="scheduleProjectPreviewClose"
					@click="openProjectPreview"
					@keydown.esc.prevent.stop="closeProjectPreview"
				>
					<span aria-hidden="true">
						<span
							v-for="(project, index) in visibleProjects"
							:key="project.id"
							v-tooltip="project.name"
							class="absolute top-0 flex h-8 w-8 items-center justify-center overflow-hidden rounded-lg border-[1.5px] border-solid border-surface-3 bg-surface-4"
							:style="{
								left: `${index * iconStackOffset}px`,
								zIndex: visibleProjects.length - index,
							}"
						>
							<Avatar
								:src="project.iconUrl"
								:alt="project.name"
								:tint-by="project.id"
								size="100%"
								no-shadow
								class="selected-project-avatar"
							/>
						</span>
						<span
							v-if="overflowCount > 0"
							class="absolute top-0 flex h-8 w-8 items-center justify-center rounded-lg border-[1.5px] border-solid border-surface-3 bg-surface-4 text-xs font-bold text-contrast"
							:style="{ left: `${visibleProjects.length * iconStackOffset}px`, zIndex: 0 }"
						>
							+{{ overflowCount }}
						</span>
					</span>
				</button>

				<Transition name="selected-project-preview">
					<div
						v-if="projectPreviewOpen && selectedCount > 1"
						:id="projectPreviewId"
						class="selected-project-preview absolute bottom-[calc(100%+0.75rem)] left-0 z-20 flex w-[min(18rem,calc(100vw-4rem))] flex-col gap-1 overflow-x-hidden overflow-y-auto rounded-lg border border-solid border-surface-5 bg-surface-2 p-2 shadow-[0px_6px_10px_0px_rgba(0,0,0,0.15),0px_16px_24px_0px_rgba(0,0,0,0.2)]"
						role="list"
						@mouseenter="openProjectPreview"
						@mouseleave="scheduleProjectPreviewClose"
					>
						<div
							v-for="project in selectedProjects"
							:key="project.id"
							class="selected-project-preview-item flex min-w-0 items-center gap-2 rounded-md bg-surface-3 px-2 py-1.5"
							role="listitem"
						>
							<Avatar
								:src="project.iconUrl"
								:alt="project.name"
								:tint-by="project.id"
								size="2rem"
								no-shadow
							/>
							<span class="min-w-0 truncate text-sm font-semibold text-contrast">
								{{ project.name }}
							</span>
						</div>
					</div>
				</Transition>
			</div>

			<span class="px-3 py-2 text-base font-semibold text-contrast tabular-nums">
				{{ selectedCountText }}
			</span>
			<div class="mx-0.5 h-6 w-px bg-surface-5" />
			<ButtonStyled type="transparent">
				<button
					type="button"
					class="!text-primary"
					:disabled="isInstallingSelected"
					@click="clearSelected"
				>
					<span>{{ formatMessage(commonMessages.clearButton) }}</span>
				</button>
			</ButtonStyled>
		</div>

		<div class="ml-auto shrink-0">
			<ButtonStyled color="brand">
				<button type="button" :disabled="isInstallingSelected" @click="installSelected">
					<PlusIcon />
					{{ actionButtonText }}
				</button>
			</ButtonStyled>
		</div>
	</FloatingActionBar>
</template>

<script setup lang="ts">
import { PlusIcon } from '@modrinth/assets'
import { computed, onUnmounted, ref, useId } from 'vue'

import Avatar from '#ui/components/base/Avatar.vue'
import ButtonStyled from '#ui/components/base/ButtonStyled.vue'
import FloatingActionBar from '#ui/components/base/FloatingActionBar.vue'
import { defineMessages, useVIntl } from '#ui/composables/i18n'
import { commonMessages } from '#ui/utils/common-messages'

import { injectBrowseManager } from '../providers/browse-manager'
import type { BrowseInstallContext } from '../types'

const { formatMessage } = useVIntl()

const messages = defineMessages({
	ariaLabel: {
		id: 'browse.selected-projects-floating-bar.aria-label',
		defaultMessage: 'Selected projects',
	},
	selectedCount: {
		id: 'browse.selected-projects-floating-bar.selected-count',
		defaultMessage: '{count, plural, one {# project selected} other {# projects selected}}',
	},
	installButton: {
		id: 'browse.selected-projects-floating-bar.install',
		defaultMessage: 'Install {count, plural, one {# project} other {# projects}}',
	},
	showProjects: {
		id: 'browse.selected-projects-floating-bar.show-projects',
		defaultMessage: 'Show {count, plural, one {# selected project} other {# selected projects}}',
	},
	hideProjects: {
		id: 'browse.selected-projects-floating-bar.hide-projects',
		defaultMessage: 'Hide selected projects',
	},
})

const props = defineProps<{
	installContext?: BrowseInstallContext | null
}>()

const ctx = injectBrowseManager(null)
const installContext = computed(() => props.installContext ?? ctx?.installContext?.value ?? null)
const selectedProjects = computed(() => installContext.value?.selectedProjects ?? [])
const selectedCount = computed(() => selectedProjects.value.length)
const iconStackOffset = 24
const isInstallingSelected = computed(() => installContext.value?.isInstallingSelected ?? false)
const shown = computed(() => selectedCount.value > 0 || isInstallingSelected.value)
const projectPreviewId = `selected-project-preview-${useId()}`
const projectPreviewOpen = ref(false)
const visibleProjects = computed(() => selectedProjects.value.slice(0, 3))
const overflowCount = computed(() => Math.max(0, selectedCount.value - 3))
const iconStackWidth = computed(() => {
	if (selectedCount.value === 0) return 0
	return (
		32 + (visibleProjects.value.length - 1 + (overflowCount.value > 0 ? 1 : 0)) * iconStackOffset
	)
})
const selectedCountText = computed(() =>
	formatMessage(messages.selectedCount, { count: selectedCount.value }),
)
const installButtonText = computed(
	() =>
		installContext.value?.installButtonLabel ??
		formatMessage(messages.installButton, { count: selectedCount.value }),
)
const actionButtonText = computed(() =>
	isInstallingSelected.value
		? (installContext.value?.processingLabel ?? installButtonText.value)
		: installButtonText.value,
)

let projectPreviewCloseTimer: ReturnType<typeof setTimeout> | null = null

function openProjectPreview() {
	if (selectedCount.value < 2) return
	if (projectPreviewCloseTimer !== null) {
		clearTimeout(projectPreviewCloseTimer)
		projectPreviewCloseTimer = null
	}
	projectPreviewOpen.value = true
}

function closeProjectPreview() {
	if (projectPreviewCloseTimer !== null) {
		clearTimeout(projectPreviewCloseTimer)
		projectPreviewCloseTimer = null
	}
	projectPreviewOpen.value = false
}

function scheduleProjectPreviewClose() {
	if (projectPreviewCloseTimer !== null) clearTimeout(projectPreviewCloseTimer)
	projectPreviewCloseTimer = setTimeout(closeProjectPreview, 160)
}

function clearSelected() {
	if (isInstallingSelected.value) return
	void (installContext.value?.clearSelected ?? installContext.value?.clearQueued)?.()
}

function installSelected() {
	if (isInstallingSelected.value) return
	void installContext.value?.installSelected?.()
}

onUnmounted(closeProjectPreview)
</script>

<style scoped>
:deep(.selected-project-avatar) {
	background-color: var(--color-button-bg);
}

.selected-project-preview {
	max-height: min(18rem, calc(100dvh - 12rem));
	scrollbar-gutter: stable;
}

.selected-project-preview-enter-active,
.selected-project-preview-leave-active {
	transition:
		opacity 160ms ease,
		transform 180ms ease;
}

.selected-project-preview-enter-from,
.selected-project-preview-leave-to {
	opacity: 0;
	transform: translateY(0.5rem) scale(0.98);
}

@media (prefers-reduced-motion) {
	.selected-project-preview-enter-active,
	.selected-project-preview-leave-active {
		transition: none;
	}
}
</style>
