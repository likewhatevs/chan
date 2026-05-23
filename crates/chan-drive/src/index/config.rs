// Persisted config for the search index. Lives at
// `<index_dir>/config.toml` (per-drive global cache; see
// `crate::paths::drive_paths`) and stores the embedding model id,
// the chunking strategy, and a schema version that triggers a full
// rebuild when bumped.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::error::ChanError;

/// Default embedding model. Small (~130 MB), English-only, fast.
pub const DEFAULT_MODEL: &str = "BAAI/bge-small-en-v1.5";

/// Current on-disk schema version. Bumping this forces a rebuild on
/// next index open; the indexer compares against the value loaded
/// from disk and clears `bm25/` + `embeddings/` on mismatch.
///
/// v1: per-heading chunking with hybrid (BM25 + dense) retrieval,
///     replacing the old per-file BM25 schema.
/// v2: candle-backed embedder replaces fastembed/ort. Vector tensors
///     are byte-compatible (same f32 layout, same BGE checkpoints)
///     but small numerical drift between ONNX and the pure-Rust
///     transformer kernel can shift cosine scores by epsilon, so
///     wipe and re-embed on first open after upgrade.
/// v3: indexer widened from `.md`-only to every `FileClass::EditableText`
///     extension (today: `.md` + `.txt`). Existing indices were
///     `.md`-only and would miss `.txt` content; a wipe-and-rebuild
///     populates them.
pub const SCHEMA_VERSION: u32 = 3;

/// How a markdown file is split into indexable units.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Chunking {
    /// One chunk per ATX heading section. Files without headings
    /// fall back to a single whole-doc chunk.
    #[default]
    Headings,
    /// One chunk per file, no splitting.
    WholeDoc,
    /// Fixed-window chunks of N characters with no overlap.
    Fixed { chars: usize },
}

/// Visual theme rendered behind the screensaver unlock card.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ScreensaverTheme {
    #[default]
    Plain,
    Matrix,
    Castaway,
}

/// On-disk shape of `<index_dir>/config.toml`.
///
/// `model` is the user-configured target: the model the indexer
/// should use for new embeddings. `vectors_model` and `vectors_dim`
/// describe what produced the vectors *currently on disk*. The two
/// can diverge when the user changes the target model (via
/// `Index::set_model` or by hand-editing this file) but the vector
/// store hasn't been rebuilt yet. `Index::open` detects that
/// divergence and wipes `embeddings/` so the next reindex repopulates
/// the store against the new model.
///
/// Keeping the "what's on disk" stamp separate from "what's
/// configured" closes a silent-corruption window: just trusting
/// `model` would let a config edit produce a vector store mixing
/// outputs from two different models, or feeding query vectors of
/// one dim against doc vectors of another.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexConfig {
    pub schema_version: u32,
    pub model: String,
    #[serde(default)]
    pub chunking: Chunking,
    /// Model id that produced the vectors currently on disk. `None`
    /// means the vector store is empty (post-wipe, fresh install,
    /// or never-embedded). On `Index::open`, a mismatch against
    /// `model` triggers an embedding-only wipe.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vectors_model: Option<String>,
    /// Dim of the vectors currently on disk. Stamped alongside
    /// `vectors_model` at the end of every embed pass. Used by
    /// `build_all` as a defensive cross-check against the live
    /// embedder's `dim()` before writing more shards.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vectors_dim: Option<u32>,
    /// systacean-7: per-drive Hybrid-search opt-in. Default-false so
    /// drives stay BM25-only after the systacean-6 model split unless
    /// the user explicitly flips it on via
    /// `chan index enable-semantic` (CLI) or the Settings UI
    /// (`fullstack-a-21`). The query path reads this flag to decide
    /// whether Hybrid is the default mode for the drive; explicit
    /// `Mode::Hybrid` overrides on `search` still work regardless.
    #[serde(default)]
    pub semantic_enabled: bool,
    /// systacean-27: per-drive chan-report opt-in (Round-2 pre-flight
    /// feature toggle). Default-false so drives stay lean (no
    /// language-detection scan, no SLOC roll-up, no COCOMO). When
    /// true, `Drive::report()` initializes the per-drive
    /// `ReportState` + the watcher fanout keeps it current. Lives
    /// alongside `semantic_enabled` for symmetry; both toggles
    /// persist in the per-drive `IndexConfig` (Round-3 may refactor
    /// to a separate `features.toml` if/when more flags accumulate).
    #[serde(default)]
    pub reports_enabled: bool,
    /// systacean-40: screensaver overlay opt-in. Default-false so
    /// drives without the feature configured stay unchanged. SPA
    /// reads `Drive::screensaver_enabled()` via the
    /// `/api/screensaver/state` endpoint + arms the overlay
    /// state machine when true.
    #[serde(default)]
    pub screensaver_enabled: bool,
    /// systacean-40: idle window in seconds before the screensaver
    /// overlay fires. Default 300 (5 minutes). The SPA computes
    /// "idle" client-side from last keystroke / pointer activity;
    /// chan-server just persists the threshold.
    #[serde(default = "default_screensaver_timeout_secs")]
    pub screensaver_timeout_secs: u32,
    /// fullstack-a-99: visual theme for the screensaver overlay.
    /// Default plain keeps the lock screen quiet unless the user opts
    /// into an animated scene.
    #[serde(default)]
    pub screensaver_theme: ScreensaverTheme,
    /// systacean-40: per-drive PIN hash. `None` when no PIN is
    /// set (overlay still arms but auto-dismisses on any input).
    /// The bytes are whatever the SPA POSTs — chan-server stores
    /// without interpretation; the verify path is a byte-equality
    /// compare. PBKDF2 happens client-side per `-a-77`.
    ///
    /// NEVER serialized back over the wire in plaintext: the
    /// `/api/screensaver/state` endpoint reports `pin_set: bool`
    /// only.
    #[serde(default, with = "screensaver_pin_serde")]
    pub screensaver_pin_hash: Option<Vec<u8>>,
}

fn default_screensaver_timeout_secs() -> u32 {
    300
}

/// systacean-40: serde adapter for `screensaver_pin_hash`. We
/// persist as base64 in the TOML so the file stays text-only +
/// the bytes round-trip cleanly (raw `Vec<u8>` would land as a
/// TOML array of integers — readable, but noisy + harder for
/// future migrations to read).
mod screensaver_pin_serde {
    use base64::Engine;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(value: &Option<Vec<u8>>, ser: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match value {
            Some(bytes) => {
                let b64 = base64::engine::general_purpose::STANDARD.encode(bytes);
                ser.serialize_some(&b64)
            }
            None => ser.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(de: D) -> Result<Option<Vec<u8>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let opt = Option::<String>::deserialize(de)?;
        match opt {
            Some(s) => base64::engine::general_purpose::STANDARD
                .decode(s.as_bytes())
                .map(Some)
                .map_err(serde::de::Error::custom),
            None => Ok(None),
        }
    }
}

impl Default for IndexConfig {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            model: DEFAULT_MODEL.to_owned(),
            chunking: Chunking::default(),
            vectors_model: None,
            vectors_dim: None,
            semantic_enabled: false,
            reports_enabled: false,
            screensaver_enabled: false,
            screensaver_timeout_secs: default_screensaver_timeout_secs(),
            screensaver_theme: ScreensaverTheme::Plain,
            screensaver_pin_hash: None,
        }
    }
}

// `toml::de::Error` and `toml::ser::Error` are intentionally
// boxed here. Both carry inline message buffers + span info that
// push their stack size on the Windows target past clippy's
// `result-large-err` 128-byte threshold; boxing brings the
// `ConfigError` variant down to a single pointer so every
// `Result<_, ConfigError>` return site stays under the lint.
// Linux + macOS don't trip the lint (different stack alignment
// and intrinsic type sizes there), but the boxing also reduces
// the size of the Ok variant's stack slot on every platform.
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("decode {path}: {source}")]
    Decode {
        path: PathBuf,
        #[source]
        source: Box<toml::de::Error>,
    },
    #[error(transparent)]
    Encode(Box<toml::ser::Error>),
    #[error(transparent)]
    Chan(#[from] ChanError),
}

// Manual `From` for the encoder side: `#[from]` on a `Box<_>`
// field would generate `From<Box<toml::ser::Error>>`, which
// breaks `?` at the `toml::to_string_pretty(...)` call site.
// Wrap the bare error in a `Box` at the boundary so `?`
// continues to compile unchanged.
impl From<toml::ser::Error> for ConfigError {
    fn from(e: toml::ser::Error) -> Self {
        Self::Encode(Box::new(e))
    }
}

/// Path to the index config inside `index_dir`.
pub fn config_path(index_dir: &Path) -> PathBuf {
    index_dir.join("config.toml")
}

/// Load the config, falling back to defaults if the file is absent.
/// A malformed file is an error; we don't silently overwrite a
/// user's edit.
pub fn load(index_dir: &Path) -> Result<IndexConfig, ConfigError> {
    let path = config_path(index_dir);
    if !path.exists() {
        return Ok(IndexConfig::default());
    }
    let raw = std::fs::read_to_string(&path)?;
    toml::from_str(&raw).map_err(|source| ConfigError::Decode {
        path,
        source: Box::new(source),
    })
}

/// Persist the config. Creates the parent directory if needed.
pub fn save(index_dir: &Path, cfg: &IndexConfig) -> Result<(), ConfigError> {
    let path = config_path(index_dir);
    let body = toml::to_string_pretty(cfg)?;
    crate::fs_ops::atomic_write(&path, body.as_bytes())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn load_returns_default_when_absent() {
        let tmp = TempDir::new().unwrap();
        let cfg = load(tmp.path()).unwrap();
        assert_eq!(cfg.model, DEFAULT_MODEL);
        assert_eq!(cfg.schema_version, SCHEMA_VERSION);
        assert!(matches!(cfg.chunking, Chunking::Headings));
    }

    #[test]
    fn save_then_load_roundtrips() {
        let tmp = TempDir::new().unwrap();
        let cfg = IndexConfig {
            model: "BAAI/bge-m3".to_owned(),
            chunking: Chunking::Fixed { chars: 512 },
            ..IndexConfig::default()
        };
        save(tmp.path(), &cfg).unwrap();
        let loaded = load(tmp.path()).unwrap();
        assert_eq!(loaded.model, "BAAI/bge-m3");
        assert!(matches!(loaded.chunking, Chunking::Fixed { chars: 512 }));
    }

    #[test]
    fn semantic_enabled_defaults_false_and_round_trips_true() {
        // systacean-7: pin the per-drive Hybrid opt-in field. Default
        // matches post-systacean-6 behaviour (BM25-only) so an existing
        // drive whose config.toml predates this field stays BM25 on
        // upgrade. Round-tripping with the field set to true
        // verifies the toml shape is preserved across save/load.
        let tmp = TempDir::new().unwrap();
        let cfg = load(tmp.path()).unwrap();
        assert!(!cfg.semantic_enabled, "default must be false");

        let cfg = IndexConfig {
            semantic_enabled: true,
            ..IndexConfig::default()
        };
        save(tmp.path(), &cfg).unwrap();
        let loaded = load(tmp.path()).unwrap();
        assert!(loaded.semantic_enabled);
    }

    #[test]
    fn reports_enabled_defaults_false_and_round_trips_true() {
        // systacean-27: pin the chan-reports opt-in field. Default
        // matches Round-2's lean-drive baseline (off). Round-
        // tripping with the field set to true verifies the toml
        // shape is preserved across save/load. Backward-compat:
        // a pre-`-27` config.toml without the field loads with
        // reports_enabled = false.
        let tmp = TempDir::new().unwrap();
        let cfg = load(tmp.path()).unwrap();
        assert!(!cfg.reports_enabled, "default must be false");

        let cfg = IndexConfig {
            reports_enabled: true,
            ..IndexConfig::default()
        };
        save(tmp.path(), &cfg).unwrap();
        let loaded = load(tmp.path()).unwrap();
        assert!(loaded.reports_enabled);
        // Backward-compat: serialize manually without the field
        // (simulating a pre-`-27` on-disk file) + verify
        // deserialize defaults it to false. Chunking + the new
        // toggles all use serde defaults so the minimum file is
        // just schema_version + model.
        std::fs::write(
            config_path(tmp.path()),
            "schema_version = 1\nmodel = \"BAAI/bge-small-en-v1.5\"\n",
        )
        .unwrap();
        let legacy = load(tmp.path()).unwrap();
        assert!(!legacy.reports_enabled, "missing field defaults to false");
        assert!(!legacy.semantic_enabled, "missing field defaults to false");
    }

    #[test]
    fn screensaver_theme_plain_round_trips_as_plain() {
        let tmp = TempDir::new().unwrap();
        let cfg = IndexConfig {
            screensaver_theme: ScreensaverTheme::Plain,
            ..IndexConfig::default()
        };
        save(tmp.path(), &cfg).unwrap();

        let raw = std::fs::read_to_string(config_path(tmp.path())).unwrap();
        assert!(
            raw.contains("screensaver_theme = \"plain\""),
            "theme should serialize as plain: {raw}"
        );
        let loaded = load(tmp.path()).unwrap();
        assert_eq!(loaded.screensaver_theme, ScreensaverTheme::Plain);
    }

    #[test]
    fn load_works_while_drive_lock_is_held() {
        // systacean-8: chan index status reads IndexConfig without
        // opening a Drive (so no writer lock acquired), which means
        // a running `chan serve` against the drive no longer blocks
        // the CLI. Pin the invariant: `config::load` doesn't touch
        // any lock file and returns successfully even while another
        // holder (simulating the chan serve process) has acquired
        // the per-drive writer flock.
        use crate::lock::DriveLock;
        let tmp = TempDir::new().unwrap();
        let index_dir = tmp.path().join("index");
        let lock_dir = tmp.path().join("lock");
        std::fs::create_dir_all(&index_dir).unwrap();
        let on_disk = IndexConfig {
            model: "BAAI/bge-m3".to_owned(),
            semantic_enabled: true,
            ..IndexConfig::default()
        };
        save(&index_dir, &on_disk).unwrap();
        let _holder = DriveLock::acquire(&lock_dir).unwrap();
        let loaded = load(&index_dir).unwrap();
        assert_eq!(loaded.model, "BAAI/bge-m3");
        assert!(loaded.semantic_enabled);
    }

    #[test]
    fn semantic_enabled_absent_in_old_file_loads_as_false() {
        // Existing drives whose config.toml predates systacean-7 don't
        // have the field at all. Pin that they load cleanly with the
        // default (false) rather than failing with a missing-field
        // error.
        let tmp = TempDir::new().unwrap();
        let path = config_path(tmp.path());
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(
            &path,
            "schema_version = 3\nmodel = \"BAAI/bge-small-en-v1.5\"\n",
        )
        .unwrap();
        let cfg = load(tmp.path()).unwrap();
        assert!(!cfg.semantic_enabled);
        assert_eq!(cfg.model, "BAAI/bge-small-en-v1.5");
    }

    #[test]
    fn malformed_is_error() {
        let tmp = TempDir::new().unwrap();
        let path = config_path(tmp.path());
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "this is not toml = =").unwrap();
        let err = load(tmp.path()).unwrap_err();
        assert!(matches!(err, ConfigError::Decode { .. }));
    }
}
