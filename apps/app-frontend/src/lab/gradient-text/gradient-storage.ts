import {
	cloneGradientDocument,
	DEFAULT_GRADIENT_COLORS,
	DEFAULT_GRADIENT_DOCUMENT,
	gradientFormatAdapters,
	type GradientFormatId,
	type GradientTextDocument,
	normalizeGradientDocument,
	normalizeHexColor,
} from './gradient-text.ts'

export type GradientPreset = {
	id: string
	name: string
	colors: string[]
	createdAt: string
}

export type GradientTextLabState = {
	version: 1
	document: GradientTextDocument
	colors: string[]
	adapterId: GradientFormatId
	vanillaCharacter: '&' | '§'
	simplifyGradients: boolean
	presets: GradientPreset[]
}

type StorageLike = Pick<Storage, 'getItem' | 'setItem'>

export const GRADIENT_TEXT_STORAGE_KEY = 'axolotl.lab.gradient-text.v1'

export function createDefaultGradientTextState(): GradientTextLabState {
	return {
		version: 1,
		document: cloneGradientDocument(DEFAULT_GRADIENT_DOCUMENT),
		colors: [...DEFAULT_GRADIENT_COLORS],
		adapterId: 'vanilla',
		vanillaCharacter: '&',
		simplifyGradients: false,
		presets: [],
	}
}

export function loadGradientTextState(
	storage: StorageLike | null = getBrowserStorage(),
): GradientTextLabState {
	const fallback = createDefaultGradientTextState()
	if (!storage) return fallback

	try {
		const raw = storage.getItem(GRADIENT_TEXT_STORAGE_KEY)
		if (!raw) return fallback
		return sanitizeGradientTextState(JSON.parse(raw), fallback)
	} catch {
		return fallback
	}
}

export function saveGradientTextState(
	state: GradientTextLabState,
	storage: StorageLike | null = getBrowserStorage(),
): void {
	if (!storage) return
	storage.setItem(GRADIENT_TEXT_STORAGE_KEY, JSON.stringify(sanitizeGradientTextState(state)))
}

export function parseGradientPresets(value: unknown): GradientPreset[] {
	if (!Array.isArray(value)) return []
	return value.flatMap((preset, index) => {
		if (!preset || typeof preset !== 'object') return []
		const record = preset as Partial<GradientPreset>
		const colors = sanitizeColors(record.colors)
		if (!colors.length) return []
		return [
			{
				id:
					typeof record.id === 'string' && record.id
						? record.id
						: `imported-${index}-${Date.now()}`,
				name:
					typeof record.name === 'string' && record.name.trim()
						? record.name.trim().slice(0, 80)
						: `Preset ${index + 1}`,
				colors,
				createdAt:
					typeof record.createdAt === 'string' && !Number.isNaN(Date.parse(record.createdAt))
						? record.createdAt
						: new Date().toISOString(),
			},
		]
	})
}

export function serializeGradientPresets(presets: GradientPreset[]): string {
	return JSON.stringify(
		presets.map(({ name, colors }) => ({ name, colors })),
		null,
		2,
	)
}

function sanitizeGradientTextState(
	value: unknown,
	fallback = createDefaultGradientTextState(),
): GradientTextLabState {
	if (!value || typeof value !== 'object') return fallback
	const record = value as Partial<GradientTextLabState>
	const adapterId = gradientFormatAdapters.some((adapter) => adapter.id === record.adapterId)
		? (record.adapterId as GradientFormatId)
		: fallback.adapterId
	const colors = sanitizeColors(record.colors)
	return {
		version: 1,
		document: record.document ? normalizeGradientDocument(record.document) : fallback.document,
		colors: colors.length ? colors : fallback.colors,
		adapterId,
		vanillaCharacter: record.vanillaCharacter === '§' ? '§' : '&',
		simplifyGradients: record.simplifyGradients === true,
		presets: parseGradientPresets(record.presets),
	}
}

function sanitizeColors(value: unknown): string[] {
	if (!Array.isArray(value)) return []
	return value
		.map((color) => (typeof color === 'string' ? normalizeHexColor(color) : null))
		.filter((color): color is string => color !== null)
}

function getBrowserStorage(): StorageLike | null {
	return typeof window === 'undefined' ? null : window.localStorage
}
