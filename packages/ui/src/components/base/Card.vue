<script setup lang="ts">
import { DropdownIcon } from '@modrinth/assets'
import { reactive } from 'vue'

import ButtonStyled from './ButtonStyled.vue'

const props = defineProps({
	collapsible: {
		type: Boolean,
		default: false,
	},
	defaultCollapsed: {
		type: Boolean,
		default: false,
	},
	noAutoBody: {
		type: Boolean,
		default: false,
	},
})

const state = reactive({
	collapsed: props.defaultCollapsed,
})

function toggleCollapsed() {
	state.collapsed = !state.collapsed
}
</script>

<template>
	<div class="card">
		<div v-if="!!$slots.header || collapsible" class="header flex">
			<slot name="header"></slot>
			<div v-if="collapsible" class="btn-group ml-auto">
				<ButtonStyled circular>
					<button @click="toggleCollapsed">
						<DropdownIcon :style="{ transform: `rotate(${state.collapsed ? 0 : 180}deg)` }" />
					</button>
				</ButtonStyled>
			</div>
		</div>
		<slot v-if="!state.collapsed" />
	</div>
</template>

<style lang="scss" scoped>
.header {
	:deep(h1, h2, h3, h4) {
		margin-block: 0;
	}

	&:not(:last-child) {
		margin-bottom: var(--gap-lg);
	}
}
</style>
