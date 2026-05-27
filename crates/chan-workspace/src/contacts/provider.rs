// ProviderKind: parser dispatch tag, not an API client. Each
// variant maps to a parser module under `contacts::`. Adding a
// provider is: define the variant here, write a parser module
// that returns `Vec<Contact>`, route from `Workspace::import_contacts`.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    #[default]
    Google,
}

impl ProviderKind {
    /// Lowercase wire form used in HTTP requests, frontmatter, and
    /// CLI flags. Matches the serde rename.
    pub fn as_str(self) -> &'static str {
        match self {
            ProviderKind::Google => "google",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "google" => Some(ProviderKind::Google),
            _ => None,
        }
    }
}

impl std::fmt::Display for ProviderKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}
