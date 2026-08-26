import type { AbstractModrinthClient } from '@modrinth/api-client'

const textEncoder = new TextEncoder()
const textDecoder = new TextDecoder()

/**
 * Maximum size in bytes of the log content sent to mclo.gs. Logs
 * larger than this are trimmed to their last `LOG_SHARE_MAX_BYTES` before
 * uploading, and the caller is notified through `LogShareResult.truncated`.
 */
export const LOG_SHARE_MAX_BYTES = 9 * 1024 * 1024

/**
 * Return the tail of `content` that fits within `maxBytes` of UTF-8 without
 * splitting a multi-byte code point. Returns the original string when it
 * already fits.
 */
export function truncateLogToMaxBytes(content: string, maxBytes: number): string {
	if (!content) return content
	const bytes = textEncoder.encode(content)
	if (bytes.byteLength <= maxBytes) return content

	// Keep the last `maxBytes` bytes, starting at the first byte that begins a
	// UTF-8 code point so decoding never produces replacement characters.
	let start = bytes.byteLength - maxBytes
	while (start < bytes.byteLength && (bytes[start] & 0xc0) === 0x80) start++

	return textDecoder.decode(bytes.subarray(start))
}

/**
 * Result of sharing log content. `truncated` is set when the content exceeded
 * `LOG_SHARE_MAX_BYTES` and only its tail was uploaded.
 */
export type LogShareResult = {
	url: string
	truncated: boolean
}

/**
 * Share log content through mclo.gs. Content larger than `LOG_SHARE_MAX_BYTES`
 * is trimmed to its last
 * `LOG_SHARE_MAX_BYTES` before uploading.
 */
export async function shareLogs(
	client: AbstractModrinthClient,
	content: string,
): Promise<LogShareResult> {
	const uploadContent = truncateLogToMaxBytes(content, LOG_SHARE_MAX_BYTES)
	const truncated = uploadContent !== content
	const data = await client.mclogs.logs_v1.create(uploadContent)
	if (data.success && data.url) return { url: data.url, truncated }
	throw new Error('mclo.gs upload failed')
}
