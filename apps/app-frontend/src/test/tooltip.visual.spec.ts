import { tooltipDirective } from '@modrinth/ui/directives/tooltip.ts'
import { expect, it } from 'vitest'
import { createApp, defineComponent, h, nextTick, ref, withDirectives } from 'vue'

import { applyTheme, assertTokensLoaded, waitFor } from './visual-harness'

/**
 * The tooltip directive's behaviour, pinned.
 *
 * Written before migrating the component-level overlays, because those are the
 * changes a diff cannot check: a tooltip that renders in the wrong place still
 * type-checks and still builds. The `position`/`z-index` omissions this file
 * guards against were exactly that -- they shipped, and `vite build` passed.
 *
 * The directive is mounted on a real app instance, the same way `main.js` and
 * the website plugin register it, rather than through a component wrapper.
 */

const TRIGGER_STYLE = 'position:absolute;top:200px;left:200px;width:80px;height:32px'

function mountTrigger(
	value: ReturnType<typeof ref<unknown>>,
	modifiers: Record<string, boolean> = {},
): { trigger: HTMLElement; unmount: () => void } {
	const host = document.createElement('div')
	document.body.appendChild(host)

	const app = createApp(
		defineComponent({
			setup: () => () =>
				withDirectives(h('button', { id: 'tooltip-trigger', style: TRIGGER_STYLE }, 'Trigger'), [
					[tooltipDirective, value.value, undefined, modifiers],
				]),
		}),
	)
	app.mount(host)

	return {
		trigger: host.querySelector('button') as HTMLElement,
		unmount: () => {
			app.unmount()
			host.remove()
		},
	}
}

const popper = () => document.querySelector('.tooltip-popper') as HTMLElement | null

it('loads the token layer', () => {
	assertTokensLoaded()
})

it('does not render a popper for a suppressed value', async () => {
	const value = ref<unknown>(undefined)
	const { trigger, unmount } = mountTrigger(value)
	applyTheme('dark')
	await nextTick()

	trigger.dispatchEvent(new MouseEvent('mouseenter'))
	// Give any incorrectly-scheduled show a chance to run.
	await new Promise((resolve) => setTimeout(resolve, 400))

	expect(popper()).toBeNull()
	unmount()
})

it('renders the popper with working position and stacking', async () => {
	// The two properties whose omission made every tooltip land in the wrong
	// place and paint under the app's own layers.
	const value = ref<unknown>('Hello')
	const { trigger, unmount } = mountTrigger(value)
	applyTheme('dark')
	await nextTick()

	trigger.dispatchEvent(new MouseEvent('mouseenter'))
	await waitFor(() => !!popper(), { label: 'the tooltip to appear' })

	const el = popper() as HTMLElement
	const style = getComputedStyle(el)
	expect(style.position, 'out of flow, or the transform is relative to the page').toBe('fixed')
	expect(Number(style.zIndex), 'above the app chrome').toBeGreaterThanOrEqual(1000)
	expect(el.textContent).toContain('Hello')

	unmount()
})

it('positions the popper adjacent to its trigger', async () => {
	const value = ref<unknown>('Adjacent')
	const { trigger, unmount } = mountTrigger(value)
	applyTheme('dark')
	await nextTick()

	trigger.dispatchEvent(new MouseEvent('mouseenter'))
	await waitFor(() => !!popper(), { label: 'the tooltip' })
	// The transform is applied asynchronously after the position is computed.
	await waitFor(() => (popper() as HTMLElement).style.transform !== '', {
		label: 'positioning to settle',
	})

	const triggerBox = trigger.getBoundingClientRect()
	const popperBox = (popper() as HTMLElement).getBoundingClientRect()

	// The default placement is `top`, so the popper must sit above the trigger
	// and overlap it horizontally. A popper left in normal flow would land at
	// the document's bottom instead, which is the failure this catches.
	expect(popperBox.bottom).toBeLessThanOrEqual(triggerBox.top + 1)
	expect(popperBox.top).toBeGreaterThan(0)
	expect(popperBox.right).toBeGreaterThan(triggerBox.left)
	expect(popperBox.left).toBeLessThan(triggerBox.right)

	unmount()
})

it('honours a placement modifier', async () => {
	const value = ref<unknown>('To the right')
	const { trigger, unmount } = mountTrigger(value, { right: true })
	applyTheme('dark')
	await nextTick()

	trigger.dispatchEvent(new MouseEvent('mouseenter'))
	await waitFor(() => !!popper(), { label: 'the tooltip' })
	await waitFor(() => (popper() as HTMLElement).style.transform !== '', {
		label: 'positioning to settle',
	})

	const triggerBox = trigger.getBoundingClientRect()
	const popperBox = (popper() as HTMLElement).getBoundingClientRect()
	expect(
		popperBox.left,
		'a `right` tooltip sits to the right of its trigger',
	).toBeGreaterThanOrEqual(triggerBox.right - 1)

	unmount()
})

it('announces the tooltip only while it is shown', async () => {
	const value = ref<unknown>('Described')
	const { trigger, unmount } = mountTrigger(value)
	applyTheme('dark')
	await nextTick()

	expect(trigger.hasAttribute('aria-describedby')).toBe(false)

	trigger.dispatchEvent(new MouseEvent('mouseenter'))
	await waitFor(() => !!popper(), { label: 'the tooltip' })

	const describedBy = trigger.getAttribute('aria-describedby')
	expect(describedBy, 'a shown tooltip should describe its trigger').toBeTruthy()
	expect(document.getElementById(describedBy as string), 'the id must resolve').toBeTruthy()

	trigger.dispatchEvent(new MouseEvent('mouseleave'))
	await waitFor(() => !popper(), { label: 'the tooltip to hide' })
	expect(trigger.hasAttribute('aria-describedby')).toBe(false)

	unmount()
})

it('renders text as text and markup only for the html option', async () => {
	const value = ref<unknown>('<b>Bold</b>')
	const { trigger, unmount } = mountTrigger(value)
	applyTheme('dark')
	await nextTick()

	trigger.dispatchEvent(new MouseEvent('mouseenter'))
	await waitFor(() => !!popper(), { label: 'the text tooltip' })

	// Without `html`, markup must stay inert: this is the property that keeps an
	// untrusted string from becoming HTML.
	expect((popper() as HTMLElement).querySelector('b')).toBeNull()
	expect((popper() as HTMLElement).textContent).toContain('<b>Bold</b>')

	trigger.dispatchEvent(new MouseEvent('mouseleave'))
	await waitFor(() => !popper(), { label: 'the tooltip to hide' })

	value.value = { content: '<b>Bold</b>', html: true }
	await nextTick()
	trigger.dispatchEvent(new MouseEvent('mouseenter'))
	await waitFor(() => !!popper(), { label: 'the html tooltip' })
	expect((popper() as HTMLElement).querySelector('b'), 'html: true renders markup').toBeTruthy()

	unmount()
})

it('applies a popperClass to the popper', async () => {
	const value = ref<unknown>({ content: 'Styled', popperClass: 'storage-tooltip' })
	const { trigger, unmount } = mountTrigger(value)
	applyTheme('dark')
	await nextTick()

	trigger.dispatchEvent(new MouseEvent('mouseenter'))
	await waitFor(() => !!popper(), { label: 'the tooltip' })

	// Several components restyle the popper through this class; losing it would
	// silently change their appearance.
	expect((popper() as HTMLElement).classList.contains('storage-tooltip')).toBe(true)

	unmount()
})

it('re-reads its content when the value changes', async () => {
	// The value is deliberately not captured from the binding object passed to
	// `mounted`: Vue replaces that object on re-render, so a captured one would
	// keep serving the first render's text.
	const value = ref<unknown>('first')
	const { trigger, unmount } = mountTrigger(value)
	applyTheme('dark')
	await nextTick()

	trigger.dispatchEvent(new MouseEvent('mouseenter'))
	await waitFor(() => !!popper(), { label: 'the tooltip' })
	expect((popper() as HTMLElement).textContent).toContain('first')

	value.value = 'second'
	await nextTick()
	trigger.dispatchEvent(new MouseEvent('mouseleave'))
	await waitFor(() => !popper(), { label: 'the tooltip to hide' })

	trigger.dispatchEvent(new MouseEvent('mouseenter'))
	await waitFor(() => (popper() as HTMLElement | null)?.textContent?.includes('second') ?? false, {
		label: 'the updated content',
	})

	unmount()
})

it('removes its popper when the trigger unmounts', async () => {
	const value = ref<unknown>('Transient')
	const { trigger, unmount } = mountTrigger(value)
	applyTheme('dark')
	await nextTick()

	trigger.dispatchEvent(new MouseEvent('mouseenter'))
	await waitFor(() => !!popper(), { label: 'the tooltip' })

	// The popper lives under the teleport target, not inside the trigger's
	// subtree, so nothing would clean it up implicitly.
	unmount()
	await waitFor(() => !popper(), { label: 'the tooltip to be cleaned up' })
})
