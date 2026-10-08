import { flushPromises, mount } from '@vue/test-utils'
import { expect, it, vi } from 'vitest'

import LabRecipeGenerator from './LabRecipeGenerator.vue'

vi.mock('@/components/lab/recipe-generator/InstanceExportModal.vue', () => ({
    default: { render: () => null },
}))
vi.mock('@/components/lab/recipe-generator/ItemPalette.vue', () => ({
    default: { render: () => null },
}))
vi.mock('@/components/lab/recipe-generator/RecipeGeneratorCopyrightModal.vue', () => ({
    default: { render: () => null },
}))
vi.mock('@/components/lab/recipe-generator/RecipeItemIcon.vue', () => ({
    default: { render: () => null },
}))
vi.mock('@/components/lab/recipe-generator/RecipeSlotGrid.vue', () => ({
    default: { render: () => null },
}))
vi.mock('@/components/lab/recipe-generator/TagPalette.vue', () => ({
    default: { render: () => null },
}))

vi.mock('@/lab/recipe-generator/resources', async (importOriginal) => ({
    ...(await importOriginal<typeof import('@/lab/recipe-generator/resources')>()),
    loadVersionResources: async (version: string) => ({
        version,
        items: [],
        itemsById: {},
        vanillaTags: {},
    }),
}))
vi.mock('@modrinth/ui', async () => {
    const { defineComponent, h, ref } = await import('vue')
    const control = defineComponent({
        setup:
            (_, { slots }) =>
            () =>
                h('div', slots.default?.()),
    })
    return {
        Button: defineComponent({
            setup:
                (_, { slots }) =>
                () =>
                    h('button', slots.default?.()),
        }),
        StyledInput: defineComponent({
            props: ['modelValue', 'size'],
            emits: ['update:modelValue'],
            setup:
                (props, { emit }) =>
                () =>
                    h('input', {
                        value: props.modelValue,
                        onInput: (event: Event) =>
                            emit('update:modelValue', (event.target as HTMLInputElement).value),
                    }),
        }),
        Checkbox: control,
        Combobox: control,
        Toggle: control,
        NewModal: { render: () => null },
        defineMessages: (messages: unknown) => messages,
        defineMessage: (message: unknown) => message,
        injectNotificationManager: () => ({ handleError: vi.fn(), addNotification: vi.fn() }),
        useVIntl: () => ({
            locale: ref('en-US'),
            formatMessage: (message: { defaultMessage: string }) => message.defaultMessage,
        }),
    }
})

it('row clone and delete operate on that row while another recipe is selected', async () => {
    const wrapper = mount(LabRecipeGenerator, { global: { directives: { tooltip: () => {} } } })
    try {
        await flushPromises()
        const group = () => wrapper.get('input[placeholder="Group"]')
        const rows = () => wrapper.findAll('.recipe-sidebar-row')
        await group().setValue('Recipe A')
        await wrapper.get('.recipe-sidebar-heading button').trigger('click')
        expect(rows()).toHaveLength(2)
        expect((group().element as HTMLInputElement).value).toBe('')
        await group().setValue('Recipe B')
        expect((group().element as HTMLInputElement).value).toBe('Recipe B')
        await rows()[0].get('.recipe-sidebar-select').trigger('click')
        expect((group().element as HTMLInputElement).value).toBe('Recipe A')
        await rows()[1].get('.recipe-row-actions button').trigger('click')
        expect(rows()).toHaveLength(3)
        expect((group().element as HTMLInputElement).value).toBe('Recipe B')
        await group().setValue('Independent copy')
        await rows()[0].get('.recipe-sidebar-select').trigger('click')
        await rows()[1].findAll('.recipe-row-actions button')[1].trigger('click')
        expect(rows()).toHaveLength(2)
        expect(rows()[0].classes()).toContain('active')
        expect((group().element as HTMLInputElement).value).toBe('Recipe A')
        await rows()[1].get('.recipe-sidebar-select').trigger('click')
        expect((group().element as HTMLInputElement).value).toBe('Independent copy')
        await rows()[1].findAll('.recipe-row-actions button')[1].trigger('click')
        expect((group().element as HTMLInputElement).value).toBe('Recipe A')
        await rows()[0].findAll('.recipe-row-actions button')[1].trigger('click')
        expect(rows()).toHaveLength(1)
        expect((group().element as HTMLInputElement).value).toBe('')
    } finally {
        wrapper.unmount()
    }
})
