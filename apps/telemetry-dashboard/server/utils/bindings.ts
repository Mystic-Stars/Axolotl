export interface DashboardBindings {
    DB?: D1Database
    ERROR_CONTEXTS?: R2Bucket
    CLOUDFLARE_ACCOUNT_ID?: string
    CLOUDFLARE_ANALYTICS_TOKEN?: string
}

export function dashboardBindings(event: { context: Record<string, unknown> }): DashboardBindings {
    const cloudflare = event.context.cloudflare as { env?: DashboardBindings } | undefined
    return cloudflare?.env ?? (event.context.env as DashboardBindings | undefined) ?? {}
}
