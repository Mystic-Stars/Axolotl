<script setup lang="ts">
import { NbtType } from 'deepslate/nbt'

defineProps<{
	type: NbtType
}>()

const colors: Record<NbtType, string> = {
	[NbtType.End]: 'text-secondary',
	[NbtType.Byte]: 'text-orange',
	[NbtType.Short]: 'text-yellow',
	[NbtType.Int]: 'text-green',
	[NbtType.Long]: 'text-blue',
	[NbtType.Float]: 'text-purple',
	[NbtType.Double]: 'text-pink',
	[NbtType.ByteArray]: 'text-orange',
	[NbtType.String]: 'text-brand',
	[NbtType.List]: 'text-cyan',
	[NbtType.Compound]: 'text-contrast',
	[NbtType.IntArray]: 'text-green',
	[NbtType.LongArray]: 'text-blue',
}

const labels: Partial<Record<NbtType, string>> = {
	[NbtType.Byte]: 'B',
	[NbtType.Short]: 'S',
	[NbtType.Int]: 'I',
	[NbtType.Long]: 'L',
	[NbtType.Float]: 'F',
	[NbtType.Double]: 'D',
}
</script>

<template>
	<svg
		viewBox="0 0 16 16"
		aria-hidden="true"
		class="size-4 shrink-0"
		:class="colors[type]"
		fill="none"
		stroke="currentColor"
		stroke-linecap="round"
		stroke-linejoin="round"
		stroke-width="1.5"
	>
		<path v-if="type === NbtType.Compound" d="M3 2.5h7l3 3v8H3zM10 2.5v3h3" />
		<path v-else-if="type === NbtType.List" d="M3 3h10M3 8h10M3 13h10M5 3v10" />
		<path
			v-else-if="type === NbtType.String"
			d="M4 3h8M4 13h8M5 3c-3 2-3 8 0 10M11 3c3 2 3 8 0 10"
		/>
		<path
			v-else-if="
				type === NbtType.ByteArray || type === NbtType.IntArray || type === NbtType.LongArray
			"
			d="M4 2.5h8v11H4zM6.5 5h3M6.5 8h3M6.5 11h3"
		/>
		<path v-else d="M3 3h10v10H3z" />
		<text
			v-if="labels[type]"
			x="8"
			y="11"
			fill="currentColor"
			stroke="none"
			text-anchor="middle"
			font-size="7"
			font-weight="700"
		>
			{{ labels[type] }}
		</text>
	</svg>
</template>
