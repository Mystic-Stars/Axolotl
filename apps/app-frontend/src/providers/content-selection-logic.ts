export interface AggregatedDependency {
	id: string
	title: string
	requiredBy: string[]
	requiredByKeys?: string[]
	required?: boolean
	requiredForKeys?: string[]
}

export interface DependencyAggregationInput<T extends AggregatedDependency> {
	ownerKey: string
	dependencies: T[]
}

function dependencyIdentity(id: string) {
	const separator = id.lastIndexOf(':')
	return {
		identity: separator === -1 ? id : id.slice(0, separator),
		versionId: separator === -1 ? '' : id.slice(separator + 1),
	}
}

export function getActiveDependencyConflictIdentities(
	dependencies: Pick<AggregatedDependency, 'id' | 'requiredByKeys'>[],
	visibleOwnerKeys: Set<string>,
) {
	const versions = new Map<string, Set<string>>()
	for (const dependency of dependencies) {
		if (
			dependency.requiredByKeys?.length &&
			!dependency.requiredByKeys.some((key) => visibleOwnerKeys.has(key))
		) {
			continue
		}
		const { identity, versionId } = dependencyIdentity(dependency.id)
		const identityVersions = versions.get(identity) ?? new Set<string>()
		identityVersions.add(versionId)
		versions.set(identity, identityVersions)
	}
	return new Set(
		[...versions.entries()]
			.filter(([, versionIds]) => versionIds.size > 1)
			.map(([identity]) => identity),
	)
}

export function aggregateContentSelectionDependencies<T extends AggregatedDependency>(
	selections: DependencyAggregationInput<T>[],
	conflictMessage: (dependency: T) => string,
) {
	const dependencies = new Map<string, T>()
	const versionOwners = new Map<string, Map<string, Set<string>>>()
	const dependencyByIdentity = new Map<string, T>()
	const conflicts = new Map<string, string>()
	const conflictIdentities = new Map<string, string[]>()

	for (const selection of selections) {
		for (const dependency of selection.dependencies) {
			const { identity, versionId } = dependencyIdentity(dependency.id)
			const ownersByVersion = versionOwners.get(identity) ?? new Map<string, Set<string>>()
			const owners = ownersByVersion.get(versionId) ?? new Set<string>()
			owners.add(selection.ownerKey)
			ownersByVersion.set(versionId, owners)
			versionOwners.set(identity, ownersByVersion)
			dependencyByIdentity.set(identity, dependency)

			const existing = dependencies.get(dependency.id)
			if (existing) {
				existing.requiredBy = [...new Set([...existing.requiredBy, ...dependency.requiredBy])]
				existing.requiredByKeys = [
					...new Set([...(existing.requiredByKeys ?? []), ...(dependency.requiredByKeys ?? [])]),
				]
				existing.required = existing.required || dependency.required
				existing.requiredForKeys = [
					...new Set([
						...(existing.requiredForKeys ?? []),
						...(dependency.required ? [selection.ownerKey] : []),
						...(dependency.requiredForKeys ?? []),
					]),
				]
			} else {
				dependencies.set(dependency.id, {
					...dependency,
					requiredForKeys: dependency.required
						? [...new Set([selection.ownerKey, ...(dependency.requiredForKeys ?? [])])]
						: dependency.requiredForKeys,
				})
			}
		}
	}

	for (const [identity, ownersByVersion] of versionOwners) {
		if (ownersByVersion.size < 2) continue
		const dependency = dependencyByIdentity.get(identity)
		if (!dependency) continue
		for (const owners of ownersByVersion.values()) {
			for (const ownerKey of owners) {
				conflicts.set(ownerKey, conflictMessage(dependency))
				conflictIdentities.set(ownerKey, [
					...new Set([...(conflictIdentities.get(ownerKey) ?? []), identity]),
				])
			}
		}
	}

	return { dependencies: [...dependencies.values()], conflicts, conflictIdentities }
}
