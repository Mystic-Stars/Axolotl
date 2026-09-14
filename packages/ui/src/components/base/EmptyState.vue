<template>
	<div
		class="empty-state mx-auto flex w-full max-w-lg flex-col items-center justify-center gap-[var(--gap-md)] px-[var(--gap-xl)] py-[var(--gap-xl)] text-center"
		:class="rootClass"
		role="status"
	>
		<div
			v-if="displayIcon"
			class="empty-state__icon grid size-12 place-content-center rounded-[var(--radius-lg)] border border-solid border-divider bg-surface-2 text-secondary"
			:class="iconClass"
			aria-hidden="true"
		>
			<component :is="displayIcon" class="size-6" />
		</div>

		<component
			:is="illustration"
			v-else-if="illustration"
			class="empty-state__illustration h-[min(200px,36vh)] w-auto"
		/>

		<div class="flex flex-col items-center gap-[var(--gap-sm)]">
			<h2 class="empty-state__title m-0 text-center text-2xl font-semibold text-contrast">
				<slot name="heading">{{ heading }}</slot>
			</h2>
			<p
				v-if="$slots.description || description"
				class="empty-state__description m-0 max-w-prose text-center text-sm leading-6 text-secondary"
			>
				<slot name="description">{{ description }}</slot>
			</p>
		</div>

		<div
			v-if="$slots.actions"
			class="empty-state__actions mt-[var(--gap-sm)] flex flex-wrap items-center justify-center gap-[var(--gap-sm)]"
		>
			<slot name="actions" />
		</div>
	</div>
</template>

<script setup lang="ts">
import {
	CircleAlertIcon,
	DoneIllustration,
	EmptyIllustration,
	EmptyInboxIllustration,
	ErrorIllustration,
	FolderSearchIcon,
	NoConnectionIllustration,
	NoCreditCardIllustration,
	NoDocumentsIllustration,
	NoGPSIllustration,
	NoImagesIllustration,
	NoItemsCartIllustration,
	NoMessagesIllustration,
	NoSearchResultIllustration,
	NoTasksIllustration,
	PackageOpenIcon,
	TagCategoryWifiOffIcon,
	TriangleAlertIcon,
} from '@modrinth/assets'
import type { Component } from 'vue'
import { computed } from 'vue'

const illustrationMap: Record<string, Component> = {
	done: DoneIllustration,
	empty: EmptyIllustration,
	'empty-inbox': EmptyInboxIllustration,
	error: ErrorIllustration,
	'no-connection': NoConnectionIllustration,
	'no-credit-card': NoCreditCardIllustration,
	'no-documents': NoDocumentsIllustration,
	'no-gps': NoGPSIllustration,
	'no-images': NoImagesIllustration,
	'no-items-cart': NoItemsCartIllustration,
	'no-messages': NoMessagesIllustration,
	'no-search-result': NoSearchResultIllustration,
	'no-tasks': NoTasksIllustration,
}

/** Compact glyph shown in a tokenized chip above the title. */
const iconMap: Record<string, Component> = {
	empty: PackageOpenIcon,
	'empty-inbox': PackageOpenIcon,
	error: TriangleAlertIcon,
	warning: CircleAlertIcon,
	offline: TagCategoryWifiOffIcon,
	'no-connection': TagCategoryWifiOffIcon,
	'no-search-result': FolderSearchIcon,
	'no-results': FolderSearchIcon,
}

const props = withDefaults(
	defineProps<{
		/** Named empty/error state. Drives default icon when `icon` is omitted. */
		type?: keyof typeof illustrationMap | keyof typeof iconMap
		/** Explicit icon component (wins over type). */
		icon?: Component
		/** Prefer the compact icon chip over the large illustration. Default true when no custom illustration size needed. */
		useIcon?: boolean
		heading?: string
		description?: string
		/** Soften vertical footprint for dense pages. */
		compact?: boolean
	}>(),
	{
		useIcon: true,
		compact: false,
	},
)

const illustration = computed(() =>
	props.type && props.type in illustrationMap ? illustrationMap[props.type] : undefined,
)

const typeIcon = computed(() =>
	props.type && props.type in iconMap ? iconMap[props.type] : undefined,
)

const displayIcon = computed(() => {
	if (props.icon) return props.icon
	if (props.useIcon === false) return undefined
	// Fall back to illustration-only when useIcon is false or nothing maps.
	return typeIcon.value
})

const iconClass = computed(() => {
	const type = props.type
	if (type === 'error' || type === 'warning') return 'text-brand-red'
	if (type === 'offline' || type === 'no-connection') return 'text-secondary'
	return ''
})

const rootClass = computed(() => ({
	'empty-state--compact': props.compact,
	'empty-state--illustration': !displayIcon.value && !!illustration.value,
}))
</script>

<style scoped>
.empty-state--compact {
	gap: var(--gap-sm);
	padding-block: var(--gap-lg);
}

.empty-state--compact .empty-state__icon {
	/* size utility cannot be used in plain CSS without @apply; keep a tokenized box */
	width: 2.5rem;
	height: 2.5rem;
	border-radius: var(--radius-md);
}

.empty-state--compact .empty-state__title {
	font-size: 1.125rem;
}
</style>
