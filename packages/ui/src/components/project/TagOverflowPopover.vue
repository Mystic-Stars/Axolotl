<script setup lang="ts">
import '../../styles/overlays.css'

import { PopoverArrow, PopoverContent, PopoverPortal, PopoverRoot, PopoverTrigger } from 'reka-ui'
import { computed } from 'vue'

const props = defineProps<{
	count: number
	wrapperClass?: string
}>()

defineOptions({
	inheritAttrs: false,
})

const portalTarget = computed(() =>
	typeof document !== 'undefined' && document.getElementById('teleports') ? '#teleports' : 'body',
)
</script>

<template>
	<PopoverRoot :modal="false">
		<PopoverTrigger as-child>
			<div
				v-bind="$attrs"
				class="inline-flex focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-brand"
				:class="props.wrapperClass"
			>
				<slot name="trigger">+{{ props.count }}</slot>
			</div>
		</PopoverTrigger>

		<PopoverPortal :to="portalTarget">
			<PopoverContent class="menu-surface" :side-offset="4">
				<div class="flex max-w-[20rem] flex-wrap gap-1">
					<slot />
				</div>
				<PopoverArrow class="menu-arrow" :width="14" :height="7" />
			</PopoverContent>
		</PopoverPortal>
	</PopoverRoot>
</template>
