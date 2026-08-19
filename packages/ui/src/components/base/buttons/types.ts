export type ButtonType = 'base' | 'colored' | 'colored-text' | 'outlined' | 'quiet'

export type ButtonSize = 'xs' | 'sm' | 'md' | 'lg' | 'xl'

export type ButtonInteraction = 'surface' | 'filled' | 'none'

// TODO: Standardized color string enum props across @modrinth/ui
export type ButtonColor =
	| 'brand'
	| 'red'
	| 'orange'
	| 'green'
	| 'blue'
	| 'purple'
	| 'medal_promotion'

export type ButtonVisualProps = {
	size?: ButtonSize
	interaction?: ButtonInteraction
} & (
	| {
			type?: 'base'
			color?: never
	  }
	| {
			type: 'outlined'
			color?: ButtonColor
	  }
	| {
			type: 'colored'
			color?: ButtonColor
	  }
	| {
			type: 'colored-text'
			color?: ButtonColor
	  }
	| {
			type: 'quiet'
			color?: ButtonColor
	  }
)

export type ButtonNativeType = 'button' | 'submit' | 'reset'

export interface ButtonProps {
	type?: ButtonType
	color?: ButtonColor
	size?: ButtonSize
	interaction?: ButtonInteraction
	nativeType?: ButtonNativeType
	disabled?: boolean
	loading?: boolean
}

export interface ButtonElementHandle {
	element: HTMLElement | null
}
