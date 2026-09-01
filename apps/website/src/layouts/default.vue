<script setup lang="ts">
import AxolotlFooter from '~/components/ui/AxolotlFooter.vue'
import AxolotlHeader from '~/components/ui/AxolotlHeader.vue'
import SiteSettingsModal from '~/components/ui/SiteSettingsModal.vue'

const settingsOpen = ref(false)
</script>

<template>
	<div class="site-layout">
		<AxolotlHeader @open-settings="settingsOpen = true" />
		<main id="main">
			<slot />
		</main>
		<AxolotlFooter />
		<SiteSettingsModal v-model="settingsOpen" />
	</div>
</template>

<style lang="scss">
@import '~/assets/styles/global.scss';

html.accent-pink.light-mode {
	--surface-1: #f8f4f7;
	--surface-2: #fcf8fb;
	--surface-3: #fffafd;
	--surface-4: #ffffff;
	--surface-5: #eadde5;
	--color-text-primary: #2e2029;
	--color-text-default: #493944;
	--color-text-tertiary: #75616e;
	--color-pink: #c72f6c;
	--color-pink-highlight: rgb(199 47 108 / 14%);
	--color-brand-shadow: rgb(199 47 108 / 30%);
	--landing-color-heading: #281b24;
	--landing-color-subheading: #65515e;
	--landing-transition-gradient-end: #f8f4f7;
	--landing-border-color: rgb(166 107 138 / 32%);
}

html.accent-pink.dark-mode {
	--surface-1: #101018;
	--surface-2: #15151f;
	--surface-3: #1b1b27;
	--surface-4: #242432;
	--surface-5: #343446;
	--color-text-primary: #fff7fb;
	--color-text-default: #ddd3da;
	--color-text-tertiary: #aa9ca6;
	--color-pink: #ff82b2;
	--color-pink-highlight: rgb(255 130 178 / 18%);
	--color-brand-shadow: rgb(255 130 178 / 30%);
	--landing-color-heading: #fff8fc;
	--landing-color-subheading: #bdb0ba;
	--landing-transition-gradient-end: #101018;
	--landing-border-color: rgb(230 170 204 / 24%);
}

html.accent-pink.oled-mode {
	--color-pink: #ff82b2;
	--color-pink-highlight: rgb(255 130 178 / 18%);
	--color-brand-shadow: rgb(255 130 178 / 30%);
}

html {
	scroll-behavior: smooth;
	scroll-padding-top: 5rem;
}

.site-layout {
	// svh 以小视口（工具栏展开时）为基准，避免移动端 Firefox/Chrome 动态工具栏导致的高度偏差
	min-height: 100vh;
	min-height: 100svh;
	background: linear-gradient(rgb(255 255 255 / 1%) 1px, transparent 1px), var(--color-bg);
	background-size: 100% 8rem;
}

.site-layout > main {
	min-height: calc(100vh - 4.5rem);
	min-height: calc(100svh - 4.5rem);
}

.reduced-effects * {
	backdrop-filter: none !important;
}

.reduced-motion,
.reduced-motion * {
	scroll-behavior: auto !important;
	animation-duration: 0.001ms !important;
	animation-iteration-count: 1 !important;
	transition-duration: 0.001ms !important;
}

@media (prefers-reduced-motion: reduce) {
	*,
	*::before,
	*::after {
		scroll-behavior: auto !important;
		animation-duration: 0.001ms !important;
		animation-iteration-count: 1 !important;
		transition-duration: 0.001ms !important;
	}
}
</style>
