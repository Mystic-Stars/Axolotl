# Axolotl telemetry worker

This Worker accepts only opted-in launcher heartbeat batches. It stores anonymous usage metadata in D1; error reports and error context are not accepted or persisted.

## Provisioning

```sh
pnpm exec wrangler d1 create axolotl-telemetry
pnpm exec wrangler secret put INSTALLATION_HMAC_SECRET
pnpm exec wrangler d1 migrations apply axolotl-telemetry --remote
```

Replace `database_id` in `wrangler.toml` and keep the Worker on the Free plan. The production custom domain is declared in `wrangler.toml`. Configure Cloudflare usage notifications at 50%, 75%, and 90% for Workers and D1. Do not enable Workers Paid.

Clients that still have queued error reports receive `400 error_reporting_disabled`; the payload is not logged or stored. Upgrade clients should discard their local error outbox.

## Storage design

D1 stays within the Free plan's daily limits (5M rows read / 100k rows written) by avoiding per-event aggregation triggers:

- `daily_active` inserts feed `wau_seen` / `mau_seen` / `daily_active_dims` through a single lightweight trigger for O(1) WAU/MAU and distribution reads.
- A nightly cron (`runMaintenance`) refreshes usage totals, rolls the active windows, and applies usage-data retention. Legacy error tables remain in the schema for migration compatibility but are no longer written.

Schema changes are forward-only migrations in `migrations/`; never edit an applied migration.
