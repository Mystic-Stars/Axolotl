import { generateKeyPair, SignJWT } from 'jose'
import { afterEach, describe, expect, it } from 'vitest'

import { authenticateAccessToken, mockAuthEnabled, verifyAccessJwt } from '../../server/utils/auth'

const settings = { teamDomain: 'fixture-team', audience: 'fixture-audience' }
const issuer = 'https://fixture-team.cloudflareaccess.com'

async function token(
    privateKey: CryptoKey,
    overrides: { audience?: string; expiresAt?: number } = {},
): Promise<string> {
    const now = Math.floor(Date.now() / 1_000)
    return new SignJWT({ email: 'fixture@example.invalid', name: 'Fixture Operator' })
        .setProtectedHeader({ alg: 'RS256' })
        .setIssuer(issuer)
        .setAudience(overrides.audience ?? settings.audience)
        .setIssuedAt(now)
        .setExpirationTime(overrides.expiresAt ?? now + 600)
        .sign(privateKey)
}

describe('Cloudflare Access authentication', () => {
    const previousNodeEnv = process.env.NODE_ENV

    afterEach(() => {
        process.env.NODE_ENV = previousNodeEnv
        delete process.env.TELEMETRY_ADMIN_MOCK_AUTH
    })

    it('rejects a missing Access assertion', async () => {
        await expect(authenticateAccessToken(null, settings)).rejects.toMatchObject({
            statusCode: 401,
            code: 'unauthenticated',
        })
    })

    it('rejects the wrong audience and expired tokens, then accepts a valid token', async () => {
        const { privateKey, publicKey } = (await generateKeyPair('RS256')) as CryptoKeyPair
        await expect(
            verifyAccessJwt(
                await token(privateKey, { audience: 'wrong-audience' }),
                settings,
                publicKey,
            ),
        ).rejects.toMatchObject({ statusCode: 401 })
        await expect(
            verifyAccessJwt(
                await token(privateKey, { expiresAt: Math.floor(Date.now() / 1_000) - 10 }),
                settings,
                publicKey,
            ),
        ).rejects.toMatchObject({ statusCode: 401 })

        const session = await verifyAccessJwt(await token(privateKey), settings, publicKey)
        expect(session).toMatchObject({
            identity: { name: 'Fixture Operator', email: 'fixture@example.invalid' },
            organization: 'Axolotl-Launcher',
            mock: false,
            dataSource: 'production',
        })
    })

    it('cannot enable MockAuthProvider in production', () => {
        process.env.NODE_ENV = 'production'
        process.env.TELEMETRY_ADMIN_MOCK_AUTH = 'true'
        expect(mockAuthEnabled({ mockAuth: true } as ReturnType<typeof useRuntimeConfig>)).toBe(
            false,
        )
    })
})
