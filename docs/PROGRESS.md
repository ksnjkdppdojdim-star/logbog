# LogBog — Suivi d'avancement

> Dernière mise à jour : 2026-04-09

Ce document trace l'avancement réel du projet. Il est mis à jour à chaque livrable complété.

---

## Tableau de bord global

| Phase | Nom | Statut | Progression | Début | Fin |
|-------|-----|--------|-------------|-------|-----|
| 0 | Fondations | **Non commencé** | 0/7 | - | - |
| 1 | Log Packs Framework | Non commencé | 0/8 | - | - |
| 2 | Collection & Storage | Non commencé | 0/7 | - | - |
| 3 | Correlation Engine | Non commencé | 0/5 | - | - |
| 4 | API & Dashboard | Non commencé | 0/6 | - | - |
| 5 | Intelligence Layer | Non commencé | 0/5 | - | - |
| 6 | Conformité & Production | Non commencé | 0/6 | - | - |
| 7 | Scaling & Ecosystem | Non commencé | 0/5 | - | - |

**Progression totale : 0/49 tâches (0%)**

---

## Détail par phase

### Phase 0 — Fondations

| ID | Tâche | Statut | Date | Notes |
|----|-------|--------|------|-------|
| P0-01 | Workspace Rust multi-crate | Non commencé | - | |
| P0-02 | Structure de configuration | Non commencé | - | |
| P0-03 | CLI de base (clap) | Non commencé | - | |
| P0-04 | Logging interne (tracing) | Non commencé | - | |
| P0-05 | CI/CD GitHub Actions | Non commencé | - | |
| P0-06 | Docker de développement | Non commencé | - | |
| P0-07 | Tests unitaires | Non commencé | - | |

### Phase 1 — Log Packs Framework

| ID | Tâche | Statut | Date | Notes |
|----|-------|--------|------|-------|
| P1-01 | Spécification format de pack | Non commencé | - | |
| P1-02 | Moteur de parsing universel | Non commencé | - | |
| P1-03 | Registre de packs | Non commencé | - | |
| P1-04 | Pack nginx | Non commencé | - | |
| P1-05 | Pack PHP-FPM | Non commencé | - | |
| P1-06 | Pack MySQL/MariaDB | Non commencé | - | |
| P1-07 | Pack systemd/journal | Non commencé | - | |
| P1-08 | Pack syslog | Non commencé | - | |

### Phase 2 — Collection & Storage

| ID | Tâche | Statut | Date | Notes |
|----|-------|--------|------|-------|
| P2-01 | File watcher (inotify) | Non commencé | - | |
| P2-02 | Journal reader (systemd) | Non commencé | - | |
| P2-03 | Récepteur OTLP (gRPC/HTTP) | Non commencé | - | |
| P2-04 | Récepteur Syslog (UDP/TCP) | Non commencé | - | |
| P2-05 | Stockage DuckDB | Non commencé | - | |
| P2-06 | Pipeline d'ingestion | Non commencé | - | |
| P2-07 | Rétention et lifecycle | Non commencé | - | |

### Phase 3 — Correlation Engine

| ID | Tâche | Statut | Date | Notes |
|----|-------|--------|------|-------|
| P3-01 | Extracteur d'identifiants | Non commencé | - | |
| P3-02 | Corrélation temporelle | Non commencé | - | |
| P3-03 | Chaînes causales | Non commencé | - | |
| P3-04 | Vue timeline d'incident | Non commencé | - | |
| P3-05 | Règles de corrélation custom | Non commencé | - | |

### Phase 4 — API & Dashboard

| ID | Tâche | Statut | Date | Notes |
|----|-------|--------|------|-------|
| P4-01 | API REST (axum) | Non commencé | - | |
| P4-02 | Authentification & RBAC | Non commencé | - | |
| P4-03 | Recherche full-text (Tantivy) | Non commencé | - | |
| P4-04 | Dashboard web (SvelteKit) | Non commencé | - | |
| P4-05 | Système d'alertes | Non commencé | - | |
| P4-06 | WebSocket live tail | Non commencé | - | |

### Phase 5 — Intelligence Layer

| ID | Tâche | Statut | Date | Notes |
|----|-------|--------|------|-------|
| P5-01 | Baseline learning | Non commencé | - | |
| P5-02 | Détection d'anomalies | Non commencé | - | |
| P5-03 | Clustering d'erreurs | Non commencé | - | |
| P5-04 | Intégration LLM | Non commencé | - | |
| P5-05 | Rapports automatiques | Non commencé | - | |

### Phase 6 — Conformité & Production

| ID | Tâche | Statut | Date | Notes |
|----|-------|--------|------|-------|
| P6-01 | Détection/masquage PII | Non commencé | - | |
| P6-02 | Rétention RGPD | Non commencé | - | |
| P6-03 | Multi-tenancy | Non commencé | - | |
| P6-04 | Haute disponibilité | Non commencé | - | |
| P6-05 | Monitoring de LogBog | Non commencé | - | |
| P6-06 | Hardening sécurité | Non commencé | - | |

### Phase 7 — Scaling & Ecosystem

| ID | Tâche | Statut | Date | Notes |
|----|-------|--------|------|-------|
| P7-01 | Packs additionnels (50+) | Non commencé | - | |
| P7-02 | Storage tiering S3 | Non commencé | - | |
| P7-03 | SDK de packs | Non commencé | - | |
| P7-04 | Agent léger edge | Non commencé | - | |
| P7-05 | Intégrations (Grafana, etc.) | Non commencé | - | |

---

## Journal des changements

| Date | Changement | Phase | Par |
|------|-----------|-------|-----|
| 2026-04-09 | Création du projet, documentation initiale | Setup | - |

---

## Légende des statuts

- **Non commencé** : Pas encore démarré
- **En cours** : Travail en cours
- **En review** : Code terminé, en attente de validation
- **Terminé** : Complété et validé
- **Bloqué** : En attente d'une dépendance ou décision
