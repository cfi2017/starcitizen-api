{{/*
Expand the name of the chart.
*/}}
{{- define "starcitizen-api.name" -}}
{{- default .Chart.Name .Values.nameOverride | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Create a default fully qualified app name.
*/}}
{{- define "starcitizen-api.fullname" -}}
{{- if .Values.fullnameOverride }}
{{- .Values.fullnameOverride | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- $name := default .Chart.Name .Values.nameOverride }}
{{- if contains $name .Release.Name }}
{{- .Release.Name | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- printf "%s-%s" .Release.Name $name | trunc 63 | trimSuffix "-" }}
{{- end }}
{{- end }}
{{- end }}

{{/*
Chart name and version as used by the chart label.
*/}}
{{- define "starcitizen-api.chart" -}}
{{- printf "%s-%s" .Chart.Name .Chart.Version | replace "+" "_" | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Common labels
*/}}
{{- define "starcitizen-api.labels" -}}
helm.sh/chart: {{ include "starcitizen-api.chart" . }}
{{ include "starcitizen-api.selectorLabels" . }}
{{- if .Chart.AppVersion }}
app.kubernetes.io/version: {{ .Chart.AppVersion | quote }}
{{- end }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
{{- with .Values.commonLabels }}
{{ toYaml . }}
{{- end }}
{{- end }}

{{/*
Selector labels
*/}}
{{- define "starcitizen-api.selectorLabels" -}}
app.kubernetes.io/name: {{ include "starcitizen-api.name" . }}
app.kubernetes.io/instance: {{ .Release.Name }}
{{- end }}

{{/*
Component-specific labels. Pass the component name as a string:
  {{ include "starcitizen-api.componentLabels" (dict "context" . "component" "api") }}
*/}}
{{- define "starcitizen-api.componentLabels" -}}
{{ include "starcitizen-api.labels" .context }}
app.kubernetes.io/component: {{ .component }}
{{- end }}

{{/*
Component-specific selector labels.
*/}}
{{- define "starcitizen-api.componentSelectorLabels" -}}
{{ include "starcitizen-api.selectorLabels" .context }}
app.kubernetes.io/component: {{ .component }}
{{- end }}

{{/*
Common annotations
*/}}
{{- define "starcitizen-api.commonAnnotations" -}}
{{- with .Values.commonAnnotations }}
{{- toYaml . }}
{{- end }}
{{- end }}

{{/*
Service account name
*/}}
{{- define "starcitizen-api.serviceAccountName" -}}
{{- if .Values.serviceAccount.create }}
{{- default (include "starcitizen-api.fullname" .) .Values.serviceAccount.name }}
{{- else }}
{{- default "default" .Values.serviceAccount.name }}
{{- end }}
{{- end }}

{{/*
Image reference. Honours image.digest when set.
*/}}
{{- define "starcitizen-api.image" -}}
{{- $registry := .Values.image.registry -}}
{{- $repo := .Values.image.repository -}}
{{- $tag := default .Chart.AppVersion .Values.image.tag -}}
{{- if .Values.image.digest -}}
{{- printf "%s/%s@%s" $registry $repo .Values.image.digest -}}
{{- else -}}
{{- printf "%s/%s:%s" $registry $repo $tag -}}
{{- end -}}
{{- end }}

{{/*
Rust API image reference.
*/}}
{{- define "starcitizen-api.rustApiImage" -}}
{{- $registry := .Values.rustApi.image.registry -}}
{{- $repo := .Values.rustApi.image.repository -}}
{{- $tag := default .Chart.AppVersion .Values.rustApi.image.tag -}}
{{- if .Values.rustApi.image.digest -}}
{{- printf "%s/%s@%s" $registry $repo .Values.rustApi.image.digest -}}
{{- else -}}
{{- printf "%s/%s:%s" $registry $repo $tag -}}
{{- end -}}
{{- end }}

{{/*
Name of the ConfigMap that carries the non-sensitive env vars.
*/}}
{{- define "starcitizen-api.configMapName" -}}
{{- printf "%s-env" (include "starcitizen-api.fullname" .) }}
{{- end }}

{{/*
Name of the Secret that carries the sensitive env vars managed by the chart.
*/}}
{{- define "starcitizen-api.secretName" -}}
{{- printf "%s-env" (include "starcitizen-api.fullname" .) }}
{{- end }}

{{/*
Shared storage PVC name
*/}}
{{- define "starcitizen-api.storageClaimName" -}}
{{- if .Values.persistence.existingClaim }}
{{- .Values.persistence.existingClaim }}
{{- else }}
{{- printf "%s-storage" (include "starcitizen-api.fullname" .) }}
{{- end }}
{{- end }}

{{/* =======================================================================
Database helpers. Resolve DB_HOST / DB_PORT / DB_DATABASE / DB_USERNAME /
DB_PASSWORD based on whether the bundled CloudPirates sub-chart or an
external database is used.
======================================================================= */}}

{{/*
Subchart postgres service name. Follows the CloudPirates fullname convention
which uses `cloudpirates.fullname`. Because we include the chart with
`alias: postgres`, the effective name is `<release>-postgres` when the
chart's own nameOverride / fullnameOverride are empty.
*/}}
{{- define "starcitizen-api.postgres.fullname" -}}
{{- if and .Values.postgres (index .Values.postgres "fullnameOverride") -}}
{{- .Values.postgres.fullnameOverride | trunc 63 | trimSuffix "-" -}}
{{- else -}}
{{- printf "%s-postgres" .Release.Name | trunc 63 | trimSuffix "-" -}}
{{- end -}}
{{- end }}

{{- define "starcitizen-api.db.host" -}}
{{- if .Values.postgres.enabled -}}
{{ include "starcitizen-api.postgres.fullname" . }}
{{- else -}}
{{- required "externalDatabase.host is required when postgres.enabled is false" .Values.externalDatabase.host -}}
{{- end -}}
{{- end }}

{{- define "starcitizen-api.db.port" -}}
{{- if .Values.postgres.enabled -}}
5432
{{- else -}}
{{ .Values.externalDatabase.port | default 5432 }}
{{- end -}}
{{- end }}

{{- define "starcitizen-api.db.name" -}}
{{- if .Values.postgres.enabled -}}
{{ .Values.postgres.auth.database }}
{{- else -}}
{{ .Values.externalDatabase.database }}
{{- end -}}
{{- end }}

{{- define "starcitizen-api.db.username" -}}
{{- if .Values.postgres.enabled -}}
{{ .Values.postgres.auth.username }}
{{- else -}}
{{ .Values.externalDatabase.username }}
{{- end -}}
{{- end }}

{{/*
Name of the secret that holds the DB password, and the key under which it is
stored. Used by workloads via `valueFrom.secretKeyRef`.
*/}}
{{- define "starcitizen-api.db.passwordSecretName" -}}
{{- if .Values.postgres.enabled -}}
{{ include "starcitizen-api.postgres.fullname" . }}
{{- else if .Values.externalDatabase.existingSecret.name -}}
{{ .Values.externalDatabase.existingSecret.name }}
{{- else -}}
{{ include "starcitizen-api.secretName" . }}
{{- end -}}
{{- end }}

{{- define "starcitizen-api.db.passwordSecretKey" -}}
{{- if .Values.postgres.enabled -}}
postgres-password
{{- else if .Values.externalDatabase.existingSecret.name -}}
{{ .Values.externalDatabase.existingSecret.key }}
{{- else -}}
DB_PASSWORD
{{- end -}}
{{- end }}

{{/*
Name and key for APP_KEY secret reference.
*/}}
{{- define "starcitizen-api.appKeySecretName" -}}
{{- if .Values.app.existingKeySecret.name -}}
{{ .Values.app.existingKeySecret.name }}
{{- else -}}
{{ include "starcitizen-api.secretName" . }}
{{- end -}}
{{- end }}

{{- define "starcitizen-api.appKeySecretKey" -}}
{{- if .Values.app.existingKeySecret.name -}}
{{ .Values.app.existingKeySecret.key }}
{{- else -}}
APP_KEY
{{- end -}}
{{- end }}

{{/*
Resolve / generate the APP_KEY value. On first install we generate a random
32-character key. On upgrades we `lookup` the existing managed secret and
re-use the stored value so the key never rotates unexpectedly.
*/}}
{{- define "starcitizen-api.appKeyValue" -}}
{{- if .Values.app.key -}}
{{ .Values.app.key }}
{{- else -}}
{{- $secretName := include "starcitizen-api.secretName" . -}}
{{- $existing := lookup "v1" "Secret" .Release.Namespace $secretName -}}
{{- if and $existing (index $existing.data "APP_KEY") -}}
{{ index $existing.data "APP_KEY" | b64dec }}
{{- else -}}
{{- printf "base64:%s" (randAlphaNum 32 | b64enc) -}}
{{- end -}}
{{- end -}}
{{- end }}

{{/*
Env vars shared by every workload. Rendered via `envFrom` + a handful of
overrides defined in `starcitizen-api.extraPodEnv`.

These are the non-sensitive fields that go into the ConfigMap.
*/}}
{{- define "starcitizen-api.configEnv" -}}
APP_NAME: {{ .Values.app.name | quote }}
APP_ENV: {{ .Values.app.env | quote }}
APP_DEBUG: {{ .Values.app.debug | quote }}
APP_URL: {{ .Values.app.url | quote }}
APP_LOCALE: {{ .Values.app.locale | quote }}
APP_FALLBACK_LOCALE: {{ .Values.app.fallbackLocale | quote }}
APP_FAKER_LOCALE: {{ .Values.app.fakerLocale | quote }}
APP_MAINTENANCE_DRIVER: {{ .Values.app.maintenanceDriver | quote }}
BCRYPT_ROUNDS: {{ .Values.app.bcryptRounds | quote }}
LOG_CHANNEL: {{ .Values.logging.channel | quote }}
LOG_STACK: {{ .Values.logging.stack | quote }}
LOG_DEPRECATIONS_CHANNEL: {{ .Values.logging.deprecationsChannel | quote }}
LOG_LEVEL: {{ .Values.logging.level | quote }}
SESSION_DRIVER: {{ .Values.session.driver | quote }}
SESSION_LIFETIME: {{ .Values.session.lifetime | quote }}
SESSION_ENCRYPT: {{ .Values.session.encrypt | quote }}
SESSION_PATH: {{ .Values.session.path | quote }}
SESSION_DOMAIN: {{ .Values.session.domain | quote }}
CACHE_STORE: {{ .Values.cache.store | quote }}
QUEUE_CONNECTION: {{ .Values.queue.connection | quote }}
FORTIFY_ALLOW_REGISTRATION: {{ .Values.auth.fortifyAllowRegistration | quote }}
SANCTUM_STATEFUL_DOMAINS: {{ .Values.auth.sanctumStatefulDomains | quote }}
MAIL_MAILER: {{ .Values.mail.mailer | quote }}
MAIL_HOST: {{ .Values.mail.host | quote }}
MAIL_PORT: {{ .Values.mail.port | quote }}
MAIL_USERNAME: {{ .Values.mail.username | quote }}
MAIL_FROM_ADDRESS: {{ .Values.mail.fromAddress | quote }}
MAIL_FROM_NAME: {{ .Values.mail.fromName | quote }}
FILESYSTEM_DISK: {{ .Values.filesystem.disk | quote }}
AWS_ACCESS_KEY_ID: {{ .Values.filesystem.s3.accessKeyId | quote }}
AWS_DEFAULT_REGION: {{ .Values.filesystem.s3.region | quote }}
AWS_BUCKET: {{ .Values.filesystem.s3.bucket | quote }}
AWS_USE_PATH_STYLE_ENDPOINT: {{ .Values.filesystem.s3.usePathStyleEndpoint | quote }}
DEEPL_TARGET_LOCALE: {{ .Values.deepl.targetLocale | quote }}
DEEPL_TRANSLATION_LOCALE: {{ .Values.deepl.translationLocale | quote }}
COMM_LINKS_AUTO_TRANSLATE_AFTER_IMPORT: {{ .Values.deepl.commLinksAutoTranslateAfterImport | quote }}
DB_CONNECTION: "pgsql"
DB_HOST: {{ include "starcitizen-api.db.host" . | quote }}
DB_PORT: {{ include "starcitizen-api.db.port" . | quote }}
DB_DATABASE: {{ include "starcitizen-api.db.name" . | quote }}
DB_USERNAME: {{ include "starcitizen-api.db.username" . | quote }}
{{- range $k, $v := .Values.extraEnv }}
{{ $k }}: {{ $v | quote }}
{{- end }}
{{- end }}

{{/*
Per-pod env overrides that can't live in a ConfigMap (valueFrom references).
*/}}
{{- define "starcitizen-api.secretEnv" -}}
- name: APP_KEY
  valueFrom:
    secretKeyRef:
      name: {{ include "starcitizen-api.appKeySecretName" . }}
      key: {{ include "starcitizen-api.appKeySecretKey" . }}
- name: DB_PASSWORD
  valueFrom:
    secretKeyRef:
      name: {{ include "starcitizen-api.db.passwordSecretName" . }}
      key: {{ include "starcitizen-api.db.passwordSecretKey" . }}
{{- if or .Values.deepl.authKey .Values.deepl.existingSecret.name }}
- name: DEEPL_AUTH_KEY
  valueFrom:
    secretKeyRef:
      {{- if .Values.deepl.existingSecret.name }}
      name: {{ .Values.deepl.existingSecret.name }}
      key: {{ .Values.deepl.existingSecret.key }}
      {{- else }}
      name: {{ include "starcitizen-api.secretName" . }}
      key: DEEPL_AUTH_KEY
      {{- end }}
{{- end }}
{{- if or .Values.mail.password .Values.mail.existingSecret.name }}
- name: MAIL_PASSWORD
  valueFrom:
    secretKeyRef:
      {{- if .Values.mail.existingSecret.name }}
      name: {{ .Values.mail.existingSecret.name }}
      key: {{ .Values.mail.existingSecret.key }}
      {{- else }}
      name: {{ include "starcitizen-api.secretName" . }}
      key: MAIL_PASSWORD
      {{- end }}
{{- end }}
{{- if or .Values.filesystem.s3.secretAccessKey .Values.filesystem.s3.existingSecret.name }}
- name: AWS_SECRET_ACCESS_KEY
  valueFrom:
    secretKeyRef:
      {{- if .Values.filesystem.s3.existingSecret.name }}
      name: {{ .Values.filesystem.s3.existingSecret.name }}
      key: {{ .Values.filesystem.s3.existingSecret.secretAccessKeyKey }}
      {{- else }}
      name: {{ include "starcitizen-api.secretName" . }}
      key: AWS_SECRET_ACCESS_KEY
      {{- end }}
{{- end }}
{{- end }}

{{/*
Volume mounts used by every workload.
*/}}
{{- define "starcitizen-api.volumeMounts" -}}
- name: storage
  mountPath: /var/www/html/storage
{{- end }}

{{/*
Volumes used by every workload.
*/}}
{{- define "starcitizen-api.volumes" -}}
- name: storage
  {{- if .Values.persistence.enabled }}
  persistentVolumeClaim:
    claimName: {{ include "starcitizen-api.storageClaimName" . }}
  {{- else }}
  emptyDir: {}
  {{- end }}
{{- end }}

{{/*
Init containers shared by every workload:
  1) init-storage: ensures Laravel's framework/ subdirectories exist on the PVC.
  2) init-submodules: clones the data-source git repositories if they are
     missing. A fast no-op on subsequent pod restarts once the repos exist.
*/}}
{{- define "starcitizen-api.initContainers" -}}
- name: init-storage
  image: {{ include "starcitizen-api.image" . }}
  imagePullPolicy: {{ .Values.image.pullPolicy }}
  command:
    - /bin/sh
    - -c
    - |
      set -e
      mkdir -p \
        /storage/app/public \
        /storage/app/api \
        /storage/framework/cache/data \
        /storage/framework/sessions \
        /storage/framework/views \
        /storage/framework/testing \
        /storage/logs
  volumeMounts:
    - name: storage
      mountPath: /storage
{{- if .Values.dataSources.init.enabled }}
- name: init-submodules
  image: "{{ .Values.dataSources.init.image.repository }}:{{ .Values.dataSources.init.image.tag }}"
  imagePullPolicy: {{ .Values.dataSources.init.image.pullPolicy }}
  command:
    - /bin/sh
    - -c
    - |
      set -e
      mkdir -p /storage/app/api
      cd /storage/app/api
      {{- range .Values.dataSources.repositories }}
      if [ ! -d "{{ .name }}/.git" ]; then
        echo ">> cloning {{ .name }}"
        rm -rf "{{ .name }}"
        git clone --depth=1 {{ if .branch }}--branch {{ .branch | quote }} {{ end }}{{ .url | quote }} "{{ .name }}"
      else
        echo ">> {{ .name }} already present, skipping"
      fi
      {{- end }}
  {{- with .Values.dataSources.init.resources }}
  resources:
    {{- toYaml . | nindent 4 }}
  {{- end }}
  volumeMounts:
    - name: storage
      mountPath: /storage
{{- end }}
{{- end }}

{{/*
`envFrom` block that wires both the ConfigMap and the managed Secret, plus any
user-supplied extraEnvFrom entries.
*/}}
{{- define "starcitizen-api.envFrom" -}}
- configMapRef:
    name: {{ include "starcitizen-api.configMapName" . }}
{{- with .Values.extraEnvFrom }}
{{- toYaml . | nindent 0 }}
{{- end }}
{{- end }}

{{/*
Checksum annotations so that ConfigMap / Secret rollouts are picked up.
*/}}
{{- define "starcitizen-api.checksumAnnotations" -}}
checksum/config: {{ include (print $.Template.BasePath "/configmap.yaml") . | sha256sum }}
checksum/secret: {{ include (print $.Template.BasePath "/secret.yaml") . | sha256sum }}
{{- end }}
