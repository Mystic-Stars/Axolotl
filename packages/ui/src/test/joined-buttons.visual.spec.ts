import { describe, expect, it } from 'vitest'
import { type Component, defineComponent, h } from 'vue'

import Button from '../components/base/buttons/Button.vue'
import ButtonGroup from '../components/base/buttons/ButtonGroup.vue'
import ButtonLink from '../components/base/buttons/ButtonLink.vue'
import { mountThemed } from './visual-harness'

const CurrentGroup = defineComponent({
    render() {
        return h(ButtonGroup, null, {
            default: () => [
                h(
                    Button as Component,
                    { type: 'colored', color: 'brand', size: 'xl' },
                    { default: () => 'Launch' },
                ),
                h(Button as Component, { type: 'quiet', size: 'xl' }, { default: () => 'Options' }),
            ],
        })
    },
})

const MixedSemanticGroup = defineComponent({
    render() {
        return h(ButtonGroup, null, {
            default: () => [
                h(Button as Component, { size: 'md' }, { default: () => 'Save' }),
                h(
                    ButtonLink as Component,
                    { href: '/help', size: 'md' },
                    { default: () => 'Help' },
                ),
            ],
        })
    },
})

function expectJoinedCorners(left: HTMLElement, right: HTMLElement) {
    const leftStyle = getComputedStyle(left)
    const rightStyle = getComputedStyle(right)

    expect(leftStyle.borderTopRightRadius).toBe('0px')
    expect(leftStyle.borderBottomRightRadius).toBe('0px')
    expect(Number.parseFloat(leftStyle.borderTopLeftRadius)).toBeGreaterThan(0)
    expect(rightStyle.borderTopLeftRadius).toBe('0px')
    expect(rightStyle.borderBottomLeftRadius).toBe('0px')
    expect(Number.parseFloat(rightStyle.borderTopRightRadius)).toBeGreaterThan(0)
}

describe('joined buttons', () => {
    it.each([CurrentGroup, MixedSemanticGroup])(
        'joins direct current-generation button roots',
        async (group) => {
            const root = (await mountThemed(group, {}, 'dark')).element as HTMLElement
            const buttons = root.querySelectorAll<HTMLElement>(':scope > [data-button]')

            expect(buttons).toHaveLength(2)
            expectJoinedCorners(buttons[0], buttons[1])
        },
    )
})
