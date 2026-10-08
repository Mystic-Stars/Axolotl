import { inject, type InjectionKey } from 'vue'

export interface MinecraftLaunchErrorPayload {
    instance_id: string
    instance_name: string
}

export type MinecraftLaunchErrorHandler = (
    error: unknown,
    payload: MinecraftLaunchErrorPayload,
) => Promise<boolean>

export const minecraftLaunchErrorKey: InjectionKey<MinecraftLaunchErrorHandler> =
    Symbol('minecraft-launch-error')

export function useMinecraftLaunchError(): MinecraftLaunchErrorHandler {
    const handler = inject(minecraftLaunchErrorKey)
    return async (error, payload) => (await handler?.(error, payload)) ?? false
}
