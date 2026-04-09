# Changelog

Toutes les modifications notables du projet LogBog sont documentees dans ce fichier.

Le format est base sur [Keep a Changelog](https://keepachangelog.com/fr/1.1.0/),
et ce projet adhere au [Semantic Versioning](https://semver.org/lang/fr/).

---

## [0.3.0] — 2026-04-09 (Phase 2 — Collect & Store)

### Ajoute
- **DuckDB storage** : schema logs, insert unitaire + batch, requetes SQL, stats, retention auto
- **File watcher** : notify v8 / inotify, glob expansion, detection rotation (inode + truncate), bookmarking JSON
- **Journal reader** : lecture async de journalctl (--output=json --follow), filtrage par unite
- **Syslog receiver** : serveur UDP + TCP concurrent, ports configurables
- **OTLP receiver** : endpoint HTTP (newline-delimited JSON), base pour gRPC futur
- **Pipeline d'ingestion** : source -> parser -> storage, batch writes, backpressure channel, flush periodique
- **Retention lifecycle** : suppression auto des logs expires (configurable retention_days)
- **CLI `query`** : requete SQL brute + filtres structures (source, pack, level, search, limit)
- **CLI `tail`** : affichage des N derniers logs avec coloration par niveau
- **CLI `remove`** : suppression de packs avec nettoyage du repertoire
- **CLI `start` upgraded** : lance le pipeline complet (watcher + syslog + journal + pipeline)
- **CLI `status` upgraded** : affiche les stats DuckDB en live (logs stored, storage used, par source)
- ~20 nouveaux tests (storage 12, watcher 3, bookmark 4, syslog 2)

## [0.2.0] — 2026-04-09 (Phase 1 — Pack It)

### Ajoute
- **PackEngine** : connecte les parsers aux packs via factory pattern (regex, grok, json, logfmt, syslog)
- **Validation renforcee** : semver, format connu, regex valide, types de champs, coherence multiline
- **RemoteRegistry** : index builtin de 10 packs (5 core + 5 planned), recherche par nom/tag
- **Test fixtures** : fichiers de logs realistes pour les 5 packs (nginx access/error, php-fpm, mysql, syslog, systemd)
- **Integration tests** : 100% parse rate valide sur chaque fixture
- Pack nginx : access + error log, regex avec groupes nommes, timestamp parsing
- Pack PHP-FPM : error log, multiline support, regex pattern
- Pack MySQL : error log, timestamp ISO 8601, regex pattern
- Pack systemd : journal JSON, all fields
- Pack syslog : RFC 3164 + 5424, auto-detection, priority-to-level mapping
- ~25 nouveaux tests (engine 11, manifest 7, remote 4, integration 9)

## [0.1.0] — 2026-04-09 (Phase 0 — Foundation)

### Ajoute
- Workspace Rust multi-crate (8 crates : core, cli, collector, parser, packs, engine, storage, api)
- CLI complet avec clap v4 : `init`, `start`, `stop`, `status`, `install`, `remove`, `list`, `pack validate`, `pack info`, `config`
- Systeme de configuration `logbog.toml` avec valeurs par defaut, chargement/sauvegarde, detection de chemins
- Logging interne avec `tracing` + `tracing-subscriber` (env filter `LOGBOG_LOG`)
- Types fondamentaux : `LogEntry`, `LogLevel`, `RawLogLine`, `Correlation`, `ServiceStatus`, `Config`
- Systeme d'erreurs unifie avec `thiserror`
- 4 parseurs : regex (groupes nommes), JSON, logfmt, syslog (RFC 3164 & 5424)
- Framework de Log Packs : manifest `pack.toml`, `PackManifest`, `PackRegistry`
- 5 packs builtin : nginx, php-fpm, mysql, systemd, syslog
- Detection automatique des services installes (nginx, apache, php-fpm, mysql, postgresql, redis, mongodb, docker)
- CI/CD GitHub Actions : check, test, clippy, fmt, security audit, build release
- Dockerfile multi-stage (build Rust + runtime minimal)
- docker-compose.dev.yml avec nginx, php-fpm, mysql et generateur de logs
- Service systemd (`logbog.service`) avec hardening securite
- 35 tests unitaires (core: 12, parser: 16, packs: 7)
- Documentation : README, ROADMAP, PROGRESS, ARCHITECTURE, CONTRIBUTING, DELIVERABLES, PACK_GUIDE
- Licence Apache 2.0
- Configuration rustfmt et .gitignore
