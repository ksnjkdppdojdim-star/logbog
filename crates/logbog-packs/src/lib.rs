//! Framework de Log Packs pour LogBog.
//!
//! Gère le chargement, la validation, l'installation et l'exécution des packs.

pub mod engine;
pub mod manifest;
pub mod registry;
pub mod remote;

pub use engine::PackEngine;
pub use manifest::PackManifest;
pub use registry::PackRegistry;
pub use remote::RemoteRegistry;
