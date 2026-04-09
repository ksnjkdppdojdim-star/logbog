# Contribuer à LogBog

Merci de votre intérêt pour LogBog ! Ce guide explique comment contribuer au projet.

---

## Prérequis

- **Rust** : version stable >= 1.78 (via [rustup](https://rustup.rs/))
- **Docker** & **Docker Compose** : pour l'environnement de test
- **Git** : configuré avec vos identifiants

```bash
# Vérifier les prérequis
rustc --version
cargo --version
docker --version
```

---

## Mise en place de l'environnement de développement

```bash
# Cloner le projet
git clone https://github.com/ksnjkdppdojdim-star/logbog.git
cd logbog

# Compiler le projet
cargo build

# Lancer les tests
cargo test

# Lancer le linting
cargo clippy -- -D warnings

# Formatter le code
cargo fmt

# Lancer l'environnement de test (génère des logs)
docker-compose -f deploy/docker/docker-compose.dev.yml up -d
```

---

## Structure du projet

```
crates/          → Code source Rust (un crate par composant)
packs/           → Log Packs officiels
web/             → Dashboard SvelteKit
tests/           → Tests d'intégration et E2E
docs/            → Documentation
deploy/          → Fichiers de déploiement
scripts/         → Scripts utilitaires
```

---

## Conventions de code

### Rust
- Format : `cargo fmt` (configuration dans `rustfmt.toml`)
- Linting : `cargo clippy -- -D warnings` (zéro warning toléré)
- Tests : chaque module public doit avoir des tests unitaires
- Documentation : les types et fonctions publics doivent avoir des doc-comments (`///`)
- Erreurs : utiliser `thiserror` pour les types d'erreur, `anyhow` dans les binaires

### Commits
- Format : `type(scope): description`
- Types : `feat`, `fix`, `docs`, `refactor`, `test`, `chore`, `perf`
- Exemples :
  - `feat(packs): add nginx access log parser`
  - `fix(collector): handle log rotation correctly`
  - `docs(readme): update installation instructions`

### Branches
- `main` : branche stable, toujours déployable
- `dev` : branche de développement active
- `feat/xxx` : features en cours
- `fix/xxx` : corrections de bugs

---

## Créer un Log Pack

Voir le [Guide de création de packs](packs/PACK_GUIDE.md) pour les instructions détaillées.

En résumé :
1. Créer un répertoire dans `packs/<nom>/`
2. Écrire le `pack.toml` avec les sources, parseurs et alertes
3. Ajouter des fichiers de test dans `packs/<nom>/testdata/`
4. Écrire les tests : `cargo test -p logbog-packs -- <nom>`
5. Soumettre une PR

---

## Pull Requests

1. Forker le repo et créer une branche depuis `main`
2. Écrire du code avec des tests
3. S'assurer que `cargo test`, `cargo clippy` et `cargo fmt --check` passent
4. Ouvrir une PR avec une description claire
5. Attendre la review

---

## Signaler un bug

Ouvrir une issue sur GitHub avec :
- Description du problème
- Étapes pour reproduire
- Logs pertinents (avec `LOGBOG_LOG=debug logbog ...`)
- Environnement (OS, version de LogBog)

---

## Licence

En contribuant, vous acceptez que vos contributions soient sous licence Apache 2.0.
