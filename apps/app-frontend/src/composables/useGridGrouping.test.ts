import assert from 'node:assert/strict'
import test from 'node:test'

import { effectScope, ref } from 'vue'

import { UNGROUPED_GROUP_KEY, useGridGrouping } from './useGridGrouping.ts'

test('renders an instance in every assigned group including favorites', () => {
    const descriptor = Object.getOwnPropertyDescriptor(globalThis, 'localStorage')
    Object.defineProperty(globalThis, 'localStorage', {
        configurable: true,
        value: { getItem: () => null, setItem: () => {}, removeItem: () => {} },
    })
    const scope = effectScope()
    try {
        scope.run(() => {
            const shared = {
                id: 'shared',
                name: 'Shared',
                groups: ['a', 'group:favorites', 'b', 'a'],
            }
            const ungrouped = { id: 'ungrouped', name: 'Ungrouped', groups: [] }
            const instances = ref([shared, ungrouped])
            const { filteredResults } = useGridGrouping('membership-test', instances)
            for (const id of ['a', 'b', 'group:favorites']) {
                assert.deepEqual(
                    filteredResults.value.get(id)?.map((instance) => instance.id),
                    ['shared'],
                )
            }
            assert.deepEqual(
                filteredResults.value.get(UNGROUPED_GROUP_KEY)?.map((instance) => instance.id),
                ['ungrouped'],
            )
            instances.value[0].groups = ['b', 'group:favorites']
            assert.equal(filteredResults.value.has('a'), false)
            assert.equal(filteredResults.value.get('b')?.length, 1)
            assert.equal(filteredResults.value.get('group:favorites')?.length, 1)
        })
    } finally {
        scope.stop()
        if (descriptor) Object.defineProperty(globalThis, 'localStorage', descriptor)
        else delete (globalThis as { localStorage?: Storage }).localStorage
    }
})
