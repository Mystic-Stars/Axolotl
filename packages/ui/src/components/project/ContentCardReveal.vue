<script setup lang="ts">
import { onMounted, ref } from 'vue'

const revealed = ref(false)

onMounted(() => {
	// Next frame so the enter transition has a starting style to animate from.
	requestAnimationFrame(() => {
		revealed.value = true
	})
})
</script>

<template>
	<div class="card-reveal" :class="{ 'card-reveal--visible': revealed }">
		<slot />
	</div>
</template>

<style scoped>
.card-reveal {
	opacity: 0;
	transform: translateY(4px);
	transition:
		opacity 220ms ease,
		transform 220ms ease;
}

.card-reveal--visible {
	opacity: 1;
	transform: translateY(0);
}
</style>
