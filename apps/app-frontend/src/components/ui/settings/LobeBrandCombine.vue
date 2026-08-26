<script setup lang="ts">
import { type Component, computed } from 'vue'

import { lobeAvatarBrands, lobeCombineBrands } from '@/data/lobeProviderIcons'

import HigressTextColor from './HigressTextColor.vue'

const props = withDefaults(
	defineProps<{
		brand: string
		extra?: string
		extraFontSize?: number
		extraMarginLeft?: number
		size: number
	}>(),
	{ extra: '', extraFontSize: undefined, extraMarginLeft: undefined },
)

const iconModules = import.meta.glob(
	'../../../../node_modules/@lobehub/icons-static-svg/icons/*.svg',
	{
		eager: true,
		import: 'default',
		query: '?component',
	},
) as Record<string, Component>

const iconComponents = Object.fromEntries(
	Object.entries(iconModules).map(([path, component]) => [
		path.split('/').pop()?.replace('.svg', ''),
		component,
	]),
) as Record<string, Component>

const config = computed(() => lobeCombineBrands[props.brand])
const brandAvatar = computed(() => lobeAvatarBrands[props.brand])
const standaloneComponent = computed(() =>
	config.value?.standalone ? iconComponents[config.value.standalone] : undefined,
)
const logoComponent = computed(() =>
	config.value?.logo ? iconComponents[config.value.logo] : undefined,
)
const textComponent = computed(() =>
	config.value?.text ? iconComponents[config.value.text] : undefined,
)
const avatarComponent = computed(() => {
	if (!config.value?.avatar || !brandAvatar.value) return undefined
	const suffix = brandAvatar.value.asset === 'color' ? '-color' : ''
	return iconComponents[`${props.brand}${suffix}`] ?? iconComponents[props.brand]
})
const standaloneSize = computed(() => props.size * (config.value?.textMultiple ?? 1))
const textSize = computed(() => props.size * (config.value?.textMultiple ?? 1))
const logoMargin = computed(() => props.size * (config.value?.spaceMultiple ?? 1))
const extraStyle = computed(() => ({
	fontSize: `${props.extraFontSize ?? textSize.value * 0.95}px`,
	marginLeft: props.extraMarginLeft === undefined ? undefined : `${props.extraMarginLeft}px`,
}))
</script>

<template>
	<span
		v-if="config"
		class="lobe-brand-combine inline-flex min-w-0 flex-none items-center justify-start"
		:class="{ inverse: config.inverse }"
		:style="{ color: config.color }"
	>
		<HigressTextColor
			v-if="brand === 'higress'"
			class="lobe-brand-standalone"
			:style="{ height: `${standaloneSize}px` }"
		/>
		<component
			:is="standaloneComponent"
			v-else-if="standaloneComponent"
			class="lobe-brand-standalone"
			:style="{ height: `${standaloneSize}px` }"
		/>
		<template v-else>
			<span
				v-if="avatarComponent && brandAvatar"
				class="lobe-brand-avatar inline-flex flex-none items-center justify-center overflow-hidden"
				:style="{
					background: brandAvatar.background,
					borderRadius: `${Math.floor(size * 0.1)}px`,
					color: brandAvatar.color,
					height: `${size}px`,
					marginLeft: config.inverse ? `${logoMargin}px` : undefined,
					marginRight: config.inverse ? undefined : `${logoMargin}px`,
					width: `${size}px`,
				}"
			>
				<component
					:is="avatarComponent"
					:style="{
						height: `${size}px`,
						transform: `scale(${brandAvatar.multiple})`,
						width: `${size}px`,
					}"
				/>
			</span>
			<component
				:is="logoComponent"
				v-else-if="logoComponent"
				class="lobe-brand-logo object-contain"
				:style="{
					height: `${size}px`,
					marginLeft: config.inverse ? `${logoMargin}px` : undefined,
					marginRight: config.inverse ? undefined : `${logoMargin}px`,
					width: `${size}px`,
				}"
			/>
			<component
				:is="textComponent"
				v-if="textComponent"
				class="lobe-brand-text"
				:style="{ height: `${textSize}px` }"
			/>
		</template>
		<span v-if="extra" class="flex-none leading-none" :style="extraStyle">{{ extra }}</span>
	</span>
</template>

<style scoped>
.lobe-brand-combine.inverse {
	flex-direction: row-reverse;
}

.lobe-brand-avatar > :deep(svg),
.lobe-brand-logo,
.lobe-brand-text,
.lobe-brand-standalone {
	display: block;
	flex: none;
	width: auto;
}
</style>
