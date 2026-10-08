import { createRemoteJWKSet, jwtVerify, type JWTVerifyGetKey } from 'jose'

import type { AdminSessionDto } from '../../shared/types/telemetry'
import { forbidden, unauthorized } from './errors'

export interface AccessSettings {
    teamDomain: string
    audience: string
}

type VerifyKey = JWTVerifyGetKey | CryptoKey | Uint8Array

export function accessSettings(config: ReturnType<typeof useRuntimeConfig>): AccessSettings | null {
    const teamDomain = String(
        process.env.CF_ACCESS_TEAM_DOMAIN || config.accessTeamDomain || '',
    ).trim()
    const audience = String(process.env.CF_ACCESS_AUDIENCE || config.accessAudience || '').trim()
    if (!teamDomain || !audience) return null
    return { teamDomain, audience }
}

export function mockAuthEnabled(config: ReturnType<typeof useRuntimeConfig>): boolean {
    if (process.env.NODE_ENV !== 'development') return false
    return String(process.env.TELEMETRY_ADMIN_MOCK_AUTH || config.mockAuth) === 'true'
}

export function mockScenario(config: ReturnType<typeof useRuntimeConfig>): string {
    return String(process.env.TELEMETRY_ADMIN_MOCK_SCENARIO || config.mockScenario || 'normal')
}

export async function verifyAccessJwt(
    token: string,
    settings: AccessSettings,
    key?: VerifyKey,
): Promise<AdminSessionDto> {
    const issuer = `https://${settings.teamDomain}.cloudflareaccess.com`
    const verifyKey = key ?? createRemoteJWKSet(new URL(`${issuer}/cdn-cgi/access/certs`))
    try {
        const { payload } = await jwtVerify(token, verifyKey, {
            issuer,
            audience: settings.audience,
            algorithms: ['RS256'],
        })
        const email = typeof payload.email === 'string' ? payload.email : null
        const nameClaim = payload.name ?? payload.common_name ?? email
        const name = typeof nameClaim === 'string' && nameClaim.trim() ? nameClaim : 'GitHub member'
        return {
            identity: { name, email },
            organization: 'Axolotl-Launcher',
            logoutUrl: '/cdn-cgi/access/logout',
            mock: false,
            dataSource: 'production',
        }
    } catch {
        throw unauthorized()
    }
}

export async function authenticateAccessToken(
    token: string | null | undefined,
    settings: AccessSettings,
    key?: VerifyKey,
): Promise<AdminSessionDto> {
    if (!token) throw unauthorized()
    return verifyAccessJwt(token, settings, key)
}

export function mockSession(scenario: string): AdminSessionDto {
    if (scenario === 'forbidden') throw forbidden()
    if (scenario === 'unconfigured-auth') throw unauthorized()
    return {
        identity: { name: '本地开发身份', email: null },
        organization: 'Axolotl-Launcher',
        logoutUrl: '/',
        mock: true,
        dataSource: 'fixture',
    }
}
