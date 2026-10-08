export function normalizeSchematicLayerRange(
    minimum: number,
    maximum: number,
    floor: number,
    ceiling: number,
): [number, number] {
    const low = Math.max(floor, Math.min(minimum, maximum, ceiling))
    const high = Math.min(ceiling, Math.max(minimum, maximum, floor))
    return [low, Math.max(low, high)]
}

export function escapeSchematicCsvCell(value: string | number) {
    const text = String(value)
    return /[",\r\n]/.test(text) ? `"${text.replaceAll('"', '""')}"` : text
}
