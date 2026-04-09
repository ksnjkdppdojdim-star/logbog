# Changelog

Toutes les modifications notables du projet LogBog sont documentees dans ce fichier.

Le format est base sur [Keep a Changelog](https://keepachangelog.com/fr/1.1.0/),
et ce projet adhere au [Semantic Versioning](https://semver.org/lang/fr/).

---

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
