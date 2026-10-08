import { invoke } from '@tauri-apps/api/core'

import { check_mojang_services, mojang_auth_use_mirror } from '@/helpers/auth.js'
import { ensureFallenAuthProxyArgs, removeFallenAuthProxyArgs } from '@/helpers/java-arguments'
import { type AppSettings, get, update } from '@/helpers/settings'

const DEFAULT_RETRIES = 3
const RETRY_DELAY_MS = 1000
const REQUIRED_MOJANG_SERVICES = new Set(['account', 'session', 'services', 'profiles'])

export async function checkMojangAuthServers(retries = DEFAULT_RETRIES): Promise<boolean> {
    for (let attempt = 0; attempt < retries; attempt++) {
        try {
            const statuses = await check_mojang_services()
            const requiredStatuses = statuses.filter((status) =>
                REQUIRED_MOJANG_SERVICES.has(status.service),
            )
            if (
                requiredStatuses.length > 0 &&
                requiredStatuses.every((status) => status.reachable)
            ) {
                return true
            }
        } catch {
            if (attempt < retries - 1) {
                await new Promise((resolve) => setTimeout(resolve, RETRY_DELAY_MS))
            }
        }
    }
    return false
}

function sameArgs(left: string[], right: string[]) {
    return left.length === right.length && left.every((arg, index) => arg === right[index])
}

export async function setMojangAuthUseMirror(useMirror: boolean, automatic: boolean) {
    await invoke('plugin:auth|set_mojang_auth_use_mirror', { useMirror, automatic })
}

export async function reconcileMojangAuthSource(settings: AppSettings): Promise<boolean> {
    const mode = settings.mojang_auth_source ?? 'auto'
    const automatic = mode === 'auto' || mode === 'official_preferred'
    let useMirror: boolean
    if (mode === 'mirror_preferred') {
        useMirror = true
    } else if (mode === 'official_only') {
        useMirror = false
    } else {
        // Automatic and official-preferred check the services used by current
        // Minecraft versions, falling back only when one of them is down.
        useMirror = !(await checkMojangAuthServers())
    }

    await setMojangAuthUseMirror(useMirror, automatic)

    const nextArgs = useMirror
        ? ensureFallenAuthProxyArgs(settings.extra_launch_args)
        : removeFallenAuthProxyArgs(settings.extra_launch_args)

    if (sameArgs(nextArgs, settings.extra_launch_args)) return false

    settings.extra_launch_args = nextArgs
    return true
}

export async function reconcileMojangAuthSourceAtStartup(): Promise<void> {
    const settings = await get()
    if (await reconcileMojangAuthSource(settings)) {
        await update({ extra_launch_args: settings.extra_launch_args })
    }
}

/**
 * Reconsiders an automatic Mojang auth source while the app is running, so a
 * session that fell back to the mirror can go back to Mojang once it answers
 * again. Only the automatic modes are reconsidered; an explicit choice is
 * never overridden.
 */
export async function reconcileMojangAuthSourceIfMirrored(): Promise<void> {
    if (!(await mojang_auth_use_mirror())) return
    const settings = await get()
    const mode = settings.mojang_auth_source ?? 'auto'
    if (mode !== 'auto' && mode !== 'official_preferred') return
    if (await reconcileMojangAuthSource(settings)) {
        await update({ extra_launch_args: settings.extra_launch_args })
    }
}
