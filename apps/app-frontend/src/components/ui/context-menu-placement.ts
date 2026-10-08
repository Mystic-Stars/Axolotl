export interface ContextMenuBoundary {
    left: number
    top: number
    right: number
    bottom: number
}

export interface ContextMenuPlacementInput {
    clientX: number
    clientY: number
    menuWidth: number
    menuHeight: number
    boundary: ContextMenuBoundary
    safeGap: number
}

export interface ContextMenuPlacement {
    left: number
    top: number
}

const fits = (
    left: number,
    top: number,
    menuWidth: number,
    menuHeight: number,
    boundary: ContextMenuBoundary,
    safeGap: number,
) =>
    left >= boundary.left + safeGap &&
    top >= boundary.top + safeGap &&
    left + menuWidth <= boundary.right - safeGap &&
    top + menuHeight <= boundary.bottom - safeGap

export function placeContextMenu({
    clientX,
    clientY,
    menuWidth,
    menuHeight,
    boundary,
    safeGap,
}: ContextMenuPlacementInput): ContextMenuPlacement {
    const candidates = [
        { left: clientX + safeGap, top: clientY + safeGap },
        { left: clientX - menuWidth - safeGap, top: clientY + safeGap },
        { left: clientX + safeGap, top: clientY - menuHeight - safeGap },
        { left: clientX - menuWidth - safeGap, top: clientY - menuHeight - safeGap },
    ]

    const fitting = candidates.find(({ left, top }) =>
        fits(left, top, menuWidth, menuHeight, boundary, safeGap),
    )
    if (fitting) return fitting

    const minLeft = boundary.left + safeGap
    const minTop = boundary.top + safeGap
    const maxLeft = Math.max(minLeft, boundary.right - menuWidth - safeGap)
    const maxTop = Math.max(minTop, boundary.bottom - menuHeight - safeGap)

    return {
        left: Math.min(maxLeft, Math.max(minLeft, candidates[0].left)),
        top: Math.min(maxTop, Math.max(minTop, candidates[0].top)),
    }
}
