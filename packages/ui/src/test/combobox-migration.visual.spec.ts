import { expect, it } from 'vitest'
import { defineComponent, h, ref } from 'vue'

import Combobox, { type ComboboxOption } from '../components/base/Combobox.vue'
import { mountThemed, waitFor } from './visual-harness'

function installTeleportTarget() {
    const teleports = document.createElement('div')
    teleports.id = 'teleports'
    document.body.append(teleports)
    return teleports
}

function activate(element: HTMLElement) {
    element.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true, button: 0 }))
    element.dispatchEvent(new MouseEvent('mousedown', { bubbles: true, button: 0 }))
    element.dispatchEvent(new MouseEvent('mouseup', { bubbles: true, button: 0 }))
    element.dispatchEvent(new MouseEvent('click', { bubbles: true, detail: 1 }))
}

it('preserves scalar values while showing formatted option labels', async () => {
    const teleports = installTeleportTarget()
    const options: ComboboxOption<number>[] = [
        { value: 10, label: '10 results' },
        { value: 25, label: '25 results' },
    ]
    const Harness = defineComponent({
        setup() {
            const selected = ref(10)
            return () =>
                h(Combobox, {
                    modelValue: selected.value,
                    options,
                    'onUpdate:modelValue': (value: number) => {
                        selected.value = value
                    },
                })
        },
    })
    const wrapper = await mountThemed(Harness, {}, 'dark', { attachTo: document.body })

    const trigger = wrapper.get('[aria-haspopup="listbox"]')
    expect(trigger.text()).toContain('10 results')
    activate(trigger.element as HTMLElement)
    await waitFor(() => document.querySelectorAll('[role="option"]').length === 2, {
        label: 'number options to open',
    })

    const nextOption = Array.from(document.querySelectorAll<HTMLElement>('[role="option"]')).find(
        (option) => option.textContent?.includes('25 results'),
    )
    expect(nextOption).toBeDefined()
    nextOption?.click()
    await waitFor(() => trigger.text().includes('25 results'), {
        label: 'number option selection to update the model',
    })
    expect(trigger.text()).toContain('25 results')
    expect(trigger.text()).not.toContain('[object Object]')

    wrapper.unmount()
    teleports.remove()
})

it('passes the formatted selected label through the selected slot', async () => {
    const teleports = installTeleportTarget()
    const wrapper = await mountThemed(
        Combobox,
        {
            modelValue: 'date_modified',
            options: [{ value: 'date_modified', label: 'Date updated' }],
        },
        'dark',
        {
            attachTo: document.body,
            slots: {
                selected: ({ label }: { label: string }) =>
                    h('span', { 'data-testid': 'selected-label' }, `Sorted by ${label}`),
            },
        },
    )

    expect(wrapper.get('[data-testid="selected-label"]').text()).toBe('Sorted by Date updated')

    wrapper.unmount()
    teleports.remove()
})

it('keeps disabled listboxes closed and does not emit a selection', async () => {
    const teleports = installTeleportTarget()
    const wrapper = await mountThemed(
        Combobox,
        {
            modelValue: 'relevance',
            options: [
                { value: 'relevance', label: 'Relevance' },
                { value: 'downloads', label: 'Downloads' },
            ],
            disabled: true,
        },
        'dark',
        { attachTo: document.body },
    )

    const trigger = wrapper.get('[aria-haspopup="listbox"]')
    expect(trigger.attributes('aria-disabled')).toBe('true')
    activate(trigger.element as HTMLElement)
    await wrapper.vm.$nextTick()
    expect(document.querySelector('[role="listbox"]')).toBeNull()
    expect(wrapper.emitted('update:modelValue')).toBeUndefined()

    wrapper.unmount()
    teleports.remove()
})

it('honors an explicit upward direction for floating dropdowns', async () => {
    const teleports = installTeleportTarget()
    const wrapper = await mountThemed(
        Combobox,
        {
            modelValue: 'relevance',
            options: [{ value: 'relevance', label: 'Relevance' }],
            forceDirection: 'up',
        },
        'dark',
        { attachTo: document.body },
    )

    const trigger = wrapper.get('[aria-haspopup="listbox"]')
    activate(trigger.element as HTMLElement)
    await waitFor(() => !!document.querySelector('[role="listbox"]'), {
        label: 'forced-up listbox to open',
    })

    const dropdown = document.querySelector('[role="listbox"]') as HTMLElement
    expect(dropdown.getBoundingClientRect().bottom).toBeLessThanOrEqual(
        (trigger.element as HTMLElement).getBoundingClientRect().top,
    )

    wrapper.unmount()
    teleports.remove()
})

it('automatically floats upward when the trigger is near the viewport bottom', async () => {
    const teleports = installTeleportTarget()
    const host = document.createElement('div')
    host.style.position = 'fixed'
    host.style.left = '16px'
    host.style.bottom = '4px'
    host.style.width = '180px'
    document.body.append(host)
    const options: ComboboxOption<string>[] = Array.from({ length: 8 }, (_, index) => ({
        value: `option-${index}`,
        label: `Option ${index}`,
    }))
    const wrapper = await mountThemed(Combobox, { modelValue: 'option-0', options }, 'dark', {
        attachTo: host,
    })

    const trigger = wrapper.get('[aria-haspopup="listbox"]')
    activate(trigger.element as HTMLElement)
    await waitFor(
        () => {
            const dropdown = document.querySelector('[role="listbox"]') as HTMLElement | null
            return !!dropdown && dropdown.getBoundingClientRect().height > 0
        },
        { label: 'auto-positioned listbox to open' },
    )

    const dropdown = document.querySelector('[role="listbox"]') as HTMLElement
    expect(dropdown.getBoundingClientRect().bottom).toBeLessThanOrEqual(
        (trigger.element as HTMLElement).getBoundingClientRect().top,
    )

    wrapper.unmount()
    host.remove()
    teleports.remove()
})
