/**
 * Compatibility forwarders for the navigation-return Pinia store.
 * Prefer `@/store/navigation-return` in new code.
 */
export {
	clearUpgradeFlow,
	cloneUpgradeFlowSnapshot,
	consumeUpgradeFlow,
	parkUpgradeFlow,
	peekUpgradeFlow,
	restoreUpgradeFlow,
	upgradeProjectPath,
} from '../store/navigation-return.ts'
