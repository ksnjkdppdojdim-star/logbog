# LogBog — Architecture technique détaillée

## Vue d'ensemble

LogBog est conçu comme un **workspace Rust multi-crate** où chaque composant est une crate indépendante avec des interfaces bien définies. Cette architecture permet :

- Le développement parallèle de composants
- Le test isolé de chaque couche
- Le remplacement de composants (ex: changer de stockage)
- La compilation conditionnelle (features Cargo)

---

## Structure des crates

```
logbog/
├── Cargo.toml                  # Workspace root
├── crates/
│   ├── logbog-core/            # Types partagés, traits, erreurs
│   ├── logbog-cli/             # Interface ligne de commande
│   ├── logbog-collector/       # Collecte de logs (file, journal, OTLP, syslog)
│   ├── logbog-packs/           # Framework de Log Packs + registre
│   ├── logbog-parser/          # Moteur de parsing (regex, grok, JSON, etc.)
│   ├── logbog-engine/          # Correlation engine + intelligence
│   ├── logbog-storage/         # Abstraction stockage (DuckDB, Parquet, S3)
│   ├── logbog-api/             # Serveur API REST + gRPC
│   └── logbog-web/             # Dashboard web (assets embarqués)
├── packs/                      # Log Packs officiels
│   ├── nginx/
│   ├── php-fpm/
│   ├── mysql/
│   ├── systemd/
│   └── syslog/
├── web/                        # Source SvelteKit du dashboard
├── tests/                      # Tests d'intégration et E2E
├── docs/                       # Documentation
└── deploy/                     # Docker, systemd, Ansible
```

---

## Flux de données

```
[Sources de logs]
     │
     ▼
┌─────────────┐     ┌──────────────┐
│  Collector   │────▶│    Parser     │
│ (file watch, │     │ (pack-based,  │
│  journal,    │     │  regex/grok/  │
│  OTLP,       │     │  JSON/syslog) │
│  syslog)     │     └──────┬───────┘
└─────────────┘            │
                           ▼
                   ┌───────────────┐
                   │  Enrichment   │
                   │ (geo-ip, PII  │
                   │  detection,   │
                   │  normalization)│
                   └───────┬───────┘
                           │
              ┌────────────┼────────────┐
              │            │            │
              ▼            ▼            ▼
       ┌──────────┐ ┌──────────┐ ┌──────────┐
       │ Storage  │ │Correlation│ │ Indexing │
       │ (DuckDB) │ │ Engine   │ │ (Tantivy)│
       └──────────┘ └──────────┘ └──────────┘
              │            │            │
              └────────────┼────────────┘
                           │
                    ┌──────▼───────┐
                    │  API Layer   │
                    │ (REST/gRPC/  │
                    │  WebSocket)  │
                    └──────┬───────┘
                           │
                    ┌──────▼───────┐
                    │  Dashboard   │
                    │  (SvelteKit) │
                    └──────────────┘
```

---

## Composants détaillés

### logbog-core

Types fondamentaux partagés par toutes les crates.

```rust
// Types principaux
pub struct LogEntry {
    pub id: Ulid,
    pub timestamp: DateTime<Utc>,
    pub source: String,         // "nginx", "php-fpm", etc.
    pub host: String,
    pub level: LogLevel,
    pub message: String,
    pub fields: HashMap<String, Value>,  // Champs structurés
    pub raw: String,            // Ligne originale
    pub pack: String,           // Pack qui a parsé cette entrée
}

pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
    Fatal,
}

pub struct Correlation {
    pub id: Ulid,
    pub entries: Vec<LogEntry>,
    pub confidence: f64,        // 0.0 - 1.0
    pub correlation_type: CorrelationType,
    pub chain: Vec<CausalLink>,
}

pub struct PackManifest {
    pub name: String,
    pub version: Version,
    pub description: String,
    pub log_sources: Vec<LogSource>,
    pub parser: ParserConfig,
    pub alerts: Vec<AlertRule>,
    pub dashboard: DashboardConfig,
    pub correlations: Vec<CorrelationRule>,
}
```

### logbog-collector

Gère la collecte de logs depuis toutes les sources.

**Trait principal :**
```rust
#[async_trait]
pub trait LogSource: Send + Sync {
    /// Nom de la source
    fn name(&self) -> &str;

    /// Stream de lignes de log brutes
    fn stream(&self) -> Pin<Box<dyn Stream<Item = RawLogLine> + Send>>;

    /// Position actuelle (pour bookmarking)
    fn position(&self) -> SourcePosition;

    /// Reprendre depuis une position
    fn seek(&mut self, pos: SourcePosition) -> Result<()>;
}
```

**Implémentations :**
- `FileSource` — surveillance de fichiers via inotify
- `JournalSource` — lecture du journal systemd
- `OtlpSource` — récepteur OTLP gRPC/HTTP
- `SyslogSource` — serveur syslog UDP/TCP

### logbog-parser

Moteur de parsing universel, configurable par pack.

```rust
pub trait Parser: Send + Sync {
    fn parse(&self, raw: &str) -> Result<ParsedLog>;
    fn format_name(&self) -> &str;
}

// Implémentations
pub struct RegexParser { patterns: Vec<Regex> }
pub struct GrokParser { patterns: Vec<GrokPattern> }
pub struct JsonParser { field_mappings: HashMap<String, String> }
pub struct SyslogParser { rfc: SyslogRfc }  // 3164 ou 5424
pub struct LogfmtParser;
pub struct MultilineParser { start_pattern: Regex, inner: Box<dyn Parser> }
```

### logbog-storage

Abstraction de stockage avec implémentation pluggable.

```rust
#[async_trait]
pub trait LogStore: Send + Sync {
    async fn insert_batch(&self, entries: &[LogEntry]) -> Result<usize>;
    async fn query(&self, sql: &str) -> Result<QueryResult>;
    async fn count(&self, filter: &LogFilter) -> Result<u64>;
    async fn delete_before(&self, timestamp: DateTime<Utc>, source: Option<&str>) -> Result<u64>;
    async fn stats(&self) -> Result<StorageStats>;
}

// Implémentations
pub struct DuckDbStore { /* hot tier */ }
pub struct ParquetStore { /* warm tier, S3 */ }
pub struct FederatedStore { hot: DuckDbStore, warm: ParquetStore }
```

### logbog-engine

Corrélation et intelligence.

```rust
pub struct CorrelationEngine {
    rules: Vec<CorrelationRule>,
    window: Duration,
    buffer: CorrelationBuffer,
}

impl CorrelationEngine {
    /// Analyse un batch de logs et retourne les corrélations trouvées
    pub fn correlate(&mut self, entries: &[LogEntry]) -> Vec<Correlation>;

    /// Construit une timeline d'incident
    pub fn build_timeline(&self, correlation: &Correlation) -> IncidentTimeline;
}

pub struct AnomalyDetector {
    baselines: HashMap<String, Baseline>,
    learning_window: Duration,
}

pub struct ErrorClusterer {
    clusters: Vec<ErrorCluster>,
    threshold: f64,  // Seuil de similarité
}
```

---

## Communication inter-composants

Les composants communiquent via des **channels async** (tokio mpsc/broadcast) :

```
Collector ──mpsc──▶ Parser ──mpsc──▶ Storage
                       │
                       ├──broadcast──▶ Correlation Engine
                       ├──broadcast──▶ Anomaly Detector
                       └──broadcast──▶ Tantivy Indexer
```

Cela permet :
- Le traitement parallèle (parsing et stockage en pipeline)
- Le backpressure naturel (channel bounded)
- L'ajout de consumers sans modifier le producer

---

## Format de configuration (logbog.toml)

```toml
[server]
host = "0.0.0.0"
port = 6060
data_dir = "/var/lib/logbog"

[storage]
engine = "duckdb"  # "duckdb" | "parquet" | "federated"
retention_days = 30
max_size_gb = 10

[storage.s3]  # Optionnel, pour le warm tier
endpoint = "http://localhost:9000"
bucket = "logbog-logs"
access_key = "minioadmin"
secret_key = "minioadmin"

[collector]
batch_size = 1000
flush_interval_ms = 500

[collector.otlp]
enabled = false
grpc_port = 4317
http_port = 4318

[collector.syslog]
enabled = false
udp_port = 5514
tcp_port = 5514

[correlation]
enabled = true
window_seconds = 2
min_confidence = 0.7

[intelligence]
anomaly_detection = true
error_clustering = true
learning_window_days = 7

[intelligence.llm]
enabled = false
provider = "ollama"  # "ollama" | "openai" | "anthropic"
model = "llama3"
endpoint = "http://localhost:11434"

[alerting]
enabled = true

[[alerting.channels]]
type = "webhook"
url = "https://hooks.slack.com/..."

[[alerting.channels]]
type = "email"
smtp_host = "smtp.example.com"
smtp_port = 587
from = "logbog@example.com"
to = ["admin@example.com"]

[compliance]
pii_detection = false
pii_masking = "redact"  # "redact" | "hash" | "pseudonymize"
audit_log = false

# Les packs installés sont auto-détectés dans data_dir/packs/
```

---

## Format d'un Log Pack (pack.toml)

```toml
[pack]
name = "nginx"
version = "1.0.0"
description = "Nginx access and error log parser"
author = "LogBog Team"
license = "Apache-2.0"
tags = ["web", "reverse-proxy", "http"]

[[sources]]
name = "access"
paths = [
    "/var/log/nginx/access.log",
    "/var/log/nginx/*-access.log"
]
format = "grok"
pattern = '%{IPORHOST:client_ip} - %{DATA:remote_user} \[%{HTTPDATE:timestamp}\] "%{WORD:method} %{URIPATHPARAM:uri} HTTP/%{NUMBER:http_version}" %{NUMBER:status:int} %{NUMBER:body_bytes:int} "%{DATA:referrer}" "%{DATA:user_agent}" %{NUMBER:request_time:float}'
timestamp_format = "%d/%b/%Y:%H:%M:%S %z"
multiline = false

[[sources]]
name = "error"
paths = [
    "/var/log/nginx/error.log",
    "/var/log/nginx/*-error.log"
]
format = "regex"
pattern = '(?P<timestamp>\d{4}/\d{2}/\d{2} \d{2}:\d{2}:\d{2}) \[(?P<level>\w+)\] (?P<pid>\d+)#(?P<tid>\d+): \*(?P<cid>\d+) (?P<message>.*)'
timestamp_format = "%Y/%m/%d %H:%M:%S"
multiline = false

[schema]
fields = [
    { name = "client_ip", type = "ip", indexed = true },
    { name = "method", type = "string", indexed = true },
    { name = "uri", type = "string", indexed = true },
    { name = "status", type = "int", indexed = true },
    { name = "body_bytes", type = "int", indexed = false },
    { name = "request_time", type = "float", indexed = false },
    { name = "user_agent", type = "string", indexed = false },
]

[[alerts]]
name = "5xx_spike"
description = "Spike of 5xx errors"
condition = "count(status >= 500) > 10 in 1m"
severity = "critical"
cooldown = "5m"

[[alerts]]
name = "high_latency"
description = "High request latency"
condition = "avg(request_time) > 5.0 in 5m"
severity = "warning"
cooldown = "10m"

[[correlations]]
target_pack = "php-fpm"
match_fields = ["timestamp", "uri:script"]
window = "2s"
description = "Correlate nginx request with PHP-FPM processing"
```

---

## Dépendances Rust principales

| Crate | Usage | Version |
|-------|-------|---------|
| `tokio` | Runtime async | 1.x |
| `clap` | CLI parsing | 4.x |
| `serde` / `serde_json` | Serialization | 1.x |
| `toml` | Config parsing | 0.8.x |
| `tracing` | Logging interne | 0.1.x |
| `regex` | Parsing regex | 1.x |
| `chrono` | Timestamps | 0.4.x |
| `duckdb` | Stockage SQL | 1.x |
| `tantivy` | Full-text search | 0.22.x |
| `axum` | HTTP API | 0.7.x |
| `tonic` | gRPC (OTLP) | 0.12.x |
| `tokio-tungstenite` | WebSocket | 0.x |
| `notify` | File watcher | 6.x |
| `ulid` | IDs uniques | 1.x |
| `arrow` / `parquet` | Format columnar | 53.x |
| `reqwest` | HTTP client | 0.12.x |

---

## Ports par défaut

| Service | Port | Protocole |
|---------|------|-----------|
| Dashboard web | 6060 | HTTP |
| API REST | 6060 | HTTP (même port) |
| OTLP gRPC | 4317 | gRPC |
| OTLP HTTP | 4318 | HTTP |
| Syslog UDP | 5514 | UDP |
| Syslog TCP | 5514 | TCP |
