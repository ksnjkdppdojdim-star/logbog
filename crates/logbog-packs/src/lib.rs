//! Framework de Log Packs pour LogBog.
//!
//! Gère le chargement, la validation et l'installation des packs.

pub mod manifest;
pub mod registry;

pub use manifest::PackManifest;
pub use registry::PackRegistry;
