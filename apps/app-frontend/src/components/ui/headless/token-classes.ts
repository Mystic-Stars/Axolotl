/**
 * Shared Tailwind token class maps for reka-ui primitives.
 * Only existing theme tokens are used (surface-*, text-[var(--color-text-primary)]|primary|secondary,
 * brand, button-bg). No second theme system.
 */
export const headlessTokenClasses = {
	dialogOverlay: 'fixed inset-0 z-50 bg-black/50',
	dialogContent:
		'fixed left-1/2 top-1/2 z-50 w-[min(28rem,calc(100vw-2rem))] -translate-x-1/2 -translate-y-1/2 rounded-[var(--radius-lg)] border border-surface-5 bg-surface-3 p-6 text-[var(--color-text-primary)] shadow-xl focus:outline-none',
	dialogTitle: 'm-0 text-lg font-bold text-[var(--color-text-primary)]',
	dialogDescription: 'm-0 mt-2 text-sm text-[var(--color-text-tertiary)]',
	tooltipContent:
		'z-50 max-w-xs rounded-[var(--radius-sm)] border border-surface-5 bg-surface-4 px-2 py-1 text-sm text-[var(--color-text-primary)] shadow-md',
	selectTrigger:
		'flex h-9 min-w-36 items-center justify-between gap-2 rounded-[var(--radius-md)] border border-surface-5 bg-surface-4 px-3 text-sm text-[var(--color-text-primary)] transition-colors hover:brightness-[var(--hover-brightness)] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-brand',
	selectContent:
		'z-50 min-w-[var(--reka-select-trigger-width)] overflow-hidden rounded-[var(--radius-md)] border border-surface-5 bg-surface-3 p-1 text-[var(--color-text-primary)] shadow-md',
	selectItem:
		'relative flex cursor-pointer select-none items-center rounded-[var(--radius-sm)] px-2 py-1.5 text-sm text-[var(--color-text-primary)] outline-none data-[highlighted]:bg-surface-4 data-[state=checked]:font-semibold',
	checkboxRoot:
		'flex size-5 shrink-0 items-center justify-center rounded-[var(--radius-xs)] border border-surface-5 bg-surface-4 text-[var(--color-accent-contrast)] transition-colors data-[state=checked]:border-brand data-[state=checked]:bg-brand focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-brand',
	buttonStandard:
		'inline-flex h-9 items-center justify-center gap-2 rounded-[var(--radius-md)] border-0 bg-surface-4 px-3 text-sm font-medium text-[var(--color-text-default)] transition-[opacity,filter,transform] hover:brightness-[var(--hover-brightness)] active:scale-[0.98] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-brand',
	buttonBrand:
		'inline-flex h-9 items-center justify-center gap-2 rounded-[var(--radius-md)] border-0 bg-brand px-3 text-sm font-semibold text-[var(--color-accent-contrast)] transition-[opacity,filter,transform] hover:brightness-[var(--hover-brightness)] active:scale-[0.98] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-brand',
} as const
