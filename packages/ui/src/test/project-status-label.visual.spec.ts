import type { ProjectStatus } from '@modrinth/utils'
import { describe, expect, it } from 'vitest'
import { ref } from 'vue'

import ProjectStatusBadge from '../components/project/ProjectStatusBadge.vue'
import { I18N_INJECTION_KEY } from '../providers/i18n'
import { mountThemed } from './visual-harness'

const STATUSES: Array<{ status: ProjectStatus; label: string }> = [
    { status: 'approved', label: 'Public' },
    { status: 'unlisted', label: 'Unlisted' },
    { status: 'withheld', label: 'Unlisted by staff' },
    { status: 'private', label: 'Private' },
    { status: 'scheduled', label: 'Scheduled' },
    { status: 'draft', label: 'Draft' },
    { status: 'archived', label: 'Archived' },
    { status: 'rejected', label: 'Rejected' },
    { status: 'processing', label: 'Under review' },
    { status: 'unknown', label: 'Unknown' },
]

async function mountStatus(status: ProjectStatus) {
    return mountThemed(ProjectStatusBadge, { status }, 'dark', {
        global: {
            provide: {
                [I18N_INJECTION_KEY as symbol]: {
                    locale: ref('en-US'),
                    t: (key: string) => key,
                    setLocale: () => undefined,
                },
            },
        },
    })
}

describe('ProjectStatusBadge', () => {
    it('renders every project status with its label and decorative icon', async () => {
        for (const { status, label } of STATUSES) {
            const wrapper = await mountStatus(status)
            const root = wrapper.get('span')
            const icon = root.get('svg')

            expect(root.text(), status).toBe(label)
            expect(icon.attributes('aria-hidden'), `${status} icon`).toBe('true')
            wrapper.unmount()
        }
    })

    it('uses unknown metadata when the API supplies an unrecognized status', async () => {
        const wrapper = await mountStatus('future-status' as ProjectStatus)

        expect(wrapper.get('span').text()).toBe('Unknown')
        expect(wrapper.get('svg').attributes('aria-hidden')).toBe('true')
    })

    it('preserves the uncontained status-label visual and semantic contract', async () => {
        const wrapper = await mountStatus('approved')
        const root = wrapper.get('span')
        const element = root.element as HTMLElement
        const style = getComputedStyle(element)

        expect(element.tagName).toBe('SPAN')
        expect(root.attributes('role')).toBeUndefined()
        expect(style.display).toBe('inline-flex')
        expect(style.alignItems).toBe('center')
        expect(style.gap).toBe('4px')
        expect(style.fontWeight).toBe('600')

        const tokenProbe = document.createElement('span')
        tokenProbe.style.color = 'var(--color-text-tertiary)'
        document.body.append(tokenProbe)
        expect(style.color).toBe(getComputedStyle(tokenProbe).color)
        tokenProbe.remove()

        expect(style.backgroundColor).toBe('rgba(0, 0, 0, 0)')
        expect(style.padding).toBe('0px')
        expect(style.borderTopWidth).toBe('0px')
    })
})
