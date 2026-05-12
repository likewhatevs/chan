//! Editor preferences: the fields that don't have another home.
//!
//! Most "preferences" the Settings UI surfaces already live in
//! existing config files:
//!
//! - `assistant`: chan-llm's LlmConfig (backend, model,
//!   auto_apply_writes, keys)
//! - `attachments_dir`: ServerConfig
//! - `answers_dir`: ServerConfig, mirrored into the assistant
//!   subtree of the unified view because the frontend types it as
//!   a sibling of `backend`
//! - `default_drive_root`: chan-drive's Registry (config.toml)
//! - `drives`: chan-drive's Registry
//!
//! What's left lives here, persisted to
//! `<config>/chan/preferences.toml`:
//!
//!   - fonts (per-role family + size)
//!   - theme  (system / light / dark)
//!   - pane_widths (inspector / graph / file-browser sidebars)
//!   - line_spacing (tight / standard)
//!   - date_format (id; UI-side mapping in dateFormats.ts)
//!
//! The Preferences view returned over /api/drive and /api/config is
//! assembled in lib.rs by joining EditorPrefs with the LlmConfig and
//! ServerConfig stores. PATCH /api/config splits the incoming body
//! the same way: edits land in whichever store owns the field.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::Error;

/// Fields persisted to `<config>/chan/preferences.toml`. Default
/// values are chosen to match the frontend's compiled-in defaults
/// in `web/src/state/fontPrefs.ts` so a fresh install renders
/// identically before the user touches anything.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EditorPrefs {
    #[serde(default)]
    pub fonts: FontPrefs,
    #[serde(default)]
    pub theme: ThemeChoice,
    #[serde(default)]
    pub pane_widths: PaneWidths,
    #[serde(default)]
    pub line_spacing: LineSpacing,
    #[serde(default = "default_date_format")]
    pub date_format: String,
}

impl Default for EditorPrefs {
    fn default() -> Self {
        Self {
            fonts: FontPrefs::default(),
            theme: ThemeChoice::default(),
            pane_widths: PaneWidths::default(),
            line_spacing: LineSpacing::default(),
            date_format: default_date_format(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FontSpec {
    pub family: String,
    pub size: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FontPrefs {
    pub heading1: FontSpec,
    pub heading2: FontSpec,
    pub heading3: FontSpec,
    pub normal: FontSpec,
    pub code: FontSpec,
    pub quote: FontSpec,
}

impl Default for FontPrefs {
    fn default() -> Self {
        // Match web/src/state/fontPrefs.ts DEFAULT_FONT_PREFS so a
        // fresh server returns the same defaults the frontend's
        // pre-fetch fallback would draw with.
        let body = "-apple-system, BlinkMacSystemFont, \"Segoe UI\", Roboto, sans-serif";
        let mono = "ui-monospace, SFMono-Regular, Menlo, Consolas, monospace";
        Self {
            heading1: FontSpec {
                family: body.into(),
                size: 32,
            },
            heading2: FontSpec {
                family: body.into(),
                size: 24,
            },
            heading3: FontSpec {
                family: body.into(),
                size: 20,
            },
            normal: FontSpec {
                family: body.into(),
                size: 16,
            },
            code: FontSpec {
                family: mono.into(),
                size: 13,
            },
            quote: FontSpec {
                family: body.into(),
                size: 16,
            },
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemeChoice {
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaneWidths {
    pub inspector: u32,
    pub graph: u32,
    pub browser: u32,
    // Per-field default so older preferences.toml (written before the
    // search inspector got its own width slot) load cleanly.
    #[serde(default = "default_search_width")]
    pub search: u32,
}

impl Default for PaneWidths {
    // Mirrors web/src/state/store.svelte.ts DEFAULT_PANE_WIDTHS.
    fn default() -> Self {
        Self {
            inspector: 220,
            graph: 260,
            browser: 240,
            search: default_search_width(),
        }
    }
}

fn default_search_width() -> u32 {
    280
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LineSpacing {
    #[default]
    Tight,
    Standard,
}

fn default_date_format() -> String {
    // Matches the editor's `dateFormats.ts` default.
    "iso".into()
}

impl EditorPrefs {
    pub fn load() -> Result<Self, Error> {
        Self::load_from(&default_path())
    }

    pub fn load_from(path: &Path) -> Result<Self, Error> {
        crate::store::load_toml(path)
    }

    pub fn save(&self) -> Result<(), Error> {
        self.save_to(&default_path())
    }

    pub fn save_to(&self, path: &Path) -> Result<(), Error> {
        crate::store::save_toml(path, self)
    }
}

/// `~/.chan/preferences.toml` on desktop. iOS / Android pass an
/// explicit path via `load_from` / `save_to` since their sandbox
/// dir isn't `chan_drive::paths::config_dir`.
pub fn default_path() -> PathBuf {
    chan_drive::paths::config_dir().join("preferences.toml")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn default_round_trips() {
        let tmp = TempDir::new().unwrap();
        let p = tmp.path().join("preferences.toml");
        let prefs = EditorPrefs::default();
        prefs.save_to(&p).unwrap();
        let loaded = EditorPrefs::load_from(&p).unwrap();
        assert_eq!(prefs, loaded);
    }

    #[test]
    fn missing_file_returns_default() {
        let tmp = TempDir::new().unwrap();
        let prefs = EditorPrefs::load_from(&tmp.path().join("nope.toml")).unwrap();
        assert_eq!(prefs, EditorPrefs::default());
    }

    #[test]
    fn partial_file_fills_defaults() {
        let tmp = TempDir::new().unwrap();
        let p = tmp.path().join("preferences.toml");
        std::fs::write(&p, "date_format = \"long\"\n").unwrap();
        let prefs = EditorPrefs::load_from(&p).unwrap();
        assert_eq!(prefs.date_format, "long");
        assert_eq!(prefs.theme, ThemeChoice::System);
    }

    #[test]
    fn pane_widths_legacy_file_fills_search_default() {
        // Regression: preferences.toml written before the search
        // inspector got its own width slot must still load.
        let tmp = TempDir::new().unwrap();
        let p = tmp.path().join("preferences.toml");
        std::fs::write(
            &p,
            "[pane_widths]\ninspector = 240\ngraph = 300\nbrowser = 250\n",
        )
        .unwrap();
        let prefs = EditorPrefs::load_from(&p).unwrap();
        assert_eq!(prefs.pane_widths.inspector, 240);
        assert_eq!(prefs.pane_widths.graph, 300);
        assert_eq!(prefs.pane_widths.browser, 250);
        assert_eq!(prefs.pane_widths.search, default_search_width());
    }

    #[test]
    fn theme_serializes_lowercase() {
        let prefs = EditorPrefs {
            theme: ThemeChoice::Dark,
            ..Default::default()
        };
        let s = toml::to_string(&prefs).unwrap();
        assert!(s.contains("theme = \"dark\""), "got: {s}");
    }
}
