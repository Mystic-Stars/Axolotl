import assert from 'node:assert/strict'
import test from 'node:test'

import {
	buildGradientCharacters,
	createDocumentFromPlainText,
	generateGradientOutput,
	getMinecraftTextShadow,
	gradientFormatAdapters,
	interpolateGradient,
	parseGradientColors,
	type TextFormat,
} from './gradient-text.ts'

const options = { vanillaCharacter: '&' as const, simplifyGradients: false }

test('interpolates every color stop including both endpoints', () => {
	assert.deepEqual(interpolateGradient(['#000000', '#FFFFFF'], 3), [
		'#000000',
		'#808080',
		'#FFFFFF',
	])
	assert.deepEqual(interpolateGradient(['#FF0000', '#00FF00', '#0000FF'], 5), [
		'#FF0000',
		'#808000',
		'#00FF00',
		'#008080',
		'#0000FF',
	])
})

test('does not spend a gradient color on whitespace and supports Unicode code points', () => {
	const characters = buildGradientCharacters(createDocumentFromPlainText('A 🦎'), [
		'#000000',
		'#FFFFFF',
	])
	assert.deepEqual(
		characters.map((character) => [character.character, character.color]),
		[
			['A', '#000000'],
			[' ', null],
			['🦎', '#FFFFFF'],
		],
	)
})

test('uses one color stop as a solid text color', () => {
	const characters = buildGradientCharacters(createDocumentFromPlainText('Solid'), ['#12AB34'])
	assert.deepEqual(
		characters.map((character) => character.color),
		['#12AB34', '#12AB34', '#12AB34', '#12AB34', '#12AB34'],
	)
	assert.equal(
		generateGradientOutput(createDocumentFromPlainText('A'), ['#12AB34'], 'vanilla', options),
		'&#12AB34A',
	)
})

test('matches Minecraft preview text shadow colors', () => {
	assert.equal(getMinecraftTextShadow('#FF0000'), '#800000')
	assert.equal(getMinecraftTextShadow(null), '#000000')
})

test('keeps multiline text and formatting in generated output', () => {
	const document = {
		lines: [[{ text: 'A', formats: ['bold', 'italic'] as const }], [{ text: 'B', formats: [] }]],
	}
	assert.equal(
		generateGradientOutput(document, ['#123456', '#ABCDEF'], 'vanilla', options),
		'&#123456&l&oA\n&#ABCDEFB',
	)
	assert.equal(
		generateGradientOutput(document, ['#123456', '#ABCDEF'], 'json', options),
		'[{"text":"A","color":"#123456","bold":true,"italic":true},{"text":"\\n"},{"text":"B","color":"#ABCDEF"}]',
	)
})

test('uses the selected legacy control character and resets formatting with the next color', () => {
	const document = {
		lines: [
			[
				{ text: 'A', formats: ['bold'] as TextFormat[] },
				{ text: 'B', formats: [] as TextFormat[] },
			],
		],
	}
	assert.equal(
		generateGradientOutput(document, ['#000000', '#FFFFFF'], 'vanilla', options),
		'&#000000&lA&#FFFFFFB',
	)
	assert.equal(
		generateGradientOutput(document, ['#000000', '#FFFFFF'], 'vanilla', {
			vanillaCharacter: '§',
			simplifyGradients: false,
		}),
		'§#000000§lA§#FFFFFFB',
	)
})

test('all registered adapters generate a non-empty result', () => {
	const document = createDocumentFromPlainText('Lab')
	assert.equal(gradientFormatAdapters.length, 19)
	for (const adapter of gradientFormatAdapters) {
		const output = generateGradientOutput(document, ['#AA00FF', '#00FFAA'], adapter.id, options)
		assert.notEqual(output, '', adapter.id)
	}
})

test('parses HEX, RGB, and CSS gradient color input', () => {
	assert.deepEqual(parseGradientColors('linear-gradient(90deg, #a0b, rgb(12, 34, 56), #ABCDEF)'), [
		'#AA00BB',
		'#ABCDEF',
		'#0C2238',
	])
})
