<script setup lang="ts">
import { LinkIcon } from '@modrinth/assets'
import { defineMessages, useVIntl } from '@modrinth/ui'
import { Admonition } from '@modrinth/ui'
import { computed } from 'vue'

import { injectSymlinkWarningDismiss } from '@/composables/useSymlinkWarningDismiss'

const props = withDefaults(
	defineProps<{
		symlinkTarget: string
		variant?: 'write' | 'delete'
		dismissible?: boolean
		badgeOnly?: boolean
	}>(),
	{
		variant: 'write',
		dismissible: false,
		badgeOnly: false,
	},
)

const emit = defineEmits<{
	dismiss: []
	dismissPermanently: []
}>()

const { formatMessage } = useVIntl()

const dismissState = injectSymlinkWarningDismiss()

const effectivelyHidden = computed(() => {
	if (props.variant === 'delete') return false
	return dismissState?.isHidden.value ?? false
})

const showBadge = computed(() => props.badgeOnly || effectivelyHidden.value)

const effectivelyDismissible = computed(() => {
	if (props.variant === 'delete') return false
	return props.dismissible || !!dismissState
})

function handleDismiss() {
	if (dismissState) {
		dismissState.dismissTemp()
	}
	emit('dismiss')
}

function handleDismissPermanently() {
	if (dismissState) {
		dismissState.dismissPermanently()
	}
	emit('dismissPermanently')
}

const messages = defineMessages({
	writeHeader: {
		id: 'app.symlink-warning.write.header',
	},
	writeBody: {
		id: 'app.symlink-warning.write.body',
	},
	deleteHeader: {
		id: 'app.symlink-warning.delete.header',
	},
	deleteBody: {
		id: 'app.symlink-warning.delete.body',
	},
	dismissPermanently: {
		id: 'app.symlink-warning.dismiss-permanently',
		defaultMessage: "Don't show again for this instance",
	},
	sharedBadge: {
		id: 'app.symlink-warning.shared-badge',
		defaultMessage: 'Shared',
	},
})
</script>

<template>
	<!-- Collapsed: small badge -->
	<div v-if="showBadge" class="inline-flex">
		<span
			v-tooltip="formatMessage(messages.writeBody, { path: props.symlinkTarget })"
			class="inline-flex items-center gap-1 rounded-full bg-bg-orange px-2 py-0.5 text-xs font-medium text-brand-orange"
		>
			<LinkIcon class="size-3" />
			{{ formatMessage(messages.sharedBadge) }}
		</span>
	</div>
	<!-- Expanded: full warning -->
	<template v-else>
		<Admonition
			v-if="props.variant === 'delete'"
			type="warning"
			:header="formatMessage(messages.deleteHeader)"
		>
			{{ formatMessage(messages.deleteBody, { path: props.symlinkTarget }) }}
		</Admonition>
		<Admonition
			v-else
			type="warning"
			:header="formatMessage(messages.writeHeader)"
			:dismissible="effectivelyDismissible"
			@dismiss="handleDismiss"
		>
			{{ formatMessage(messages.writeBody, { path: props.symlinkTarget }) }}
			<template v-if="effectivelyDismissible" #actions>
				<div class="flex justify-end">
					<button
						type="button"
						class="text-xs font-medium text-secondary hover:text-contrast transition-colors"
						@click="handleDismissPermanently"
					>
						{{ formatMessage(messages.dismissPermanently) }}
					</button>
				</div>
			</template>
		</Admonition>
	</template>
</template>
