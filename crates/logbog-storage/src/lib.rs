//! Couche d'abstraction de stockage pour LogBog.
//!
//! Phase 2+ — structure de base posée, implémentation DuckDB à venir.

/// Placeholder pour le stockage.
pub struct LogStore;

impl LogStore {
    pub fn new() -> Self {
        Self
    }
}

impl Default for LogStore {
    fn default() -> Self {
        Self::new()
    }
}
