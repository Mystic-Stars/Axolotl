import assert from 'node:assert/strict'
import test from 'node:test'

import type {
	InstanceUpgradePlan,
	InstanceUpgradeTargetEnvironment,
} from '@/helpers/instance-upgrade'

import {
	actionableWarningContentIds,
	AUTOMATIC_FABRIC_LOADER_VERSION,
	automaticFabricLoaderTargetAvailable,
	availablePredefinedStrategies,
	captureInitialUpgradeBlockingIssues,
	commitUpgradePlanSelection,
	compatibilitySummary,
	confirmDependencyReleaseSlots,
	confirmSelectionReleaseSlots,
	confirmSolutionGroups,
	confirmTargetLoaderLabel,
	confirmUpgradeOptions,
	contentIdentityKeys,
	customConstraintsEqual,
	editableUpgradeRoots,
	fabricLoaderVersionForTarget,
	fabricUpgradeLoaderVersions,
	groupUpgradeIssues,
	inferShaderRuntime,
	isSharedUpgradeInstance,
	newerStableGameVersions,
	preserveFabricLoaderSelection,
	resolveConfirmDependencyReleases,
	resolveUpgradePlanSelection,
	sanitizeMinecraftDisplayTitle,
	setFixedConstraint,
	shouldReuseUpgradePlan,
	solutionSummary,
	upgradeContentDisplayMetadata,
	upgradeResolutionPresentation,
	upgradeTargetsEqual,
} from './analysis.ts'

function issue(code: string, contentId: string | null, projectId: string | null, message = code) {
	return {
		code,
		message,
		contentId,
		provider: projectId ? 'modrinth' : null,
		projectId,
		conflictingProjectId: null,
		dependencyRequirements: [],
	}
}

function planItem(contentId: string, projectId: string | null = contentId) {
	return {
		contentId,
		relativePath: `mods/${contentId}.jar`,
		projectType: 'mod',
		provider: projectId ? 'modrinth' : null,
		projectId,
		currentReleaseId: 'old',
		currentEnabled: true,
		autoDependency: false,
		status: 'already_compatible',
		resolution: {
			contentId,
			action: 'upgrade',
			allowPrerelease: false,
			confirmedPrereleaseDependencies: [],
		},
		candidateReleaseIds: [],
	}
}

test('newer stable versions follow metadata order without numeric parsing', () => {
	const result = newerStableGameVersions(
		[
			{ version: '26.1.2', version_type: 'release', date: '', major: false },
			{ version: '26.1-beta', version_type: 'snapshot', date: '', major: false },
			{ version: '26.1', version_type: 'release', date: '', major: true },
			{ version: '1.21.8', version_type: 'release', date: '', major: false },
			{ version: '1.21.7', version_type: 'release', date: '', major: false },
		],
		'1.21.8',
	)

	assert.deepEqual(result, { currentFound: true, versions: ['26.1.2', '26.1'] })
})

test('upgrade target equality uses semantic environment fields', () => {
	const target = {
		gameVersion: '26.2',
		modLoader: 'fabric',
		modLoaderVersion: '0.18.4',
		shaderRuntime: 'iris',
	} as const
	assert.equal(upgradeTargetsEqual(target, { ...target }), true)
	assert.equal(upgradeTargetsEqual(target, { ...target, gameVersion: '26.1' }), false)
	assert.equal(upgradeTargetsEqual(target, { ...target, modLoaderVersion: '1' }), false)
})

test('matching instance and semantic target reuse existing plan without replacing state', () => {
	const target = {
		gameVersion: '26.2',
		modLoader: 'fabric',
		modLoaderVersion: null,
		shaderRuntime: 'iris',
	} as const
	const plan = {
		id: 'plan-one',
		instanceId: 'instance-one',
		targetEnvironment: target,
		items: [{ resolution: { action: 'keep' } }],
		customConstraints: [{ contentId: 'root', versionId: 'fixed' }],
	} as InstanceUpgradePlan

	assert.equal(shouldReuseUpgradePlan('instance-one', plan, { ...target }), true)
	assert.equal(plan.id, 'plan-one')
	assert.equal(plan.items[0].resolution.action, 'keep')
	assert.equal(plan.customConstraints[0].versionId, 'fixed')
	assert.equal(
		shouldReuseUpgradePlan('instance-one', plan, { ...target, gameVersion: '26.1' }),
		false,
	)
	assert.equal(shouldReuseUpgradePlan('instance-two', plan, target), false)
	assert.equal(shouldReuseUpgradePlan('instance-one', null, target), false)
})

test('plan selection skips planner for matching target and calls it once for a confirmed change', async () => {
	const target: InstanceUpgradeTargetEnvironment = {
		gameVersion: '26.2',
		modLoader: 'fabric',
		modLoaderVersion: '0.18.4',
		shaderRuntime: 'iris',
	}
	const existing = {
		id: 'plan-one',
		instanceId: 'instance-one',
		targetEnvironment: target,
	} as InstanceUpgradePlan
	let calls = 0
	const planner = async (_instanceId: string, nextTarget: InstanceUpgradeTargetEnvironment) => {
		calls += 1
		return {
			...existing,
			id: 'plan-two',
			targetEnvironment: nextTarget,
		} as InstanceUpgradePlan
	}

	const reused = await resolveUpgradePlanSelection('instance-one', existing, { ...target }, planner)
	assert.equal(calls, 0)
	assert.equal(reused.plan, existing)
	assert.equal(reused.reused, true)

	const replanned = await resolveUpgradePlanSelection(
		'instance-one',
		existing,
		{ ...target, modLoaderVersion: '0.18.5' },
		planner,
	)
	assert.equal(calls, 1)
	assert.equal(replanned.plan.id, 'plan-two')
	assert.equal(replanned.reused, false)
})

test('failed replan preserves existing plan and authoritative target', async () => {
	const oldTarget = {
		gameVersion: '26.1',
		modLoader: 'fabric',
		modLoaderVersion: '0.18.4',
		shaderRuntime: 'iris',
	} as const
	const attemptedTarget = { ...oldTarget, gameVersion: '26.2', modLoaderVersion: '0.18.5' }
	const oldPlan = {
		id: 'old-plan',
		instanceId: 'instance-one',
		targetEnvironment: oldTarget,
	} as InstanceUpgradePlan
	let authoritativePlan = oldPlan
	let authoritativeTarget = oldTarget

	await assert.rejects(
		commitUpgradePlanSelection(
			'instance-one',
			oldPlan,
			attemptedTarget,
			async () => {
				throw new Error('planning failed')
			},
			(plan) => (authoritativePlan = plan),
			(target) => (authoritativeTarget = target as typeof oldTarget),
		),
		/planning failed/,
	)
	assert.equal(authoritativePlan, oldPlan)
	assert.equal(authoritativeTarget, oldTarget)
})

test('Fabric loader choices exclude downgrades using numeric semantic comparison', () => {
	assert.deepEqual(
		fabricUpgradeLoaderVersions('0.18.4', ['0.18.6', '0.18.5', '0.18.4', '0.18.3']),
		['0.18.6', '0.18.5', '0.18.4'],
	)
	assert.deepEqual(fabricUpgradeLoaderVersions('0.18.9', ['0.18.10', '0.18.9']), [
		'0.18.10',
		'0.18.9',
	])
	assert.deepEqual(fabricUpgradeLoaderVersions('custom', ['0.19.0']), [])
})

test('Fabric loader pending selection preserves valid exact values and maps target values', () => {
	assert.equal(preserveFabricLoaderSelection('0.18.5', ['0.18.5']), '0.18.5')
	assert.equal(preserveFabricLoaderSelection('0.18.5', ['0.18.6']), AUTOMATIC_FABRIC_LOADER_VERSION)
	assert.equal(
		preserveFabricLoaderSelection(AUTOMATIC_FABRIC_LOADER_VERSION, []),
		AUTOMATIC_FABRIC_LOADER_VERSION,
	)
	assert.equal(fabricLoaderVersionForTarget(AUTOMATIC_FABRIC_LOADER_VERSION), null)
	assert.equal(fabricLoaderVersionForTarget('0.18.5'), '0.18.5')
	assert.equal(automaticFabricLoaderTargetAvailable(true, true, []), false)
	assert.equal(automaticFabricLoaderTargetAvailable(true, true, ['0.18.5']), true)
	assert.equal(automaticFabricLoaderTargetAvailable(false, true, []), true)
	assert.equal(automaticFabricLoaderTargetAvailable(true, false, []), true)
})

test('shared upgrade detection uses established link or external target metadata', () => {
	assert.equal(isSharedUpgradeInstance({ link: { type: 'shared_instance' } } as never), true)
	assert.equal(isSharedUpgradeInstance({ symlink_target: 'D:/Minecraft' } as never), true)
	assert.equal(isSharedUpgradeInstance({ link: null, symlink_target: null } as never), false)
})

test('unknown current version exposes stable releases conservatively', () => {
	const result = newerStableGameVersions(
		[
			{ version: '26.1', version_type: 'release', date: '', major: true },
			{ version: '26.1-beta', version_type: 'snapshot', date: '', major: false },
		],
		'custom',
	)

	assert.deepEqual(result, { currentFound: false, versions: ['26.1'] })
})

test('compatibility summary uses selected solution and changed dependencies', () => {
	const plan = {
		blockingIssues: [{ code: 'dependency_conflict' }, { code: 'prerelease_only' }],
		selectedSolution: {
			selections: [
				{ action: 'upgrade', currentReleaseId: 'old', targetReleaseId: 'new' },
				{ action: 'keep', currentReleaseId: 'same', targetReleaseId: 'same' },
				{ action: 'disable', currentReleaseId: 'off', targetReleaseId: null },
			],
			dependencyChanges: [{ kind: 'add' }, { kind: 'keep' }, { kind: 'remove' }],
		},
	} as InstanceUpgradePlan

	assert.deepEqual(compatibilitySummary(plan), {
		updates: 1,
		keptOrCompatible: 1,
		disabled: 1,
		dependencyChanges: 2,
		needsAttention: 2,
	})
})

test('shader runtime inference uses exact loader component and provider identity', () => {
	const instance = {
		loader: 'fabric',
		loader_components: [],
	} as never
	const snapshot = {
		items: [
			{
				projectType: 'mod',
				provider: 'modrinth',
				providerProjectId: 'YL57xq9U',
				content: null,
			},
		],
	} as never

	assert.equal(inferShaderRuntime(instance, snapshot), 'iris')
	assert.equal(
		inferShaderRuntime({ ...instance, loader_components: [{ kind: 'optifine' }] }, undefined),
		'opti_fine',
	)
	assert.equal(inferShaderRuntime(instance, undefined), 'unknown')
})

test('solution summary separates root and dependency changes', () => {
	const summary = solutionSummary({
		kind: 'newest',
		selections: [
			{
				contentId: 'a',
				provider: 'modrinth',
				projectId: 'a',
				currentReleaseId: '1',
				targetReleaseId: '2',
				action: 'upgrade',
				enabled: true,
			},
			{
				contentId: 'b',
				provider: 'modrinth',
				projectId: 'b',
				currentReleaseId: '1',
				targetReleaseId: '1',
				action: 'keep',
				enabled: true,
			},
			{
				contentId: 'c',
				provider: 'modrinth',
				projectId: 'c',
				currentReleaseId: '1',
				targetReleaseId: null,
				action: 'disable',
				enabled: false,
			},
		],
		dependencyChanges: [
			{
				existingContentId: null,
				provider: 'modrinth',
				projectId: 'd',
				currentReleaseId: null,
				targetReleaseId: '1',
				kind: 'add',
				enabled: true,
			},
			{
				existingContentId: 'e',
				provider: 'modrinth',
				projectId: 'e',
				currentReleaseId: '1',
				targetReleaseId: '2',
				kind: 'upgrade',
				enabled: true,
			},
			{
				existingContentId: 'f',
				provider: 'modrinth',
				projectId: 'f',
				currentReleaseId: '1',
				targetReleaseId: null,
				kind: 'remove',
				enabled: false,
			},
		],
		warnings: [],
	})
	assert.deepEqual(summary, {
		upgraded: 1,
		kept: 1,
		disabled: 1,
		dependencyAdditions: 1,
		dependencyUpdates: 1,
		dependencyRemovals: 1,
	})
})

test('confirm detail groups follow authoritative selection actions', () => {
	const solution = {
		kind: 'custom',
		selections: [
			{ contentId: 'update', action: 'upgrade', currentReleaseId: '1', targetReleaseId: '2' },
			{ contentId: 'same', action: 'upgrade', currentReleaseId: '1', targetReleaseId: '1' },
			{ contentId: 'keep', action: 'keep', currentReleaseId: '1', targetReleaseId: '1' },
			{ contentId: 'disable', action: 'disable', currentReleaseId: '1', targetReleaseId: null },
		],
		dependencyChanges: [{ kind: 'add' }, { kind: 'upgrade' }, { kind: 'keep' }, { kind: 'remove' }],
		warnings: [],
	} as never
	const groups = confirmSolutionGroups(solution)

	assert.deepEqual(
		groups.updated.map((item) => item.contentId),
		['update'],
	)
	assert.deepEqual(
		groups.kept.map((item) => item.contentId),
		['same', 'keep'],
	)
	assert.deepEqual(
		groups.disabled.map((item) => item.contentId),
		['disable'],
	)
	assert.deepEqual(
		groups.dependencyChanges.map((item) => item.kind),
		['add', 'upgrade', 'remove'],
	)
})

test('confirm upgrade options require shared mode and suppress redundant copy backup', () => {
	assert.deepEqual(confirmUpgradeOptions(false, null, true), {
		effectiveMode: 'direct',
		createFullBackup: true,
		canStart: true,
	})
	assert.deepEqual(confirmUpgradeOptions(true, null, true), {
		effectiveMode: null,
		createFullBackup: true,
		canStart: false,
	})
	assert.deepEqual(confirmUpgradeOptions(true, 'direct', false), {
		effectiveMode: 'direct',
		createFullBackup: false,
		canStart: true,
	})
	assert.deepEqual(confirmUpgradeOptions(true, 'copy_and_upgrade', true), {
		effectiveMode: 'copy_and_upgrade',
		createFullBackup: false,
		canStart: true,
	})
})

test('confirm release slots keep current and target changelogs independent', () => {
	assert.deepEqual(
		confirmSelectionReleaseSlots({
			action: 'upgrade',
			currentReleaseId: 'current',
			targetReleaseId: 'target',
		} as never),
		{ currentReleaseId: 'current', targetReleaseId: 'target' },
	)
	assert.deepEqual(
		confirmSelectionReleaseSlots({
			action: 'keep',
			currentReleaseId: 'current',
			targetReleaseId: 'current',
		} as never),
		{ currentReleaseId: 'current', targetReleaseId: null },
	)
	assert.deepEqual(
		confirmDependencyReleaseSlots({
			currentReleaseId: 'current',
			targetReleaseId: 'target',
		} as never),
		{ currentReleaseId: 'current', targetReleaseId: 'target' },
	)
})

test('dependency detail resolves deterministic current and target release slots', () => {
	const cases = [
		{
			change: { kind: 'upgrade', currentReleaseId: 'old', targetReleaseId: 'new' },
			expected: {
				currentReleaseId: 'old',
				targetReleaseId: 'new',
				current: 'old-label',
				target: 'new-label',
			},
		},
		{
			change: { kind: 'add', currentReleaseId: null, targetReleaseId: 'new' },
			expected: {
				currentReleaseId: null,
				targetReleaseId: 'new',
				current: null,
				target: 'new-label',
			},
		},
		{
			change: { kind: 'remove', currentReleaseId: 'old', targetReleaseId: null },
			expected: {
				currentReleaseId: 'old',
				targetReleaseId: null,
				current: 'old-label',
				target: null,
			},
		},
	]
	for (const { change, expected } of cases) {
		assert.deepEqual(
			resolveConfirmDependencyReleases(change as never, (releaseId) =>
				releaseId ? `${releaseId}-label` : null,
			),
			expected,
		)
	}
})

test('confirm target loader label shows explicit version or honest automatic policy', () => {
	assert.equal(confirmTargetLoaderLabel('Fabric', 'fabric', '0.18.4', 'automatic'), 'Fabric 0.18.4')
	assert.equal(
		confirmTargetLoaderLabel('Fabric', 'fabric', null, 'automatic'),
		'Fabric (automatic)',
	)
	assert.equal(confirmTargetLoaderLabel('Vanilla', 'vanilla', null, 'automatic'), 'Vanilla')
})

test('fixed constraints replace and remove by physical content without duplicates', () => {
	const first = {
		contentId: 'a',
		provider: 'modrinth' as const,
		projectId: 'project',
		versionId: 'one',
	}
	const replaced = setFixedConstraint([first], { ...first, versionId: 'two' }, 'a')
	assert.deepEqual(replaced, [{ ...first, versionId: 'two' }])
	assert.deepEqual(setFixedConstraint(replaced, null, 'a'), [])
	assert.equal(customConstraintsEqual(replaced, [{ ...first, versionId: 'two' }]), true)
})

test('editable roots exclude automatic dependencies', () => {
	const root = {
		contentId: 'root',
		autoDependency: false,
		provider: 'modrinth',
		projectId: 'root',
		candidateReleaseIds: ['one'],
	}
	const dependency = { ...root, contentId: 'dependency', autoDependency: true }
	const plan = { items: [root, dependency], customConstraints: [] } as InstanceUpgradePlan
	assert.deepEqual(
		editableUpgradeRoots(plan).map((item) => item.contentId),
		['root'],
	)
})

test('unavailable minimal solution is not selectable', () => {
	const newestSolution = { kind: 'newest', selections: [], dependencyChanges: [], warnings: [] }
	assert.deepEqual(
		availablePredefinedStrategies({
			newestSolution,
			minimalChangeSolution: null,
		} as InstanceUpgradePlan),
		['newest'],
	)
})

test('issue grouping gives blocking precedence and includes every content once', () => {
	const plan = {
		items: [planItem('blocked'), planItem('warned'), planItem('clear')],
		blockingIssues: [issue('dependency_conflict', 'blocked', 'blocked')],
		warnings: [
			issue('keep_incompatible', 'blocked', 'blocked'),
			issue('keep_incompatible', 'warned', 'warned'),
		],
	} as InstanceUpgradePlan
	const groups = groupUpgradeIssues(plan)

	assert.deepEqual(
		groups.blocking.map((group) => group.item.contentId),
		['blocked'],
	)
	assert.deepEqual(
		groups.warnings.map((group) => group.item.contentId),
		['warned'],
	)
	assert.deepEqual(
		groups.noIssues.map((group) => group.item.contentId),
		['clear'],
	)
	assert.equal(
		new Set(
			[...groups.blocking, ...groups.warnings, ...groups.noIssues].map(
				(group) => group.item.contentId,
			),
		).size,
		3,
	)
})

test('initial blockers stay in blocking presentation without duplication after resolution', () => {
	const initialPlan = {
		items: [planItem('voxy'), planItem('clear')],
		blockingIssues: [issue('prerelease_only', 'voxy', 'voxy')],
		warnings: [],
	} as InstanceUpgradePlan
	const initial = captureInitialUpgradeBlockingIssues(initialPlan)
	const resolvedPlan = {
		...initialPlan,
		blockingIssues: [],
		warnings: [issue('keep_incompatible', 'voxy', 'voxy')],
	} as InstanceUpgradePlan
	const groups = groupUpgradeIssues(resolvedPlan, initial)

	assert.deepEqual(
		groups.blocking.map((group) => group.item.contentId),
		['voxy'],
	)
	assert.equal(groups.blocking[0].currentlyBlocking, false)
	assert.equal(groups.blocking[0].warnings.length, 2)
	assert.equal(groups.warnings.length, 0)
	assert.deepEqual(
		groups.noIssues.map((group) => group.item.contentId),
		['clear'],
	)
})

test('resolution presentation follows authoritative plan resolution rules', () => {
	const resolution = planItem('item').resolution
	assert.deepEqual(upgradeResolutionPresentation('two-option', resolution), {
		selectedAction: null,
		showUndo: false,
	})
	assert.deepEqual(upgradeResolutionPresentation('two-option', { ...resolution, action: 'keep' }), {
		selectedAction: 'keep',
		showUndo: false,
	})
	assert.deepEqual(
		upgradeResolutionPresentation('two-option', { ...resolution, action: 'disable' }),
		{ selectedAction: 'disable', showUndo: false },
	)
	assert.deepEqual(upgradeResolutionPresentation('single-prerelease', resolution), {
		selectedAction: null,
		showUndo: false,
	})
	assert.deepEqual(
		upgradeResolutionPresentation('single-prerelease', {
			...resolution,
			allowPrerelease: true,
		}),
		{ selectedAction: 'upgrade', showUndo: true },
	)
})

test('root and content forms of one issue coalesce on exact project identity', () => {
	const plan = {
		items: [planItem('content', 'project')],
		blockingIssues: [
			issue('no_compatible_release', null, 'project', 'root form'),
			issue('no_compatible_release', 'content', 'project', 'content form'),
		],
		warnings: [],
	} as InstanceUpgradePlan
	const groups = groupUpgradeIssues(plan)

	assert.equal(groups.blocking[0].blockingIssues.length, 1)
	assert.equal(groups.blocking[0].blockingIssues[0].message, 'content form')
})

test('unmapped and ambiguous project issues remain global', () => {
	const duplicate = planItem('duplicate-b', 'duplicate')
	const plan = {
		items: [planItem('duplicate-a', 'duplicate'), duplicate],
		blockingIssues: [issue('dependency_conflict', null, 'missing')],
		warnings: [issue('keep_incompatible', null, 'duplicate')],
	} as InstanceUpgradePlan
	const groups = groupUpgradeIssues(plan)

	assert.equal(groups.globalBlockingIssues.length, 1)
	assert.equal(groups.globalWarnings.length, 1)
	assert.equal(groups.noIssues.length, 2)
})

test('actionable warning filtering excludes global and informational conflicts', () => {
	const plan = {
		items: [planItem('actionable'), planItem('conflict')],
		blockingIssues: [],
		warnings: [
			issue('keep_incompatible', 'actionable', 'actionable'),
			issue('dependency_conflict', 'conflict', 'conflict'),
			issue('keep_incompatible', null, null),
		],
	} as InstanceUpgradePlan

	assert.deepEqual(actionableWarningContentIds(groupUpgradeIssues(plan)), ['actionable'])
})

test('actionable warning count uses unique content rows', () => {
	const plan = {
		items: [planItem('one'), planItem('two')],
		blockingIssues: [],
		warnings: [
			issue('keep_incompatible', 'one', 'one'),
			issue('shader_runtime_unknown', 'one', 'one'),
			issue('unidentified', 'two', 'two'),
		],
	} as InstanceUpgradePlan

	assert.equal(actionableWarningContentIds(groupUpgradeIssues(plan)).length, 2)
})

test('content display metadata prefers normalized content then snapshot then plan fallback', () => {
	const item = planItem('entry', 'plan-project') as never
	const snapshot = {
		expectedRelativePath: 'resourcepacks/file.zip',
		content: {
			project: { title: 'Snapshot title', icon_url: 'snapshot.png' },
			version: { version_number: 'snapshot-version' },
		},
	} as never
	const content = {
		project: { title: 'Resolved title', icon_url: 'resolved.png' },
		version: { version_number: 'resolved-version' },
	} as never

	assert.deepEqual(upgradeContentDisplayMetadata(item, content, snapshot), {
		title: 'Resolved title',
		iconUrl: 'resolved.png',
		currentVersion: 'resolved-version',
	})
	assert.deepEqual(upgradeContentDisplayMetadata(item, undefined, snapshot), {
		title: 'Snapshot title',
		iconUrl: 'snapshot.png',
		currentVersion: 'snapshot-version',
	})
	assert.equal(upgradeContentDisplayMetadata(item).title, 'entry.jar')
})

test('local content identity joins by normalized path when entry ids are absent', () => {
	assert.deepEqual(contentIdentityKeys({ relativePath: 'resourcepacks\\pack.zip' }), [
		'resourcepacks/pack.zip',
	])
	assert.deepEqual(
		contentIdentityKeys({ instanceEntryId: 'entry', relativePath: 'resourcepacks/pack.zip' }),
		['entry', 'resourcepacks/pack.zip'],
	)
})

test('Minecraft formatting codes are removed from display title only', () => {
	const item = planItem('identity')
	item.relativePath = 'resourcepacks/§9§lExample §rPack.zip'
	const originalPath = item.relativePath
	assert.equal(sanitizeMinecraftDisplayTitle('§9§lExample §rPack'), 'Example Pack')
	assert.equal(upgradeContentDisplayMetadata(item).title, 'Example Pack.zip')
	assert.equal(item.relativePath, originalPath)
	assert.equal(item.contentId, 'identity')
})
