export interface DropdownPlacementGeometry {
	viewportHeight: number
	controlTop: number
	controlBottom: number
	floatingActionBarClearance: number
	safeGap: number
	expectedMenuHeight: number
}

export interface DropdownPlacement {
	renderUp: boolean
	availableHeight: number
}

export function dropdownPlacement({
	viewportHeight,
	controlTop,
	controlBottom,
	floatingActionBarClearance,
	safeGap,
	expectedMenuHeight,
}: DropdownPlacementGeometry): DropdownPlacement {
	const availableBelow = Math.max(
		0,
		viewportHeight - controlBottom - floatingActionBarClearance - safeGap,
	)
	const availableAbove = Math.max(0, controlTop - safeGap)
	const renderUp = availableBelow < expectedMenuHeight && availableAbove > availableBelow
	return {
		renderUp,
		availableHeight: renderUp ? availableAbove : availableBelow,
	}
}

export function shouldRenderDropdownUp(geometry: DropdownPlacementGeometry): boolean {
	return dropdownPlacement(geometry).renderUp
}
