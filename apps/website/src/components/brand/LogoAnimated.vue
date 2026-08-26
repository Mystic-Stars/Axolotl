<template>
	<div class="flex items-center justify-center h-20 mt-4">
		<img src="/axolotl.png" alt="" />
	</div>
</template>

<style lang="scss" scoped>
img {
	width: 5rem;
	height: 5rem;
	object-fit: contain;
	filter: drop-shadow(0 0.75rem 1.5rem rgb(245 139 174 / 28%));
	animation: float 3.5s ease-in-out infinite;
}

@keyframes float {
	0%,
	100% {
		transform: translateY(0) rotate(-2deg);
	}
	50% {
		transform: translateY(-0.4rem) rotate(2deg);
	}
}
</style>
