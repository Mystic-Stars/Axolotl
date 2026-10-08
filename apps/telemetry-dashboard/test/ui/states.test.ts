import { readFileSync } from 'node:fs'

import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'

import AppState from '../../components/AppState.vue'
import TrendChart from '../../components/charts/TrendChart.vue'
import Skeleton from '../../components/ui/Skeleton.vue'

describe('dashboard states', () => {
    it.each([
        ['error', '遥测数据暂不可用'],
        ['unauthenticated', '需要登录'],
        ['forbidden', '无权访问'],
    ] as const)('renders the %s state', (kind, title) => {
        const wrapper = mount(AppState, { props: { kind } })
        expect(wrapper.attributes('data-state')).toBe(kind)
        expect(wrapper.text()).toContain(title)
    })

    it('renders stable loading and empty chart states', () => {
        expect(mount(Skeleton, { props: { class: 'h-14' } }).classes()).toContain('h-14')
        const chart = mount(TrendChart, {
            props: {
                data: [],
                series: [{ key: 'errorOccurrences', label: '错误次数', color: '#d85d4a' }],
            },
        })
        expect(chart.get('[data-state="empty"]').text()).toContain('暂无数据')
    })
})

describe('responsive theme contract', () => {
    it('keeps dark tokens and narrow layouts from creating page overflow', () => {
        const root = process.cwd()
        const css = readFileSync(`${root}/assets/styles/tailwind.css`, 'utf8')
        const layout = readFileSync(`${root}/layouts/default.vue`, 'utf8')
        expect(css).toContain('.dark')
        expect(css).toContain('overflow-x-hidden')
        expect(layout).toContain('min-w-0 md:pl-60')
        expect(layout).toContain('w-60 -translate-x-full')
        expect(layout).toContain('生产数据')
    })
})
