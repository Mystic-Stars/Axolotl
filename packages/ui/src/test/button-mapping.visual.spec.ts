import { describe, expect, it } from 'vitest'

import Button from '../components/base/buttons/Button.vue'
import type { ButtonSize } from '../components/base/buttons/types'
import { mountThemed } from './visual-harness'

const EXPECTED_SIZES: Record<ButtonSize, { height: string; radius: string }> = {
    '2xs': { height: '24px', radius: '8px' },
    xs: { height: '28px', radius: '8px' },
    sm: { height: '32px', radius: '10px' },
    md: { height: '36px', radius: '12px' },
    lg: { height: '40px', radius: '14px' },
    xl: { height: '48px', radius: '16px' },
}

async function measure(props: Record<string, unknown>, content = 'Label') {
    const wrapper = await mountThemed(Button, props, 'dark', { slots: { default: content } })
    const style = getComputedStyle(wrapper.element as HTMLElement)
    return {
        width: style.width,
        height: style.height,
        radius: style.borderRadius,
        weight: style.fontWeight,
        textColor: style.color,
    }
}

describe('current button geometry', () => {
    it.each(Object.entries(EXPECTED_SIZES))(
        'keeps %s on its explicit size contract',
        async (size, expected) => {
            const geometry = await measure({ size })
            expect(geometry.height).toBe(expected.height)
            expect(geometry.radius).toBe(expected.radius)
            expect(Number.parseFloat(geometry.width)).toBeGreaterThan(0)
        },
    )

    it('keeps the size ladder strictly ordered', async () => {
        const sizes: ButtonSize[] = ['2xs', 'xs', 'sm', 'md', 'lg', 'xl']
        const heights = await Promise.all(
            sizes.map(async (size) => (await measure({ size })).height),
        )
        const numeric = heights.map((height) => Number.parseFloat(height))

        expect(numeric).toEqual([...numeric].sort((a, b) => a - b))
        expect(new Set(numeric).size).toBe(sizes.length)
    })

    it.each([
        ['base', {}],
        ['outlined', { type: 'outlined' }],
        ['quiet', { type: 'quiet' }],
        ['colored', { type: 'colored', color: 'brand' }],
        ['chip', { type: 'chip', color: 'brand' }],
        ['chip-text', { type: 'chip-text', color: 'brand' }],
        ['highlight', { type: 'highlight', color: 'brand' }],
    ] as const)('renders a non-empty %s variant', async (_name, props) => {
        const geometry = await measure(props)
        expect(Number.parseFloat(geometry.width)).toBeGreaterThan(0)
        expect(geometry.height).toBe('36px')
    })

    it('keeps xl icon-only buttons square and circular', async () => {
        const icon = '<svg width="24" height="24" aria-hidden="true"></svg>'
        const current = await measure(
            { size: 'xl', circular: true, iconOnly: true, label: 'Open menu' },
            icon,
        )
        const padded = await measure({ size: 'xl' }, icon)

        expect(current.width).toBe('48px')
        expect(current.height).toBe('48px')
        expect(Number.parseFloat(current.radius)).toBeGreaterThanOrEqual(24)
        expect(Number.parseFloat(padded.width)).toBeGreaterThan(48)
        expect(Number.parseFloat(padded.radius)).toBeLessThan(24)
    })
})
