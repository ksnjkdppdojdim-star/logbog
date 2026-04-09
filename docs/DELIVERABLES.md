# LogBog — Livrables par phase

Ce document définit les livrables concrets pour chaque phase, permettant de valider formellement la complétion de chaque étape.

---

## Phase 0 — Fondations

| # | Livrable | Format | Critère d'acceptation |
|---|----------|--------|----------------------|
| L0.1 | Binaire `logbog` compilé | Exécutable Linux x86_64 | `logbog --version` affiche la version |
| L0.2 | Commande `logbog init` | CLI | Crée un fichier `logbog.toml` valide |
| L0.3 | Commande `logbog status` | CLI | Affiche l'état du service (même si vide) |
| L0.4 | CI/CD GitHub Actions | Workflow YAML | Build + test + clippy passent au vert |
| L0.5 | Docker de dev | docker-compose.dev.yml | `docker-compose up` lance les services de test |
| L0.6 | Tests unitaires | `cargo test` | 100% des tests passent, couverture > 70% |
| L0.7 | Documentation Phase 0 | Markdown | README mis à jour avec les instructions d'installation |

**Version : 0.1.0**

---

## Phase 1 — Log Packs Framework

| # | Livrable | Format | Critère d'acceptation |
|---|----------|--------|----------------------|
| L1.1 | Spécification pack.toml | Document + code | Struct Rust qui désérialise un pack.toml |
| L1.2 | Moteur de parsing | Crate Rust | Parse regex, grok, JSON, syslog correctement |
| L1.3 | Commande `logbog install` | CLI | Installe un pack depuis le registre local |
| L1.4 | Commande `logbog pack test` | CLI | Valide un pack sur ses fichiers testdata |
| L1.5 | Pack nginx | Répertoire pack | Parse 100% des lignes access.log et error.log |
| L1.6 | Pack php-fpm | Répertoire pack | Parse error.log et slow.log (multilignes) |
| L1.7 | Pack mysql | Répertoire pack | Parse error.log et slow-query.log |
| L1.8 | Pack systemd | Répertoire pack | Lit le journal systemd correctement |
| L1.9 | Pack syslog | Répertoire pack | Parse RFC 3164 et 5424 |
| L1.10 | Guide de création de pack | Markdown | Un développeur externe peut créer un pack |

**Version : 0.2.0**

---

## Phase 2 — Collection & Storage

| # | Livrable | Format | Critère d'acceptation |
|---|----------|--------|----------------------|
| L2.1 | File watcher | Crate Rust | Détecte les nouvelles lignes + gère la rotation |
| L2.2 | Journal reader | Crate Rust | Lit le journal systemd en streaming |
| L2.3 | Récepteur OTLP | Endpoint gRPC + HTTP | Reçoit des logs OTel et les stocke |
| L2.4 | Stockage DuckDB | Crate Rust | Insert batch + query SQL fonctionnels |
| L2.5 | Pipeline d'ingestion | Code Rust | source → parse → store fonctionne end-to-end |
| L2.6 | Commande `logbog query` | CLI | Exécute des requêtes SQL sur les logs stockés |
| L2.7 | Bookmarking | Code Rust | Reprise correcte après redémarrage |
| L2.8 | Rétention automatique | Config + code | Les logs expirés sont supprimés automatiquement |

**Version : 0.3.0**

---

## Phase 3 — Correlation Engine

| # | Livrable | Format | Critère d'acceptation |
|---|----------|--------|----------------------|
| L3.1 | Extracteur d'identifiants | Code Rust | Extrait IP, PID, request_id, timestamp |
| L3.2 | Corrélation temporelle | Code Rust | Corrèle nginx→php→mysql sur un scénario 502 |
| L3.3 | Chaînes causales | Code Rust | Détecte les séquences connues (502, OOM) |
| L3.4 | API timeline | Endpoint REST | `/api/v1/incidents/{id}/timeline` retourne JSON |
| L3.5 | Règles custom | DSL + code | Syntaxe WHEN/WITHIN/CORRELATE fonctionne |

**Version : 0.4.0**

---

## Phase 4 — API & Dashboard

| # | Livrable | Format | Critère d'acceptation |
|---|----------|--------|----------------------|
| L4.1 | API REST complète | Endpoints axum | Swagger/OpenAPI auto-généré |
| L4.2 | Authentification | JWT + bcrypt | Login, tokens, RBAC fonctionnels |
| L4.3 | Recherche full-text | Tantivy intégré | Recherche < 2s sur 10M de logs |
| L4.4 | Dashboard web | App SvelteKit | Charge en < 2s, dashboards par pack |
| L4.5 | Live tail | WebSocket | Stream filtrable en temps réel, latence < 500ms |
| L4.6 | Système d'alertes | Config + code | Notification webhook/email/Slack fonctionnelle |

**Version : 0.5.0**

---

## Phase 5 — Intelligence Layer

| # | Livrable | Format | Critère d'acceptation |
|---|----------|--------|----------------------|
| L5.1 | Baseline learning | Code Rust | Apprend le "normal" sur 7 jours de données |
| L5.2 | Détection d'anomalies | Code Rust | Détecte un spike 3x en < 30s |
| L5.3 | Clustering d'erreurs | Code Rust | > 90% de regroupement correct |
| L5.4 | Intégration LLM | API + config | Résumé d'incident via Ollama local |
| L5.5 | Rapports automatiques | PDF/HTML + email | Rapport quotidien généré et envoyé |

**Version : 0.6.0**

---

## Phase 6 — Conformité & Production

| # | Livrable | Format | Critère d'acceptation |
|---|----------|--------|----------------------|
| L6.1 | Masquage PII | Config + code | Emails, IPs, tel masqués automatiquement |
| L6.2 | Rétention RGPD | Config + code | Suppression certifiée + journal |
| L6.3 | Multi-tenancy | Config + code | Isolation des données par tenant |
| L6.4 | Métriques Prometheus | Endpoint `/metrics` | Scrappable par Prometheus |
| L6.5 | TLS end-to-end | Config | HTTPS sur tous les endpoints |
| L6.6 | Audit de sécurité | Rapport | `cargo-audit` + OWASP = zéro critique |

**Version : 1.0.0** (première version production-ready)

---

## Phase 7 — Scaling & Ecosystem

| # | Livrable | Format | Critère d'acceptation |
|---|----------|--------|----------------------|
| L7.1 | 50+ Log Packs | Répertoires pack | Chaque pack a des tests + testdata |
| L7.2 | Storage tiering S3 | Config + code | Export Parquet + requêtes fédérées |
| L7.3 | SDK de packs | Lib Rust + YAML | Documentation + exemples |
| L7.4 | Agent edge | Binaire < 5 MB | Collecte + buffer + delivery garanti |
| L7.5 | Plugin Grafana | Datasource plugin | Requêtes LogBog depuis Grafana |

**Version : 1.x**

---

## Résumé des versions

| Version | Phase | Nom de release | Description |
|---------|-------|----------------|-------------|
| 0.1.0 | 0 | **Foundation** | Skeleton, CLI, CI |
| 0.2.0 | 1 | **Pack It** | Framework de packs + 5 packs |
| 0.3.0 | 2 | **Pipeline** | Collecte + stockage |
| 0.4.0 | 3 | **Connect** | Corrélation cross-logs |
| 0.5.0 | 4 | **Dashboard** | API + interface web |
| 0.6.0 | 5 | **Smart** | IA + anomaly detection |
| 1.0.0 | 6 | **Production** | RGPD + sécurité + HA |
| 1.x | 7 | **Ecosystem** | Scaling + plugins + packs |
