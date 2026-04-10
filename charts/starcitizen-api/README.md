# starcitizen-api

A Helm chart for the [Star Citizen API](https://github.com/StarCitizenWiki/API) - a
Laravel-based API providing access to Star Citizen game data (items, vehicles,
manufacturers, starmap, comm-links, galactapedia) with multi-language support.

## Architecture

The chart deploys the full multi-role application, matching the Docker Compose
topology used by the upstream project:

| Component | Kind | Description |
|-----------|------|-------------|
| `api` | Deployment | Serves HTTP on port 8080 via Apache + PHP-FPM. Uses Laravel's `/up` healthcheck. |
| `scheduler` | Deployment (1 replica) | Runs `php artisan schedule:work` and drives all of Laravel's periodic jobs (ship matrix, starmap, galactapedia, stats, comm-link sync, …). |
| `queue-default` | Deployment | Runs `php artisan queue:work` on the `default` queue. |
| `queue-expensive` | Deployment | Runs a dedicated queue worker for the `expensive` queue (image hashing, similarity). |
| `install` | Job (Helm hook) | Runs `php artisan migrate --force` on install / upgrade. |
| `data-refresh` | CronJob | Periodically `git pull`s the upstream data repositories and re-runs `php artisan game:sync`. |

All application workloads share a single `PersistentVolumeClaim` mounted at
`/var/www/html/storage`. An init container seeds the required Laravel
directories and clones the upstream data repositories on first boot, so every
pod sees the same logs, cache and game data files.

## Automatic data updates

The API has two kinds of "automatic updates":

1. **Online data sources** (ship matrix, MSRP, starmap, galactapedia, stats,
   comm-links) are refreshed by Laravel's scheduler — handled by the
   dedicated `scheduler` Deployment.
2. **Upstream data repositories** (`scunpacked-data`, `StarCitizenDeutsch`,
   `ScToolBoxLocales`) are checked out into the shared PVC by the init
   container on pod start, and re-pulled by the `data-refresh` CronJob
   (default: weekly on Monday at 04:00). The CronJob then runs
   `php artisan game:sync` to import the refreshed game data.

See the `dataSources` section of `values.yaml` to customise both behaviours.

## Installing

```bash
helm install starcitizen ./charts/starcitizen-api
```

This installs the chart using the bundled CloudPirates PostgreSQL sub-chart,
auto-generates a Laravel `APP_KEY`, creates a shared PVC, and runs the
install job to apply migrations.

## Database

### Bundled PostgreSQL (default)

The chart depends on the [CloudPirates `postgres`](https://github.com/CloudPirates-io/helm-charts)
sub-chart. It is enabled by default and connects the application via the
secret the sub-chart generates.

```yaml
postgres:
  enabled: true
  auth:
    database: starcitizen
    username: starcitizen
    password: ""        # optional; sub-chart generates one if empty
  persistence:
    enabled: true
    size: 20Gi
```

### External database

To connect to an existing PostgreSQL instance instead:

```yaml
postgres:
  enabled: false

externalDatabase:
  host: postgres.example.com
  port: 5432
  database: starcitizen
  username: starcitizen
  # Either set the password directly:
  password: "supersecret"
  # Or reference an existing secret:
  existingSecret:
    name: my-postgres-credentials
    key: password
```

## Environment variables

Every variable documented in the upstream [README "Environment Configuration"
section](../../readme.md#environment-configuration) is exposed in an idiomatic
Kubernetes/Helm way:

- **Non-sensitive values** (`APP_NAME`, `APP_URL`, `APP_LOCALE`, `LOG_LEVEL`,
  `FILESYSTEM_DISK`, `QUEUE_CONNECTION`, `SESSION_DRIVER`, ...) are rendered
  into a `ConfigMap` and consumed via `envFrom`.
- **Sensitive values** (`APP_KEY`, `DB_PASSWORD`, `DEEPL_AUTH_KEY`,
  `MAIL_PASSWORD`, `AWS_SECRET_ACCESS_KEY`) are rendered into a `Secret` and
  consumed via `envFrom` + explicit `secretKeyRef` entries. You can also
  reference external secrets via the `existingSecret` blocks.
- The chart auto-generates a Laravel `APP_KEY` on first install using Helm's
  `lookup` function so the key is preserved across upgrades.
- `extraEnv` / `extraEnvFrom` are provided as escape hatches.

Values are grouped by domain to mirror the README:

```yaml
app:           # APP_NAME, APP_ENV, APP_URL, APP_DEBUG, APP_LOCALE, ...
logging:       # LOG_CHANNEL, LOG_LEVEL, ...
session:       # SESSION_DRIVER, SESSION_LIFETIME, ...
cache:         # CACHE_STORE
queue:         # QUEUE_CONNECTION
auth:          # SANCTUM_STATEFUL_DOMAINS, FORTIFY_ALLOW_REGISTRATION
mail:          # MAIL_MAILER, MAIL_HOST, MAIL_PORT, ...
filesystem:    # FILESYSTEM_DISK, AWS_*
deepl:         # DEEPL_AUTH_KEY, DEEPL_TARGET_LOCALE, ...
```

## Routing

### Gateway API (HTTPRoute)

```yaml
gatewayApi:
  enabled: true
  apiVersion: gateway.networking.k8s.io/v1
  parentRefs:
    - name: external
      namespace: gateway-system
      sectionName: https
  hostnames:
    - api.star-citizen.wiki
```

Leave `rules` empty for a default "match `/` → api service" rule. Fill it in
to override path/method/header matching and add filters.

### Classic Ingress (fallback)

```yaml
ingress:
  enabled: true
  className: nginx
  hosts:
    - host: api.star-citizen.wiki
      paths:
        - path: /
          pathType: Prefix
  tls:
    - hosts:
        - api.star-citizen.wiki
      secretName: starcitizen-api-tls
```

## Shared storage

By default the chart creates a `ReadWriteMany` PVC so api, scheduler, queue
and install workloads can all read and write the same `storage/` directory.
Set `persistence.accessMode=ReadWriteOnce` only if every pod is scheduled to
the same node.

Use `persistence.existingClaim` to plug in a PVC you manage yourself.

## Uninstalling

```bash
helm uninstall starcitizen
```

The PVC is retained on uninstall. Delete it manually if you want to fully
reset the release:

```bash
kubectl delete pvc -l app.kubernetes.io/instance=starcitizen
```
