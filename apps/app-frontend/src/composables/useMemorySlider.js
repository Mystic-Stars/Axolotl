import { useQuery } from '@tanstack/vue-query'
import { computed } from 'vue'

import { get_max_memory } from '@/helpers/jre.js'

export default function () {
	const memoryQuery = useQuery({
		queryKey: ['jre', 'max-memory'],
		queryFn: get_max_memory,
		staleTime: Infinity,
	})
	const maxMemory = computed(() => Math.floor(Number(memoryQuery.data.value ?? 0) / 1024))

	const snapPoints = computed(() => {
		const points = []
		let memory = 2048

		while (memory <= maxMemory.value) {
			points.push(memory)
			memory *= 2
		}

		return points
	})

	return { maxMemory, snapPoints, memoryQuery }
}
