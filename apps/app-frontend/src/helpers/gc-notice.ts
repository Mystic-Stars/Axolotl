import { ref } from 'vue'

import type { GcLaunchReport } from '@/helpers/instance'

/**
 * The GC launch report from the most recent launch, so settings pages can show
 * what strategy the JVM actually accepted (and any fallback that happened).
 */
export const lastGcLaunchReport = ref<GcLaunchReport | null>(null)

export function setLastGcLaunchReport(report: GcLaunchReport | null) {
	lastGcLaunchReport.value = report
}
