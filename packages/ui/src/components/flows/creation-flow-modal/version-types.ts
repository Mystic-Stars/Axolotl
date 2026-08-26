export type GameVersionType = 'release' | 'snapshot' | 'alpha' | 'ancient'

export const aprilFoolsVersions = new Set([
	'15w14a',
	'1.RV-Pre1',
	'3D Shareware v1.34',
	'20w14∞',
	'22w13oneblockatatime',
	'23w13a_or_b',
	'24w14potato',
	'25w14craftmine',
	'26w14a',
])

export function isVersionTypeMatch(
	versionType: string,
	versionId: string,
	selectedType: GameVersionType,
): boolean {
	switch (selectedType) {
		case 'release':
			return versionType === 'release'
		case 'snapshot':
			return versionType === 'snapshot' && !aprilFoolsVersions.has(versionId)
		case 'alpha':
			return aprilFoolsVersions.has(versionId)
		case 'ancient':
			return (
				!aprilFoolsVersions.has(versionId) &&
				(versionType === 'alpha' || versionType === 'beta' || versionType.startsWith('old_'))
			)
	}
}
