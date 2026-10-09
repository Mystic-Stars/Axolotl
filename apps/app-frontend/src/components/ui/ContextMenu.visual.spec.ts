import { afterEach, expect, it, vi } from 'vitest'
import { h, nextTick } from 'vue'

import { mountThemed, waitFor } from '@/test/visual-harness'

import ContextMenu from './ContextMenu.vue'

const cleanup: (() => void)[] = []
afterEach(() =>
    cleanup
        .splice(0)
        .reverse()
        .forEach((fn) => fn()),
)

async function menu(item: unknown = null) {
    const action = vi.fn()
    const wrapper = await mountThemed(ContextMenu, {}, 'dark', {
        slots: { inspect: () => h('span', 'Nested label') },
    })
    cleanup.push(() => wrapper.unmount())
    const vm = wrapper.vm as unknown as {
        showMenu: (event: MouseEvent, item: unknown, options: unknown[]) => void
        close: () => void
    }
    vm.showMenu(new MouseEvent('contextmenu', { clientX: 40, clientY: 40 }), item, [
        { name: 'inspect', label: 'Inspect' },
        { type: 'divider' },
        { name: 'disabled', label: 'Disabled', disabled: true, action },
        { name: 'action', label: 'Action', action },
        { name: 'add_content', label: 'Add content' },
    ])
    await nextTick()
    return { wrapper, vm, action }
}

it('keeps padding, separators and disabled items inert while actions target the correct item', async () => {
    const item = { id: 'instance-a' }
    const { wrapper, vm, action } = await menu(item)
    const root = wrapper.get('[role="menu"]')
    root.element.dispatchEvent(new MouseEvent('click', { bubbles: true }))
    await wrapper.get('[role="separator"]').trigger('click')
    wrapper
        .findAll('[role="menuitem"]')[1]
        .element.dispatchEvent(new MouseEvent('click', { bubbles: true }))
    expect(root.isVisible()).toBe(true)
    expect(action).not.toHaveBeenCalled()
    expect(wrapper.emitted('option-clicked')).toBeUndefined()
    expect(wrapper.emitted('menu-closed')).toBeUndefined()
    await wrapper.get('[role="menuitem"] span').trigger('click')
    expect(wrapper.emitted('option-clicked')?.[0]).toEqual([{ item, option: 'inspect' }])
    expect(wrapper.emitted('menu-closed')).toHaveLength(1)
    vm.close()
    document.body.click()
    expect(wrapper.emitted('menu-closed')).toHaveLength(1)
})

it('closes on outside click and Escape, but respects consumed Escape', async () => {
    const { wrapper } = await menu()
    const root = wrapper.get('[role="menu"]')
    const consumed = new KeyboardEvent('keydown', {
        key: 'Escape',
        bubbles: true,
        cancelable: true,
    })
    consumed.preventDefault()
    root.element.dispatchEvent(consumed)
    expect(root.isVisible()).toBe(true)
    document.body.click()
    await waitFor(() => !root.isVisible())
    expect(wrapper.emitted('menu-closed')).toHaveLength(1)
    const second = await menu()
    second.wrapper
        .get('[role="menu"]')
        .element.dispatchEvent(
            new KeyboardEvent('keydown', { key: 'Escape', bubbles: true, cancelable: true }),
        )
    await waitFor(() => !second.wrapper.get('[role="menu"]').isVisible())
    expect(second.wrapper.emitted('menu-closed')).toHaveLength(1)
})

it('closes other menus once and releases listeners when unmounted', async () => {
    const first = await menu()
    const second = await menu()
    expect(first.wrapper.emitted('menu-closed')).toHaveLength(1)
    expect(second.wrapper.get('[role="menu"]').isVisible()).toBe(true)
    second.wrapper.unmount()
    const before = second.wrapper.emitted('menu-closed')?.length ?? 0
    document.body.click()
    window.dispatchEvent(new CustomEvent('close-all-context-menus'))
    window.dispatchEvent(new Event('resize'))
    expect(second.wrapper.emitted('menu-closed')?.length ?? 0).toBe(before)
    expect(first.wrapper.emitted('menu-closed')).toHaveLength(1)
})

it('does not render a clickable placeholder for hidden linked-instance actions', async () => {
    const { wrapper, action } = await menu({ instance: { link: true } })
    expect(wrapper.findAll('[role="menuitem"]')).toHaveLength(3)
    await wrapper.findAll('[role="menuitem"]')[2].trigger('click')
    expect(action).toHaveBeenCalledOnce()
    expect(wrapper.emitted('option-clicked')).toBeUndefined()
})
