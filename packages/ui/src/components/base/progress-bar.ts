/**
 * Converts progress in any natural unit to a safe 0..1 ratio.
 * Invalid progress and non-positive or invalid maxima represent no determinate progress.
 */
export function progressRatio(progress: number, max: number): number {
    if (!Number.isFinite(progress) || !Number.isFinite(max) || max <= 0) return 0

    return Math.min(Math.max(progress / max, 0), 1)
}
