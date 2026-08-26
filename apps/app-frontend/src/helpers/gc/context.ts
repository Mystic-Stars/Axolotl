import type { InstanceLoader } from '@/helpers/types'

import type { GcContext } from './types'

export async function collectGcContext(
	allocatedMemoryMb: number,
	loader: InstanceLoader | null,
	javaMajorVersion?: number | null,
	modCount?: number,
): Promise<GcContext> {
	const systemCpuCores = navigator.hardwareConcurrency ?? 4
	const systemLogicalProcessors = systemCpuCores

	return {
		javaMajorVersion: javaMajorVersion ?? null,
		allocatedMemoryMb,
		systemCpuCores,
		systemLogicalProcessors,
		modCount: modCount ?? 0,
		loader: loader ?? 'vanilla',
	}
}

export function extractJavaMajorVersion(
	parsedVersion: string | number | null | undefined,
): number | null {
	if (parsedVersion === null || parsedVersion === undefined) return null

	// 如果是数字，直接返回
	if (typeof parsedVersion === 'number') {
		return Number.isNaN(parsedVersion) ? null : parsedVersion
	}

	// 如果是字符串，尝试解析
	if (typeof parsedVersion === 'string') {
		const match = parsedVersion.match(/^(?:1\.)?(\d+)/)
		if (!match) return null
		const num = parseInt(match[1], 10)
		return Number.isNaN(num) ? null : num
	}

	return null
}
