# Agent Guidelines for rsfb

This repository contains `rsfb` (Rust Shakes & Fidget Bot), an asynchronous Rust automation tool.

## Configuration Synchronization (Mandatory)

**`src/constant.rs` and `rsfb.toml` MUST always be kept strictly in sync.**

1. **Role of `src/constant.rs`**:
   - Contains the fallback compile-time constants used by `Config::default()` in `src/config.rs`.
   - Used when no configuration file exists or when specific keys are omitted.

2. **Role of `rsfb.toml`**:
   - The canonical example and template configuration file shipped in the repository root.
   - Documents every available configuration key, its data type, and its default value with descriptive comments.

3. **Rule for Changes**:
   - Whenever any configuration setting, feature flag, limit, ratio, or interval is added, modified, renamed, or deleted:
     - Update the constant in `src/constant.rs`.
     - Update the key, description, and default value in `rsfb.toml`.
     - Update the corresponding struct and `Default` implementation in `src/config.rs`.
     - Ensure the default values in `src/constant.rs` and `rsfb.toml` are identical.

## Code Conventions & Formatting

- **File-Level Config Extraction**:
  Module files extract config values at the top of the file as `static LazyLock` variables (e.g. `static MIN_FREE_SLOTS: LazyLock<usize> = LazyLock::new(|| CONFIG.inventory.min_free_slots);`).
- **Single-Line Extractions**:
  Variable names and config paths must be kept concise enough that each static declaration fits on a single line without wrapping (within the 100-character line width limit).
- **Verification**:
  Always verify changes by running:
  - `cargo check`
  - `cargo test`
  - `cargo fmt --check`
