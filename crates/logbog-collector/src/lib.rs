//! Couche de collecte de logs pour LogBog.
//!
//! Implémente la surveillance de fichiers, la lecture du journal systemd,
//! et les récepteurs OTLP/syslog.

use logbog_core::RawLogLine;
use tokio::sync::mpsc;

/// Canal de transmission des lignes de log brutes.
pub type LogSender = mpsc::Sender<RawLogLine>;
pub type LogReceiver = mpsc::Receiver<RawLogLine>;

/// Crée un canal de collecte avec la capacité spécifiée.
pub fn channel(capacity: usize) -> (LogSender, LogReceiver) {
    mpsc::channel(capacity)
}

/// Trait pour les sources de logs.
pub trait LogSource: Send + Sync {
    /// Nom de la source.
    fn name(&self) -> &str;
}
