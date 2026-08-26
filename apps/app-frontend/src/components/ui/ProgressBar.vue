<template>
	<div class="w-full h-2 bg-button-bg rounded-[var(--radius-lg)] overflow-hidden">
		<div
			class="progress-bar__fill h-full"
			:style="{
				width: `${progress}%`,
				'background-color': error ? 'var(--color-red)' : 'var(--color-brand)',
			}"
		></div>
	</div>
</template>

<script setup>
defineProps({
	progress: {
		type: Number,
		required: true,
		validator(value) {
			return value >= 0 && value <= 100
		},
	},
	error: {
		type: Boolean,
		default: false,
	},
})
</script>

<style scoped>
.progress-bar__fill {
	transition: width 0.3s ease-out;
}
</style>
