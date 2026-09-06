import { z } from 'zod'

const uuid = z.string().regex(/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i)
const timestamp = z.string().datetime({ offset: true })

export const heartbeatEventSchema = z
	.object({
		type: z.literal('heartbeat'),
		event_id: uuid,
		occurred_at: timestamp,
		day: z.string().regex(/^\d{4}-\d{2}-\d{2}$/),
	})
	.strict()

export const telemetryEventSchema = heartbeatEventSchema

export const batchSchema = z
	.object({
		schema_version: z.literal(1),
		batch_id: uuid,
		installation_id: uuid,
		app: z
			.object({
				version: z.string().min(1).max(64),
				environment: z.enum(['production', 'development']),
				platform: z.string().min(1).max(32),
				arch: z.string().min(1).max(32),
			})
			.strict(),
		events: z.array(telemetryEventSchema).min(1).max(10),
	})
	.strict()

export type TelemetryBatch = z.infer<typeof batchSchema>
