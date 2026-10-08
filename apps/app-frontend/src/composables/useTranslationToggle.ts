import { ref } from 'vue'

/**
 * Manages translation toggle state (active/loading) with version-based
 * cancellation to handle async race conditions.
 *
 * Usage:
 * ```
 * const ctrl = useTranslationToggle()
 *
 * async function doTranslate() {
 *   const version = ctrl.start()
 *   try {
 *     const result = await api()
 *     if (ctrl.isStale(version)) return   // superseded, discard
 *     // apply result
 *     ctrl.translationActive.value = true
 *   } finally {
 *     ctrl.done(version)
 *   }
 * }
 *
 * function toggleTranslation() {
 *   ctrl.toggle(
 *     () => { restoreOriginal() },
 *     () => void doTranslate(),
 *   )
 * }
 *
 * // When new data arrives (e.g. new search results):
 * ctrl.cancel()
 * ```
 */
export function useTranslationToggle() {
    const translationActive = ref(false)
    const translationLoading = ref(false)
    let _version = 0

    /** Bump version, set loading, and return the new version number for staleness checks. */
    function start(): number {
        translationLoading.value = true
        return ++_version
    }

    /** Returns true if `version` is no longer the current request. */
    function isStale(version: number): boolean {
        return version !== _version
    }

    /** Stop loading only if `version` is still current. */
    function done(version: number): void {
        if (version === _version) translationLoading.value = false
    }

    /**
     * Toggle between original and translated content.
     *
     * @param showOriginal - callback that restores the original untranslated data
     * @param doTranslate  - callback that kicks off a new translation
     */
    function toggle(showOriginal: () => void, doTranslate: () => void): void {
        if (translationActive.value) {
            _version++ // cancel any in-flight translation
            showOriginal()
            translationActive.value = false
            translationLoading.value = false
            return
        }
        doTranslate()
    }

    /** Cancel any in-flight translation and reset to inactive state. Returns the current version. */
    function cancel(): number {
        _version++
        translationActive.value = false
        translationLoading.value = false
        return _version
    }

    return {
        translationActive,
        translationLoading,
        start,
        isStale,
        done,
        toggle,
        cancel,
    }
}
