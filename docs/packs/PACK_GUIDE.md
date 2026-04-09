# Guide de création de Log Packs

Un **Log Pack** est l'unité modulaire fondamentale de LogBog. Chaque pack encapsule tout ce qui est nécessaire pour collecter, parser, visualiser et alerter sur un type spécifique de log.

---

## Structure d'un pack

```
packs/<nom>/
├── pack.toml           # Manifest : métadonnées, sources, parseurs, alertes
├── testdata/           # Fichiers de logs réels pour les tests
│   ├── access.log
│   └── error.log
├── dashboard.json      # Configuration du dashboard (optionnel)
└── README.md           # Documentation du pack
```

---

## Le fichier pack.toml

### Métadonnées

```toml
[pack]
name = "nginx"                          # Identifiant unique
version = "1.0.0"                       # Semver
description = "Nginx access and error log parser"
author = "LogBog Team"
license = "Apache-2.0"
tags = ["web", "reverse-proxy", "http"]
dependencies = []                       # Autres packs requis
```

### Sources de logs

Chaque source représente un fichier ou type de log distinct.

```toml
[[sources]]
name = "access"                         # Identifiant de la source
paths = [                               # Chemins des fichiers (globs supportés)
    "/var/log/nginx/access.log",
    "/var/log/nginx/*-access.log"
]
format = "grok"                         # "grok" | "regex" | "json" | "logfmt" | "syslog"
pattern = '...'                         # Pattern de parsing
timestamp_format = "%d/%b/%Y:%H:%M:%S %z"
timestamp_field = "timestamp"           # Champ contenant le timestamp
multiline = false                       # true pour les stack traces
multiline_start = '^[^\s]'             # Regex pour le début d'une entrée multilignes
```

### Formats de parsing supportés

| Format | Usage | Exemple de source |
|--------|-------|-------------------|
| `grok` | Logs textuels structurés | nginx access, Apache |
| `regex` | Logs avec format custom | nginx error, app custom |
| `json` | Logs JSON | Applications modernes |
| `logfmt` | Format clé=valeur | Heroku-style |
| `syslog` | RFC 3164 / 5424 | syslog, rsyslog |

### Schéma des champs

```toml
[schema]
fields = [
    { name = "client_ip", type = "ip", indexed = true, description = "Client IP address" },
    { name = "status", type = "int", indexed = true, description = "HTTP status code" },
    { name = "request_time", type = "float", indexed = false, description = "Request duration in seconds" },
    { name = "message", type = "string", indexed = true, description = "Log message" },
]
```

Types supportés : `string`, `int`, `float`, `bool`, `ip`, `datetime`, `duration`

### Règles d'alerte

```toml
[[alerts]]
name = "5xx_spike"
description = "Spike of 5xx errors"
condition = "count(status >= 500) > 10 in 1m"
severity = "critical"                   # "info" | "warning" | "critical"
cooldown = "5m"                         # Temps minimum entre deux alertes
message = "{{count}} erreurs 5xx en 1 minute sur {{source}}"
```

### Règles de corrélation

```toml
[[correlations]]
target_pack = "php-fpm"                 # Pack cible
match_fields = ["timestamp", "uri:script"]  # Champs à matcher
window = "2s"                           # Fenêtre temporelle
description = "Correlate nginx request with PHP-FPM processing"
```

---

## Tester un pack

```bash
# Tester le parsing sur les fichiers testdata/
logbog pack test nginx

# Afficher les champs parsés pour une ligne
logbog pack parse nginx "192.168.1.1 - - [09/Apr/2026:10:00:00 +0000] \"GET /api/users HTTP/1.1\" 200 1234 \"-\" \"curl/7.68\" 0.042"

# Valider la syntaxe du pack.toml
logbog pack validate nginx
```

---

## Bonnes pratiques

1. **Toujours inclure des testdata** : des vrais fichiers de logs (anonymisés) pour valider le parsing
2. **Couvrir les cas limites** : lignes malformées, encodages spéciaux, logs vides
3. **Documenter les champs** : chaque champ du schéma doit avoir une description
4. **Alertes conservatrices** : mieux vaut des seuils élevés par défaut (l'utilisateur peut baisser)
5. **Corrélations précises** : n'utiliser que des champs fiables pour la corrélation

---

## Packs officiels prévus

| Pack | Priorité | Sources |
|------|----------|---------|
| nginx | P0 | access.log, error.log |
| php-fpm | P0 | error.log, slow.log |
| mysql | P0 | error.log, slow-query.log |
| systemd | P0 | journalctl |
| syslog | P0 | /var/log/syslog, auth.log |
| apache | P1 | access.log, error.log |
| postgresql | P1 | postgresql.log |
| redis | P1 | redis-server.log |
| docker | P1 | container logs (JSON) |
| python | P1 | application logs |
| node | P2 | PM2, application logs |
| mongodb | P2 | mongod.log |
| haproxy | P2 | haproxy.log |
| fail2ban | P2 | fail2ban.log |
| cron | P2 | /var/log/cron |
