# Deployment runbook

Operators run this playbook to deploy the `mehen-core` service to the
`us-east-1` region. The runbook assumes the operator has push access to
[`ophidiarium/mehen`](../README.md) and AWS credentials configured for the
`mehen-deploy` IAM role (version 1.4.2 or later).

## Preflight

Check that the current commit builds cleanly:

```bash
cargo check --all-features --locked
cargo clippy --all-targets --all-features --locked -- -D warnings
```

Confirm that the last release passed CI:

```bash
gh run list --branch main --limit 5 --json status,conclusion
```

If CI shows red, stop here and open an issue linking the run id.

## Apply the migration

The schema migration lives in [`migrations/0018_docs_index.sql`](../README.md).
Apply it to the staging database first:

```sh
psql "$MEHEN_STAGING_URL" -f migrations/0018_docs_index.sql
```

Verify the new index:

```sql
SELECT indexname, tablename FROM pg_indexes WHERE tablename = 'docs';
```

You should see `docs_text_idx` in the result set. If the index is missing,
roll back with `migrations/0018_docs_index.down.sql` before proceeding.

## Promote the build

Trigger a new release build. The Docker image tag follows semver
(`v0.5.0`, `v0.5.1`, …):

```sh
./scripts/release.sh v0.5.0
```

The script runs `cargo build --release --locked`, tags the image with
`ghcr.io/ophidiarium/mehen:v0.5.0`, and pushes it to the registry. It also
updates the `deploy/staging/kustomization.yaml` manifest to pin the new
image digest.

## Verify rollout

After the staging deploy completes, confirm health with:

```bash
curl -sf https://staging.mehen.example.com/healthz
```

The endpoint should return `200 OK` with the JSON body:

```json
{"version":"0.5.0","git":"abcdef1","ready":true}
```

## Rollback

If the `/healthz` check fails within 10 minutes, run:

```bash
./scripts/rollback.sh v0.4.3
```

See also [`docs/mehen_markdown_metrics_research_foundation.md`](../docs/mehen_markdown_metrics_research_foundation.md)
for the metric thresholds the rollout dashboard uses.

## Configuration reference

The service reads configuration from `/etc/mehen/config.yaml`. The file
must define these keys:

```yaml
database_url: postgres://mehen@db.internal/mehen
redis_url: redis://cache.internal:6379/0
log_level: info
listen_port: 8080
metrics:
  enabled: true
  bind: 0.0.0.0:9090
features:
  experimental_diff: false
```

The `listen_port` setting defaults to `8080` when unset. Override it via
`MEHEN_LISTEN_PORT` for ephemeral staging runs. See also
[`config/defaults.yaml`](../README.md) for the baseline values.

Known API endpoints exposed by this service:

- `GET /healthz` — liveness probe (returns `200 OK`).
- `GET /readyz` — readiness probe (returns `200 OK` after warmup).
- `POST /v1/analyze` — primary document analysis API.
- `GET /metrics` — Prometheus scrape endpoint.

## Troubleshooting

If the deploy stalls, inspect the pod logs with `kubectl logs` and grep for
`ERROR`:

```sh
kubectl -n mehen logs deploy/mehen-core | grep -F ERROR | head -n 20
```

Common causes and remedies:

| Symptom | Likely cause | Fix |
|---------|--------------|-----|
| `OOMKilled` | Memory limit too low for v0.5.0 | Raise `resources.limits.memory` to `512Mi` |
| `connect ECONNREFUSED` | Pod can't reach Redis | Check `NetworkPolicy` |
| `timeout 5s` on `/v1/analyze` | Document too large | Increase `timeout_secs: 30` |
| `permission denied` on `/etc/mehen/config.yaml` | Wrong secret mount | Reapply `helm upgrade --install mehen-core` |

Tracking issue: https://github.com/ophidiarium/mehen/issues/12345
See also https://github.com/ophidiarium/mehen/issues/12346 for the related
follow-up.

## References

| Runbook | Purpose | Owner |
|---------|---------|-------|
| `deploy.md` | Primary deploy path | `@platform` |
| `rollback.md` | Rollback procedure | `@platform` |
| `incident.md` | Incident response | `@sre` |
| `config.md` | Config reference | `@platform` |
