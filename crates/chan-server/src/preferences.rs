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
//!   - editor_theme (github / google_docs / word)
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

/// Fields persisted to `<config>/chan/preferences.toml`. A fresh
/// install defaults to the GitHub editor theme so the renderer
/// matches what most users expect from a markdown editor without
/// any further configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EditorPrefs {
    #[serde(default)]
    pub editor_theme: EditorTheme,
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
            editor_theme: EditorTheme::default(),
            theme: ThemeChoice::default(),
            pane_widths: PaneWidths::default(),
            line_spacing: LineSpacing::default(),
            date_format: default_date_format(),
        }
    }
}

/// Editor theme. Drives the markdown renderer + source view
/// typography and chrome (headings, body, code blocks, blockquotes,
/// tables). Light/dark variants are picked from the active
/// `ThemeChoice`; density from `LineSpacing`. App chrome
/// (toolbar, panes, status bar) is not affected.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EditorTheme {
    #[default]
    Github,
    GoogleDocs,
    Word,
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
    // Per-field default so older preferences.toml (written before the
    // outline pane was split out of the right-side inspector) load
    // cleanly. Width of the left-side outline pane in the file editor.
    #[serde(default = "default_outline_width")]
    pub outline: u32,
}

impl Default for PaneWidths {
    // Mirrors web/src/state/store.svelte.ts DEFAULT_PANE_WIDTHS.
    fn default() -> Self {
        Self {
            inspector: 220,
            graph: 260,
            browser: 240,
            search: default_search_width(),
            outline: default_outline_width(),
        }
    }
}

fn default_search_width() -> u32 {
    280
}

fn default_outline_width() -> u32 {
    220
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
        assert_eq!(prefs.pane_widths.outline, default_outline_width());
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

    #[test]
    fn editor_theme_defaults_to_github_and_serializes_snake_case() {
        let prefs = EditorPrefs::default();
        assert_eq!(prefs.editor_theme, EditorTheme::Github);
        let prefs = EditorPrefs {
            editor_theme: EditorTheme::GoogleDocs,
            ..Default::default()
        };
        let s = toml::to_string(&prefs).unwrap();
        assert!(s.contains("editor_theme = \"google_docs\""), "got: {s}");
    }
}
