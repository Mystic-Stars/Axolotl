import assert from 'node:assert/strict'
import test from 'node:test'

import {
	buildFontFamilyOptions,
	canonicalFontFamily,
	cssFontFamilyName,
	FOLLOW_DEFAULT_FONT,
	type FontFamilyOption,
	fontSettingToOptionValue,
	optionValueToFontSetting,
	resolveFontFamily,
	type SystemFontFamily,
} from './font-family.ts'

const FALLBACK = 'Inter, sans-serif'

function isDivider(option: ReturnType<typeof buildFontFamilyOptions>[number]): boolean {
	return !('value' in option)
}

function values(options: ReturnType<typeof buildFontFamilyOptions>): string[] {
	return options
		.filter((option): option is FontFamilyOption => 'value' in option)
		.map((o) => o.value)
}

test('quotes and escapes family names', () => {
	assert.equal(cssFontFamilyName('Inter'), '"Inter"')
	assert.equal(cssFontFamilyName('  Fira Sans  '), '"Fira Sans"')
	assert.equal(cssFontFamilyName('A"B'), '"A\\"B"')
	assert.equal(cssFontFamilyName('back\\slash'), '"back\\\\slash"')
	assert.equal(cssFontFamilyName('bad\u0000name'), '"badname"')
})

test('rejects family names that cannot be used', () => {
	assert.equal(cssFontFamilyName(''), null)
	assert.equal(cssFontFamilyName('   '), null)
	assert.equal(cssFontFamilyName('x'.repeat(129)), null)
})

test('appends the fallback stack after the chosen family', () => {
	assert.equal(resolveFontFamily(null, FALLBACK), FALLBACK)
	assert.equal(resolveFontFamily(undefined, FALLBACK), FALLBACK)
	assert.equal(resolveFontFamily('', FALLBACK), FALLBACK)
	assert.equal(resolveFontFamily('Fira Sans', FALLBACK), '"Fira Sans", Inter, sans-serif')
	assert.equal(resolveFontFamily('x'.repeat(129), FALLBACK), FALLBACK)
})

test('round trips the picker sentinel through the stored setting', () => {
	assert.equal(fontSettingToOptionValue(null), FOLLOW_DEFAULT_FONT)
	assert.equal(fontSettingToOptionValue('Fira Code'), 'Fira Code')
	assert.equal(optionValueToFontSetting(FOLLOW_DEFAULT_FONT), null)
	assert.equal(optionValueToFontSetting('  Fira Code  '), 'Fira Code')
})

test('lists the default entry first and sorts the rest case insensitively', () => {
	const options = buildFontFamilyOptions(
		[
			{ family: 'zeta', monospaced: false },
			{ family: 'Alpha', monospaced: false },
		],
		{ defaultLabel: 'Launcher default' },
	)

	assert.deepEqual(values(options), [FOLLOW_DEFAULT_FONT, 'Alpha', 'zeta'])
	assert.equal((options[0] as FontFamilyOption).label, 'Launcher default')
})

test('merges duplicate families and keeps the monospace flag', () => {
	const fonts: SystemFontFamily[] = [
		{ family: 'Fira Code', monospaced: false },
		{ family: 'fira code', monospaced: true },
		{ family: '   ', monospaced: false },
		{ family: 'Inter', monospaced: false },
	]

	const options = buildFontFamilyOptions(fonts, {
		defaultLabel: 'Default',
		monospaceLabel: 'Monospace',
		groupMonospaced: true,
	})

	assert.deepEqual(values(options), [FOLLOW_DEFAULT_FONT, 'Fira Code', 'Inter'])
	assert.equal((options[1] as FontFamilyOption).subLabel, 'Monospace')
	assert.equal(isDivider(options[2]), true)
})

test('groups monospaced families ahead of the rest with a divider', () => {
	const options = buildFontFamilyOptions(
		[
			{ family: 'Inter', monospaced: false },
			{ family: 'Consolas', monospaced: true },
		],
		{
			defaultLabel: 'Default',
			monospaceLabel: 'Monospace',
			groupMonospaced: true,
		},
	)

	assert.deepEqual(values(options), [FOLLOW_DEFAULT_FONT, 'Consolas', 'Inter'])
	assert.equal(isDivider(options[2]), true)
	assert.equal((options[1] as FontFamilyOption).searchTerms?.includes('Monospace'), true)
	assert.equal((options[3] as FontFamilyOption).subLabel, undefined)
})

test('omits the divider when only one group has entries', () => {
	const options = buildFontFamilyOptions([{ family: 'Consolas', monospaced: true }], {
		defaultLabel: 'Default',
		monospaceLabel: 'Monospace',
		groupMonospaced: true,
	})

	assert.equal(options.some(isDivider), false)
})

test('re-adds a selected family that is no longer installed', () => {
	const options = buildFontFamilyOptions([{ family: 'Inter', monospaced: false }], {
		defaultLabel: 'Default',
		missingLabel: 'Not installed',
		selected: 'JetBrains Mono',
	})

	assert.deepEqual(values(options), [FOLLOW_DEFAULT_FONT, 'JetBrains Mono', 'Inter'])
	assert.equal((options[1] as FontFamilyOption).subLabel, 'Not installed')
})

test('does not duplicate a selected family that is installed', () => {
	const options = buildFontFamilyOptions([{ family: 'JetBrains Mono', monospaced: true }], {
		defaultLabel: 'Default',
		missingLabel: 'Not installed',
		selected: 'JetBrains Mono',
	})

	assert.deepEqual(values(options), [FOLLOW_DEFAULT_FONT, 'JetBrains Mono'])
	assert.equal((options[1] as FontFamilyOption).subLabel, undefined)
})

test('canonicalises a stored family that differs only in case', () => {
	const fonts: SystemFontFamily[] = [{ family: 'JetBrains Mono', monospaced: true }]

	assert.equal(canonicalFontFamily(fonts, 'jetbrains mono'), 'JetBrains Mono')
	assert.equal(canonicalFontFamily(fonts, '  Inter  '), 'Inter')
	assert.equal(canonicalFontFamily(fonts, null), FOLLOW_DEFAULT_FONT)
	assert.equal(canonicalFontFamily(fonts, ''), FOLLOW_DEFAULT_FONT)
	assert.equal(canonicalFontFamily(fonts, 'JetBrains Mono'), 'JetBrains Mono')
})
