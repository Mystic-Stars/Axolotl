/**
 * Compatibility forwarders for the navigation-return Pinia store.
 * Prefer `@/store/navigation-return` in new code.
 */
export type { BrowseReturnSnapshot } from '../store/navigation-return.ts'
export {
	clearBrowseReturnSnapshot,
	completeBrowseReturnNavigation,
	consumeBrowseReturnSnapshot,
	hasBrowseReturnSnapshot,
	isBrowseReturnNavigation,
	isBrowseReturnSourcePath,
	prepareBrowseReturnNavigation,
	saveBrowseReturnSnapshot,
} from '../store/navigation-return.ts'
