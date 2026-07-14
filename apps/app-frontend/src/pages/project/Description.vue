<template>
	<Card>
		<TranslatedProjectDescription
			:description="project.body"
			:active="translationActive"
			:translations="translations"
			:mode="translationMode"
			:style="translationStyle"
		/>
	</Card>
</template>

<script setup>
import { Card } from '@modrinth/ui'

import TranslatedProjectDescription from '@/components/ui/TranslatedProjectDescription.vue'

defineProps({
	project: {
		type: Object,
		default: () => {},
	},
	translationActive: {
		type: Boolean,
		default: false,
	},
	translations: {
		type: Object,
		default: () => ({}),
	},
	translationMode: {
		type: String,
		default: 'bilingual',
	},
	translationStyle: {
		type: String,
		default: 'weakened',
	},
})
</script>

<script>
export default {
	name: 'Description',
}
</script>
