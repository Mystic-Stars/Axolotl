const encoder = new TextEncoder()
const decoder = new TextDecoder()

const replacements: Array<[RegExp, string]> = [
    [/\bbearer\s+[a-z0-9._~+/=-]+/gi, 'Bearer <redacted>'],
    [
        /\b(authorization|x-api-key|api[_-]?key|access[_-]?token|refresh[_-]?token|client[_-]?secret|token)\b\s*[:=]\s*[^\s,;]+/gi,
        '$1=<redacted>',
    ],
    [
        /([?&](?:token|access_token|refresh_token|api_key|key|code|secret|session|signature)=)[^&#\s]+/gi,
        '$1<redacted>',
    ],
    [/\b[a-z0-9.!#$%&'*+/=?^_`{|}~-]+@[a-z0-9.-]+\.[a-z]{2,}\b/gi, '<email>'],
    [/\b[a-z]:\\users\\[^\\/\s]+/gi, '<home>'],
    [/(?:\/home|\/users)\/[^/\s]+\//gi, '<home>/'],
    [/\b[0-9a-f]{8}-?[0-9a-f]{4}-?[0-9a-f]{4}-?[0-9a-f]{4}-?[0-9a-f]{12}\b/gi, '<uuid>'],
]

export function redact(input: string): string {
    return replacements.reduce(
        (value, [pattern, replacement]) => value.replace(pattern, replacement),
        input.replaceAll('\0', ''),
    )
}

export function truncateUtf8(input: string, maxBytes: number): string {
    const bytes = encoder.encode(input)
    if (bytes.byteLength <= maxBytes) return input
    return decoder.decode(bytes.slice(0, maxBytes)).replace(/\uFFFD$/, '')
}

export function byteLength(input: string): number {
    return encoder.encode(input).byteLength
}
