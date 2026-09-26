export type ButtonType =
	'base' | 'colored' | 'colored-text' | 'outlined' | 'quiet' | 'chip' | 'chip-text' | 'highlight'

export type ButtonSize = '2xs' | 'xs' | 'sm' | 'md' | 'lg' | 'xl'

export type ButtonInteraction = 'surface' | 'filled' | 'none'

// TODO: Standardized color string enum props across @modrinth/ui
export type ButtonColor = 'brand' | 'red' | 'orange' | 'green' | 'blue' | 'purple' | 'medal-promo'

type ButtonVisualBase = {
	size?: ButtonSize
}

type ButtonVisualWithoutInteraction =
	| {
			type?: 'base'
			color?: never
			interaction?: never
	  }
	| {
			type: Exclude<ButtonType, 'base' | 'quiet'>
			color?: ButtonColor
			interaction?: never
	  }

/**
 * Public visual combinations supported by the current button frame.
 *
 * `interaction` only changes quiet buttons, so exposing it on any other type
 * would accept a prop that has no effect. Keeping that restriction here makes
 * Button and ButtonLink share one truthful visual contract.
 */
export type ButtonVisualProps = ButtonVisualBase &
	(
		| ButtonVisualWithoutInteraction
		| {
				type: 'quiet'
				color?: ButtonColor
				interaction?: ButtonInteraction
		  }
	)

export type ButtonNativeType = 'button' | 'submit' | 'reset'

/** Icon-only controls need a name because their slot has no visible label. */
export type ButtonContentProps =
	| {
			iconOnly: true
			label: string
	  }
	| {
			iconOnly?: false
			label?: never
	  }

export type ButtonProps = ButtonVisualProps &
	ButtonContentProps & {
		circular?: boolean
		nativeType?: ButtonNativeType
		disabled?: boolean
		loading?: boolean
	}

export type ButtonLinkProps = ButtonVisualProps &
	ButtonContentProps & {
		as?: string | import('vue').Component
		href?: string
		disabled?: boolean
		circular?: boolean
	}

export interface ButtonElementHandle {
	element: HTMLElement | null
}
