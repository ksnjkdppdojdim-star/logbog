//! Moteur de corrélation pour LogBog.
//!
//! Extrait les identifiants des logs, les corrèle dans une fenêtre temporelle,
//! et détecte les chaînes causales (502, OOM, etc.).

pub mod chains;
pub mod correlator;
pub mod extractor;
pub mod rules;

pub use chains::{CausalChain, ChainEntry, ChainRole};
pub use correlator::Correlator;
pub use extractor::{ExtractedIds, MatchField, extract};
pub use rules::{CorrelationRule, parse_rule};
