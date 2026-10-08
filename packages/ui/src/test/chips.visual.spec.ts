import { describe, expect, it, vi } from 'vitest'

import Chips from '../components/base/Chips.vue'
import { mountThemed } from './visual-harness'

describe('Chips', () => {
    it('exposes its selected option as a semantic radio state', async () => {
        const wrapper = await mountThemed(
            Chips,
            {
                items: ['first-option', 'second-option'],
                modelValue: 'second-option',
                ariaLabel: 'Choose an option',
            },
            'dark',
        )
        const group = wrapper.get('[role="radiogroup"]')
        const radios = wrapper.findAll<HTMLButtonElement>('[role="radio"]')

        expect(group.attributes('aria-label')).toBe('Choose an option')
        expect(radios.map((radio) => radio.attributes('aria-checked'))).toEqual(['false', 'true'])
        expect(radios.map((radio) => radio.attributes('data-state'))).toEqual([
            'unchecked',
            'checked',
        ])
        expect(radios[0].text()).toBe('first-option')
        expect(getComputedStyle(radios[0].element).textTransform).toBe('capitalize')
        expect(radios[1].classes()).toContain('button-frame--chip')
    })

    it('preserves selection and never-empty behavior', async () => {
        const onUpdate = vi.fn()
        const wrapper = await mountThemed(
            Chips,
            { items: ['one', 'two'], modelValue: 'one', 'onUpdate:modelValue': onUpdate },
            'dark',
        )
        const radios = wrapper.findAll<HTMLButtonElement>('[role="radio"]')

        radios[0].element.click()
        await wrapper.vm.$nextTick()
        expect(onUpdate).not.toHaveBeenCalled()

        radios[1].element.click()
        await wrapper.vm.$nextTick()
        expect(onUpdate).toHaveBeenCalledWith('two')

        const onOptionalUpdate = vi.fn()
        const optional = await mountThemed(
            Chips,
            {
                items: ['one'],
                modelValue: 'one',
                neverEmpty: false,
                'onUpdate:modelValue': onOptionalUpdate,
            },
            'dark',
        )
        optional.get<HTMLButtonElement>('[role="radio"]').element.click()
        await optional.vm.$nextTick()
        expect(onOptionalUpdate).toHaveBeenCalledWith(null)
    })

    it('keeps sizing, disabled state, formatting, and disabled tooltips', async () => {
        const wrapper = await mountThemed(
            Chips,
            {
                items: [1, 2],
                modelValue: 1,
                size: 'small',
                capitalize: false,
                formatLabel: (item: number) => `Option ${item}`,
                disabledItems: [2],
                disabledTooltip: (item: number) => `Option ${item} is unavailable`,
            },
            'dark',
        )
        const radios = wrapper.findAll<HTMLButtonElement>('[role="radio"]')

        expect(radios.map((radio) => radio.text())).toEqual(['Option 1', 'Option 2'])
        expect(radios[0].classes()).not.toContain('capitalize')
        expect(getComputedStyle(radios[0].element).height).toBe('32px')
        expect(radios[1].element.disabled).toBe(true)

        radios[1].element.dispatchEvent(new MouseEvent('mouseenter'))
        await new Promise((resolve) => setTimeout(resolve, 250))
        expect(document.querySelector('[role="tooltip"]')?.textContent).toContain(
            'Option 2 is unavailable',
        )
    })
})
