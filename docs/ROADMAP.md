# LogBog — Roadmap technique détaillée

Ce document décrit les phases de développement de LogBog, les livrables attendus, les dépendances et les critères de validation pour chaque étape.

---

## Vue d'ensemble des phases

| Phase | Nom | Durée estimée | Dépendances | Livrable principal |
|-------|-----|---------------|-------------|-------------------|
| 0 | Fondations | 3-4 semaines | Aucune | Skeleton Rust, CLI, config, CI/CD |
| 1 | Log Packs Framework | 4-6 semaines | Phase 0 | Framework de packs + 5 premiers packs |
| 2 | Collection & Storage | 4-5 semaines | Phase 0 | Pipeline de collecte + stockage DuckDB |
| 3 | Correlation Engine | 4-6 semaines | Phase 1, 2 | Moteur de corrélation cross-logs |
| 4 | API & Dashboard | 4-5 semaines | Phase 2, 3 | API REST + interface web |
| 5 | Intelligence Layer | 6-8 semaines | Phase 3, 4 | Anomaly detection + clustering |
| 6 | Conformité & Production | 4-5 semaines | Phase 4, 5 | RGPD, RBAC, hardening |
| 7 | Scaling & Ecosystem | Continu | Phase 6 | 50+ packs, S3 tiering, plugins |

---

## Phase 0 — Fondations

**Objectif** : Poser les bases solides du projet. Un binaire Rust qui compile, un CLI fonctionnel, une CI qui passe.

### Tâches

- [ ] **P0-01** : Initialisation du projet Rust (workspace Cargo multi-crate)
  - Crates : `logbog-core`, `logbog-cli`, `logbog-collector`, `logbog-engine`, `logbog-storage`, `logbog-api`, `logbog-packs`
  - Workspace Cargo.toml avec dépendances partagées

- [ ] **P0-02** : Structure de configuration
  - Fichier `logbog.toml` — config principale
  - Détection automatique de l'OS et des services installés
  - Validation de la configuration au démarrage

- [ ] **P0-03** : CLI de base (`logbog-cli`)
  - Commandes : `init`, `start`, `stop`, `status`, `install <pack>`, `list`, `config`
  - Librairie : `clap` v4 pour le parsing des commandes
  - Output coloré avec `colored` ou `console`

- [ ] **P0-04** : Système de logging interne
  - LogBog doit logger proprement ses propres opérations
  - Librairie : `tracing` + `tracing-subscriber`
  - Niveaux : ERROR, WARN, INFO, DEBUG, TRACE

- [ ] **P0-05** : CI/CD GitHub Actions
  - Build + test sur Linux (Ubuntu latest)
  - Linting (`clippy`), formatting (`rustfmt`)
  - Security audit (`cargo-audit`)
  - Release automatique des binaires

- [ ] **P0-06** : Docker de développement
  - Dockerfile multi-stage (build Rust + runtime minimal)
  - docker-compose.dev.yml avec services de test (nginx, php-fpm, mysql)
  - Génération de logs de test

- [ ] **P0-07** : Tests unitaires et framework de test
  - Setup `cargo test` avec fixtures
  - Helpers de test pour générer des lignes de log

### Livrables Phase 0
- Binaire `logbog` qui compile et affiche l'aide
- `logbog init` crée un fichier de config
- `logbog status` affiche l'état (vide)
- CI verte sur GitHub
- Docker de dev fonctionnel

### Critères de validation
- `cargo build --release` sans erreur ni warning
- `cargo test` — tous les tests passent
- `cargo clippy` — zéro warning
- `logbog --help` affiche les commandes disponibles
- `logbog init` crée `logbog.toml` valide
- GitHub Actions passe au vert

---

## Phase 1 — Log Packs Framework

**Objectif** : Le système de packs modulaires — la killer feature de LogBog. Permettre d'ajouter le support d'un type de log avec `logbog install <pack>`.

### Tâches

- [ ] **P1-01** : Spécification du format de pack
  - Fichier `pack.toml` : métadonnées, source du log, format, champs
  - Répertoire standard : `parser/`, `schema/`, `dashboard/`, `alerts/`, `correlations/`
  - Versioning sémantique des packs

- [ ] **P1-02** : Moteur de parsing universel
  - Support des formats : regex, grok, JSON, logfmt, CSV, syslog (RFC 3164 & 5424)
  - Pipeline de parsing configurable (chain de transformations)
  - Normalisation des champs : timestamp, level, message, source, host
  - Librairies : `regex`, `chrono`, `serde_json`

- [ ] **P1-03** : Registre de packs
  - Registre local (répertoire de packs installés)
  - Registre distant (GitHub repo avec index)
  - Commandes : `logbog install`, `logbog update`, `logbog remove`
  - Résolution de dépendances entre packs

- [ ] **P1-04** : Pack — nginx
  - Parseur access log (format combined + custom)
  - Parseur error log
  - Champs : status_code, method, uri, response_time, upstream_time, client_ip, user_agent
  - Alertes : spike 5xx, latence élevée, requêtes suspectes
  - Dashboard : traffic, codes de statut, top URIs, top IPs

- [ ] **P1-05** : Pack — PHP-FPM
  - Parseur slow log
  - Parseur error log (avec stack traces multilignes)
  - Champs : pool, pid, duration, script, error_type, trace
  - Alertes : slow requests, fatal errors, pool exhaustion
  - Corrélation avec nginx (via request timing + script path)

- [ ] **P1-06** : Pack — MySQL / MariaDB
  - Parseur error log
  - Parseur slow query log
  - Parseur general log (optionnel, volume élevé)
  - Champs : query_time, lock_time, rows_examined, query, user, db
  - Alertes : slow queries récurrentes, deadlocks, connexions refusées

- [ ] **P1-07** : Pack — systemd/journal
  - Lecture directe du journal systemd via `journalctl --output=json`
  - Champs : unit, priority, message, pid, uid
  - Alertes : service crashes (restart loops), OOM-killer, kernel panics
  - Filtrage par unité de service

- [ ] **P1-08** : Pack — syslog générique
  - Parseur RFC 3164 et RFC 5424
  - Support facility/severity
  - Auth logs (ssh, sudo, su)
  - Alertes : tentatives SSH échouées, escalade de privilèges

### Livrables Phase 1
- Framework de pack complet et documenté
- 5 packs fonctionnels : nginx, php-fpm, mysql, systemd, syslog
- `logbog install nginx` fonctionne
- Chaque pack parse correctement les logs réels
- Tests avec des fichiers de logs réels comme fixtures

### Critères de validation
- Chaque pack parse 100% des lignes d'un fichier de log de référence
- Temps de parsing < 100ms pour 10 000 lignes
- `logbog install <pack>` + `logbog test <pack>` fonctionne
- Documentation de création d'un pack custom

---

## Phase 2 — Collection & Storage

**Objectif** : Lire les logs en temps réel et les stocker efficacement.

### Tâches

- [ ] **P2-01** : File watcher
  - Surveillance de fichiers avec `notify` (inotify sur Linux)
  - Gestion du log rotation (détection rename/truncate)
  - Bookmarking : sauvegarde de la position de lecture (offset)
  - Support des fichiers compressés (.gz) pour le rattrapage

- [ ] **P2-02** : Journal reader
  - Lecture du journal systemd via `libsystemd` bindings
  - Filtrage par unité, priorité, time range
  - Cursor-based reading (pas de relecture)

- [ ] **P2-03** : Récepteur OpenTelemetry (OTLP)
  - Endpoint gRPC OTLP pour recevoir des logs de collecteurs externes
  - Endpoint HTTP OTLP (fallback)
  - Mapping OTLP → schéma interne LogBog
  - Librairies : `tonic` (gRPC), `opentelemetry-proto`

- [ ] **P2-04** : Récepteur Syslog
  - Serveur syslog UDP/TCP (port 514 ou custom)
  - Support RFC 3164 et RFC 5424
  - Pour les appareils réseau, routeurs, etc.

- [ ] **P2-05** : Stockage DuckDB (hot tier)
  - Schéma de base : table `logs` avec colonnes typées
  - Index sur timestamp, source, level
  - Partitionnement par jour
  - Requêtes SQL natives
  - Librairie : `duckdb-rs`

- [ ] **P2-06** : Pipeline d'ingestion
  - Architecture : source → parser → enrichment → storage
  - Backpressure handling (quand le stockage est saturé)
  - Batch writes pour la performance
  - Métriques d'ingestion (logs/sec, bytes/sec, erreurs)

- [ ] **P2-07** : Rétention et lifecycle
  - Politique de rétention configurable par pack/source
  - Suppression automatique des données expirées
  - Statistiques de stockage par source

### Livrables Phase 2
- Collecte en temps réel de fichiers de log
- Stockage dans DuckDB avec requêtes SQL
- `logbog query "SELECT * FROM logs WHERE level = 'ERROR' AND source = 'nginx' LIMIT 10"`
- Métriques d'ingestion visibles via `logbog status`

### Critères de validation
- Aucune ligne perdue lors d'un log rotation
- Ingestion soutenue de 10 000 logs/sec sur hardware modeste
- Requête SQL sur 1M de lignes < 1 seconde
- Reprise correcte après redémarrage (bookmarking)

---

## Phase 3 — Correlation Engine

**Objectif** : Le vrai différenciateur. Corréler automatiquement les logs de sources différentes pour reconstituer des chaînes d'événements.

### Tâches

- [ ] **P3-01** : Extracteur d'identifiants
  - Extraction automatique de : timestamps, IPs, PIDs, request IDs, session IDs
  - Patterns configurables par pack
  - Normalisation des timestamps cross-sources (même timezone)

- [ ] **P3-02** : Moteur de corrélation temporelle
  - Fenêtre de corrélation configurable (défaut : ±2 secondes)
  - Matching par IP client (nginx access → php-fpm → mysql)
  - Matching par PID/process (systemd → application)
  - Score de confiance pour chaque corrélation

- [ ] **P3-03** : Chaînes causales
  - Détection de séquences : requête → traitement → erreur → conséquence
  - Graphe de causalité pour les incidents
  - Template de chaînes connues (502 chain, OOM chain, disk full chain)

- [ ] **P3-04** : Vue timeline d'incident
  - API endpoint : `/api/v1/incidents/{id}/timeline`
  - Agrégation temporelle des événements corrélés
  - Marqueurs de sévérité et d'impact

- [ ] **P3-05** : Règles de corrélation custom
  - DSL simple pour définir des corrélations
  - Exemple : `WHEN nginx.status >= 500 WITHIN 2s CORRELATE php.error WHERE php.script = nginx.uri`
  - Stockage des règles dans les packs

### Livrables Phase 3
- Corrélation automatique fonctionnelle sur les 5 packs de la Phase 1
- Chaînes causales pour les scénarios courants (502, OOM, slow query)
- API de timeline d'incident

### Critères de validation
- Scénario 502 : corrèle correctement nginx → php-fpm → mysql en < 500ms
- Taux de faux positifs < 5% sur un jeu de test réaliste
- Corrélation fonctionne même avec des timestamps légèrement décalés (±1s)

---

## Phase 4 — API & Dashboard

**Objectif** : Interface utilisateur pour visualiser, chercher et explorer les logs.

### Tâches

- [ ] **P4-01** : API REST
  - Framework : `axum` (Rust, async, performant)
  - Endpoints : logs, search, incidents, timeline, packs, config, metrics
  - Pagination, filtrage, tri
  - OpenAPI/Swagger auto-généré

- [ ] **P4-02** : Authentification & autorisation
  - Auth locale (user/password avec bcrypt)
  - JWT tokens pour les sessions
  - RBAC : admin, viewer, pack-specific access

- [ ] **P4-03** : Recherche full-text
  - Intégration Tantivy pour la recherche dans le contenu des logs
  - Recherche par champs structurés (SQL) + texte libre (Tantivy)
  - Auto-complétion et suggestions

- [ ] **P4-04** : Dashboard web (SvelteKit)
  - Vue temps réel (tail -f dans le navigateur via WebSocket)
  - Dashboard par pack (métriques clés, graphiques)
  - Vue corrélation (timeline d'incident interactive)
  - Vue recherche (barre de recherche unifiée)
  - Dark mode par défaut

- [ ] **P4-05** : Système d'alertes
  - Règles d'alerte configurables
  - Canaux de notification : webhook, email (SMTP), Slack, Telegram
  - Historique des alertes déclenchées
  - Cooldown et agrégation (pas de spam)

- [ ] **P4-06** : WebSocket live tail
  - Stream de logs en temps réel filtrable
  - Highlight des erreurs et anomalies
  - Pause/resume du stream

### Livrables Phase 4
- API REST complète et documentée (Swagger)
- Dashboard web fonctionnel
- Live tail dans le navigateur
- Système d'alertes avec notifications

### Critères de validation
- Dashboard charge en < 2 secondes
- Live tail avec latence < 500ms
- API supporte 100 requêtes concurrentes
- Recherche full-text sur 10M de logs < 2 secondes

---

## Phase 5 — Intelligence Layer

**Objectif** : Rendre LogBog intelligent — détection d'anomalies, clustering, et résumés IA.

### Tâches

- [ ] **P5-01** : Baseline learning
  - Apprentissage du "normal" par source/pack (volume, distribution des niveaux, patterns)
  - Fenêtre d'apprentissage configurable (7 jours par défaut)
  - Mise à jour incrémentale de la baseline
  - Algorithmes : moyennes mobiles, écart-type, IQR

- [ ] **P5-02** : Détection d'anomalies
  - Anomalies de volume (spike/drop soudain)
  - Anomalies de pattern (nouvelle erreur jamais vue)
  - Anomalies temporelles (événement à une heure inhabituelle)
  - Scoring de sévérité (info → warning → critical)

- [ ] **P5-03** : Clustering d'erreurs
  - Regroupement automatique des messages d'erreur similaires
  - Algorithme : distance de Levenshtein + tokenisation
  - Déduplication intelligente ("500 occurrences de la même erreur" au lieu de 500 lignes)
  - Identification des "nouvelles" erreurs vs erreurs connues

- [ ] **P5-04** : Intégration LLM (optionnelle)
  - API compatible OpenAI/Anthropic pour l'analyse de logs
  - Résumé d'incident en langage naturel
  - Suggestion de cause racine
  - Mode local avec modèles Ollama (pas de données envoyées à l'extérieur)
  - Pré-traitement : LogSieve pour réduire les tokens envoyés (42% de réduction)

- [ ] **P5-05** : Rapports automatiques
  - Rapport quotidien/hebdomadaire généré automatiquement
  - Résumé des anomalies, top erreurs, tendances
  - Export PDF/HTML
  - Envoi par email configurable

### Livrables Phase 5
- Détection d'anomalies fonctionnelle sans ML externe
- Clustering d'erreurs visible dans le dashboard
- Intégration LLM optionnelle pour les résumés
- Rapports automatiques

### Critères de validation
- Détecte un spike de 3x le volume normal en < 30 secondes
- Clustering regroupe correctement > 90% des erreurs similaires
- LLM summarization fonctionne avec Ollama local
- Rapport quotidien généré et envoyé sans intervention

---

## Phase 6 — Conformité & Production-Ready

**Objectif** : Rendre LogBog prêt pour la production et conforme RGPD.

### Tâches

- [ ] **P6-01** : Détection et masquage PII
  - Détection automatique : emails, IPs, numéros de téléphone, noms
  - Masquage configurable : hash, redact, pseudonymize
  - Règles custom de PII par pack
  - Audit log des accès aux données non-masquées

- [ ] **P6-02** : Politiques de rétention RGPD
  - Rétention configurable par source et par niveau de sensibilité
  - Suppression automatique certifiée (pas de résidus)
  - Export des données sur demande (droit d'accès)
  - Journal de suppression (preuve de conformité)

- [ ] **P6-03** : Multi-tenancy
  - Isolation des données par tenant
  - Configuration et packs par tenant
  - Quotas de stockage et d'ingestion
  - Administration centralisée

- [ ] **P6-04** : Haute disponibilité
  - Mode cluster (réplication des données)
  - Failover automatique
  - Load balancing des écritures

- [ ] **P6-05** : Monitoring de LogBog lui-même
  - Métriques Prometheus `/metrics`
  - Health check endpoint
  - Alertes sur la santé de LogBog (disk full, ingestion stalled)

- [ ] **P6-06** : Hardening sécurité
  - TLS partout (API, collecte, inter-nodes)
  - Rate limiting
  - Input sanitization
  - Security headers
  - Audit de sécurité (cargo-audit, OWASP)

### Livrables Phase 6
- Conformité RGPD documentée
- Mode multi-tenant fonctionnel
- Métriques Prometheus
- Documentation de déploiement production

### Critères de validation
- PII masqué dans 100% des cas configurés
- Suppression RGPD vérifiable
- TLS end-to-end fonctionnel
- Aucune vulnérabilité critique (cargo-audit)

---

## Phase 7 — Scaling & Ecosystem (continu)

**Objectif** : Enrichir l'écosystème et supporter le passage à l'échelle.

### Tâches

- [ ] **P7-01** : Packs additionnels
  - Apache httpd, PostgreSQL, Redis, MongoDB, Docker, Kubernetes
  - HAProxy, Traefik, Caddy
  - Python (Flask, FastAPI, Celery), Node.js (Express, PM2)
  - Postfix/Dovecot (mail), BIND/dnsmasq (DNS)
  - Fail2ban, UFW/iptables, ModSecurity (sécurité)
  - Objectif : 50+ packs

- [ ] **P7-02** : Storage tiering S3
  - Export automatique vers S3/MinIO en format Parquet
  - Requêtes fédérées (hot DuckDB + warm S3)
  - Dashboard de coûts par source

- [ ] **P7-03** : SDK et API de packs
  - SDK pour créer des packs en Rust, Python, ou YAML
  - Registre communautaire de packs (logbog.dev/packs)
  - CI de validation automatique des packs soumis

- [ ] **P7-04** : Agent léger pour edge
  - Binaire < 5 MB pour VPS, IoT, edge
  - Buffer local avec guaranteed delivery
  - Compression et batching

- [ ] **P7-05** : Intégrations
  - Grafana datasource plugin
  - PagerDuty, OpsGenie intégration
  - Terraform provider pour la config-as-code
  - Ansible role pour le déploiement

---

## Dépendances entre phases

```
Phase 0 (Fondations)
  ├── Phase 1 (Log Packs)
  │     └── Phase 3 (Correlation) ──┐
  └── Phase 2 (Collection/Storage)──┤
                                    ├── Phase 4 (API/Dashboard)
                                    │     └── Phase 5 (Intelligence)
                                    │           └── Phase 6 (Conformité)
                                    │                 └── Phase 7 (Ecosystem)
                                    └─────────────────────┘
```

Les Phases 1 et 2 peuvent avancer **en parallèle** après la Phase 0.
