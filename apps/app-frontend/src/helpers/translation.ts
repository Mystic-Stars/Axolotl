import { renderHighlightedString } from '@modrinth/utils'
import { configuredXss } from '@modrinth/utils/parse'
import { invoke } from '@tauri-apps/api/core'

export type TranslationProvider = 'microsoft' | 'google' | 'openai-compatible'
export type TranslationMode = 'bilingual' | 'translation-only'
export type TranslationStyle = 'default' | 'weakened' | 'brand' | 'border' | 'background'
export type TranslationTextFormat = 'plain' | 'html'

export interface TranslationSettings {
	provider: TranslationProvider
	target_language: string
	mode: TranslationMode
	auto_translate: boolean
	style: TranslationStyle
	openai_base_url: string
	openai_model: string
	openai_has_api_key: boolean
}

export interface TranslationSegment {
	id: string
	text: string
	format: TranslationTextFormat
}

export interface TranslationRequest {
	source_language: string
	target_language: string
	context: {
		title: string
		description: string
	}
	segments: TranslationSegment[]
}

export interface TranslationResponse {
	segments: Array<{ id: string; text: string }>
}

interface ProtectedElement {
	tagName: string
	attributes: Array<[string, string]>
	innerHtml?: string
}

export interface PreparedDescriptionBlock {
	id: string
	originalHtml: string
	translatable: boolean
	protectedElements: Record<string, ProtectedElement>
}

export interface PreparedDescription {
	blocks: PreparedDescriptionBlock[]
	segments: TranslationSegment[]
}

export async function getTranslationSettings(): Promise<TranslationSettings> {
	return await invoke('plugin:translation|translation_get_settings')
}

export async function updateTranslationSettings(settings: TranslationSettings): Promise<void> {
	await invoke('plugin:translation|translation_update_settings', { settings })
}

export async function setTranslationSecret(
	provider: TranslationProvider,
	secret: string | null,
): Promise<void> {
	await invoke('plugin:translation|translation_set_secret', { provider, secret })
}

export async function testTranslationProvider(provider: TranslationProvider): Promise<string> {
	return await invoke('plugin:translation|translation_test_provider', { provider })
}

export async function translate(request: TranslationRequest): Promise<TranslationResponse> {
	return await invoke('plugin:translation|translation_translate', { request })
}

export async function clearTranslationCache(): Promise<void> {
	await invoke('plugin:translation|translation_clear_cache')
}

function containsReadableText(element: Element): boolean {
	if (element.matches('pre, script, style, video, audio, iframe')) return false
	const clone = element.cloneNode(true) as Element
	clone.querySelectorAll('pre, code, script, style').forEach((node) => node.remove())
	clone.querySelectorAll('a').forEach((node) => {
		if (isUrlOnlyText(node.textContent ?? '')) node.remove()
	})
	return (clone.textContent ?? '').trim().length > 0
}

function isUrlOnlyText(value: string): boolean {
	return /^(?:https?:\/\/|www\.|mailto:)[^\s]+$/i.test(value.trim())
}

function protectElementAttributes(
	element: Element,
	blockIndex: number,
): Record<string, ProtectedElement> {
	const protectedElements: Record<string, ProtectedElement> = {}
	const elements = [element, ...Array.from(element.querySelectorAll('*'))]

	elements.forEach((current, elementIndex) => {
		const marker = `${blockIndex}-${elementIndex}`
		const attributes = Array.from(current.attributes).map(
			(attribute) => [attribute.name, attribute.value] as [string, string],
		)
		protectedElements[marker] = {
			tagName: current.tagName,
			attributes,
			...(current.matches('code, pre') ||
			(current.matches('a') && isUrlOnlyText(current.textContent ?? ''))
				? { innerHtml: current.innerHTML }
				: {}),
		}

		Array.from(current.attributes).forEach((attribute) => current.removeAttribute(attribute.name))
		current.setAttribute('data-ax-translation-attr', marker)
		if (protectedElements[marker].innerHtml !== undefined) current.setAttribute('translate', 'no')
	})

	return protectedElements
}

export function prepareDescription(description: string): PreparedDescription {
	const document = new DOMParser().parseFromString(
		`<body>${renderHighlightedString(description ?? '')}</body>`,
		'text/html',
	)
	const blocks: PreparedDescriptionBlock[] = []
	const segments: TranslationSegment[] = []

	Array.from(document.body.children).forEach((source, index) => {
		const id = `body-${index}`
		const originalHtml = configuredXss.process(source.outerHTML)
		const translatable = containsReadableText(source)
		const clone = source.cloneNode(true) as Element
		const protectedElements = translatable ? protectElementAttributes(clone, index) : {}

		blocks.push({ id, originalHtml, translatable, protectedElements })
		if (translatable) {
			segments.push({ id, text: clone.outerHTML, format: 'html' })
		}
	})

	return { blocks, segments }
}

function restoreTranslatedBlock(block: PreparedDescriptionBlock, translatedHtml: string): string {
	const document = new DOMParser().parseFromString(`<body>${translatedHtml}</body>`, 'text/html')
	const root = document.body.firstElementChild
	const translatedElements = document.body.querySelectorAll('*')
	if (
		!root ||
		document.body.children.length !== 1 ||
		translatedElements.length !== Object.keys(block.protectedElements).length ||
		Array.from(translatedElements).some(
			(element) => !element.hasAttribute('data-ax-translation-attr'),
		)
	) {
		throw new Error(`Translation markup changed for block ${block.id}`)
	}

	for (const [marker, protectedElement] of Object.entries(block.protectedElements)) {
		const matches = document.body.querySelectorAll(`[data-ax-translation-attr="${marker}"]`)
		if (matches.length !== 1 || matches[0].tagName !== protectedElement.tagName) {
			throw new Error(`Translation markup changed for block ${block.id}`)
		}
		const element = matches[0]
		Array.from(element.attributes).forEach((attribute) => element.removeAttribute(attribute.name))
		protectedElement.attributes.forEach(([name, value]) => element.setAttribute(name, value))
		if (protectedElement.innerHtml !== undefined) element.innerHTML = protectedElement.innerHtml
	}

	return configuredXss.process(root.outerHTML)
}

function translationStyleClass(style: TranslationStyle): string {
	return `ax-translation-style-${style}`
}

function restorePreparedDescription(
	prepared: PreparedDescription,
	translations: Record<string, string>,
): Map<string, string> {
	const restored = new Map<string, string>()
	for (const block of prepared.blocks) {
		if (!block.translatable) continue
		const translated = translations[block.id]
		if (!translated) throw new Error(`Missing translated block ${block.id}`)
		restored.set(block.id, restoreTranslatedBlock(block, translated))
	}
	return restored
}

export function validateTranslatedDescription(
	prepared: PreparedDescription,
	translations: Record<string, string>,
): void {
	restorePreparedDescription(prepared, translations)
}

export function renderTranslatedDescription(
	prepared: PreparedDescription,
	translations: Record<string, string>,
	mode: TranslationMode,
	style: TranslationStyle,
): string {
	let restored: Map<string, string>
	try {
		restored = restorePreparedDescription(prepared, translations)
	} catch {
		return prepared.blocks.map((block) => block.originalHtml).join('')
	}

	return prepared.blocks
		.map((block) => {
			if (!block.translatable) return block.originalHtml
			const translated = restored.get(block.id) ?? block.originalHtml
			if (mode === 'translation-only') return translated
			return `${block.originalHtml}<div class="ax-translation-block ${translationStyleClass(style)}">${translated}</div>`
		})
		.join('')
}
