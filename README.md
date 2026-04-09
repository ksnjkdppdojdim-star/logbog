# LogBog

**De l'installation à l'insight en 5 minutes, pour tout serveur standard.**

LogBog est un framework modulaire open-source de gestion et d'intelligence de logs serveur. Il collecte, parse, corrèle et analyse automatiquement les logs de toutes les briques d'un serveur (nginx, Apache, PHP, Python, MySQL, systemd, etc.) via un système de **Log Packs** installables en une commande.

---

## Pourquoi LogBog ?

L'écosystème actuel de gestion de logs souffre de plusieurs problèmes majeurs :

- **Le gouffre d'accessibilité** : entre `tail -f` et un cluster ELK de 32 Go de RAM, il n'y a rien de simple
- **Zéro clé-en-main** : chaque solution existante demande des heures de configuration manuelle par source de log
- **Pas de corrélation cross-stack** : quand un 502 survient, personne ne corrèle automatiquement nginx → PHP-FPM → MySQL → OOM-killer
- **IA/ML absent en open-source** : 74% des organisations veulent de l'IA sur leurs logs, aucune solution intégrée n'existe
- **Conformité RGPD verrouillée** : rétention, masquage PII, audit — tout est derrière des licences commerciales

LogBog résout ces problèmes avec une approche modulaire et intelligente.

---

## Fonctionnalités principales

### Log Packs — Le coeur de LogBog
```bash
logbog install nginx php-fpm mysql python-django
logbog start
# Collecte, parsing, dashboard, alertes : tout fonctionne immédiatement
```

Chaque Log Pack est un module autonome contenant :
- Un parseur pré-configuré (regex/grok)
- Un schéma de données normalisé
- Un dashboard prêt à l'emploi
- Des règles d'alerte par défaut
- Des règles de corrélation inter-packs

### Correlation Engine
- Corrélation automatique cross-logs **sans instrumentation applicative**
- Utilise timestamps, IPs, PIDs et request IDs existants dans les logs
- Vue "timeline d'incident" pour le diagnostic rapide

### Intelligence Layer
- Détection d'anomalies par apprentissage de baseline
- Clustering automatique des erreurs similaires
- Intégration LLM optionnelle pour résumés en langage naturel
- Alertes intelligentes basées sur des patterns, pas des seuils

### Conformité & Multi-tenancy
- Détection et masquage automatique de PII
- Politiques de rétention configurables par source
- RBAC et audit logs natifs

---

## Architecture

```
                    ┌─────────────────────────┐
                    │      LogBog CLI          │
                    │   logbog install/start   │
                    └───────────┬─────────────┘
                                │
        ┌───────────┬───────────┼───────────┬───────────┐
        │           │           │           │           │
   ┌────▼───┐ ┌────▼───┐ ┌────▼───┐ ┌────▼───┐ ┌────▼───┐
   │ Pack:  │ │ Pack:  │ │ Pack:  │ │ Pack:  │ │ Pack:  │
   │ nginx  │ │php-fpm │ │ mysql  │ │ python │ │systemd │
   └────┬───┘ └────┬───┘ └────┬───┘ └────┬───┘ └────┬───┘
        │          │          │          │          │
        └──────────┴──────┬───┴──────────┴──────────┘
                          │
                ┌─────────▼──────────┐
                │  Collection Layer  │
                │  (OTel-compatible) │
                └─────────┬──────────┘
                          │
                ┌─────────▼──────────┐
                │ Correlation Engine │
                │ timestamp/IP/PID   │
                │ causal chains      │
                └─────────┬──────────┘
                          │
                ┌─────────▼──────────┐
                │  Intelligence      │
                │  anomaly/cluster   │
                │  LLM (optional)    │
                └─────────┬──────────┘
                          │
            ┌─────────────┼─────────────┐
            │             │             │
      ┌─────▼────┐ ┌─────▼────┐ ┌─────▼────┐
      │ Hot:     │ │ Warm:    │ │ Cold:    │
      │ DuckDB   │ │ Parquet  │ │ Archive  │
      │ (local)  │ │ (S3)     │ │ (gz)     │
      └──────────┘ └──────────┘ └──────────┘
                          │
                ┌─────────▼──────────┐
                │   API REST/gRPC    │
                │   + Web Dashboard  │
                └────────────────────┘
```

---

## Stack technique

| Composant | Technologie | Justification |
|-----------|-------------|---------------|
| Langage principal | Rust | Performance, sécurité mémoire, tendance de l'industrie |
| Collecte | Compatible OpenTelemetry | Standard ~95% d'adoption prévu |
| Stockage hot | DuckDB embarqué | SQL natif, zéro dépendance |
| Stockage warm/cold | Apache Parquet sur S3/MinIO | Réduction 80-90% des coûts |
| Requêtes | SQL standard | Pas de langage propriétaire |
| API | REST + gRPC | REST (dashboard), gRPC (haute perf) |
| Web UI | SvelteKit | Léger, réactif, moderne |
| ML/Anomaly | Rust + Python | Runtime Rust, expérimentation Python |
| Packaging | Binaire unique + Docker | Installation en une commande |

---

## Installation (à venir)

```bash
# Via script d'installation
curl -fsSL https://logbog.dev/install.sh | sh

# Via Docker
docker run -d --name logbog -v /var/log:/var/log:ro logbog/logbog:latest

# Via cargo
cargo install logbog
```

---

## Démarrage rapide (à venir)

```bash
# Initialiser LogBog sur le serveur
logbog init

# Installer des packs de logs
logbog install nginx php-fpm mysql

# Démarrer la collecte
logbog start

# Ouvrir le dashboard
logbog dashboard
# -> http://localhost:6060
```

---

## Roadmap

Voir [ROADMAP.md](docs/ROADMAP.md) pour la roadmap détaillée.

Voir [PROGRESS.md](docs/PROGRESS.md) pour le suivi d'avancement en temps réel.

---

## Contribuer

Voir [CONTRIBUTING.md](docs/CONTRIBUTING.md).

---

## Licence

Apache License 2.0 — Voir [LICENSE](LICENSE).

---

## Liens

- [Documentation](docs/)
- [Roadmap](docs/ROADMAP.md)
- [Suivi d'avancement](docs/PROGRESS.md)
- [Architecture détaillée](docs/architecture/ARCHITECTURE.md)
- [Guide des Log Packs](docs/packs/PACK_GUIDE.md)
