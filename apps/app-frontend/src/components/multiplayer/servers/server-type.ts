import type { ServerTypeId } from '@modrinth/server'

/** Color, monogram and icon used to badge a server type across cards and the wizard. */
export interface ServerTypeMeta {
	colorVar: string
	monogram: string
}

const PLATFORM_ID = (id: ServerTypeId) => `var(--color-platform-${id})`

export const SERVER_TYPE_META: Record<ServerTypeId, ServerTypeMeta> = {
	vanilla: { colorVar: 'var(--color-brand)', monogram: 'V' },
	fabric: { colorVar: PLATFORM_ID('fabric'), monogram: 'F' },
	paper: { colorVar: PLATFORM_ID('paper'), monogram: 'P' },
	forge: { colorVar: PLATFORM_ID('forge'), monogram: 'Fo' },
	neoforge: { colorVar: PLATFORM_ID('neoforge'), monogram: 'N' },
	quilt: { colorVar: PLATFORM_ID('quilt'), monogram: 'Q' },
}
