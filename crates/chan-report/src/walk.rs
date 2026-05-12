// Walker over the configured root.
//
// Wraps the `ignore` crate so consumers get gitignore-aware,
// hidden-file-aware traversal with the chan-report defaults
// applied. Emits drive-relative POSIX paths the counter can
// consume directly.
//
// `Filter` caches the gitignore + override matchers so
// `Index::update` can reapply them per file without rebuilding
// the matchers on every event.

use ignore::gitignore::Gitignore;
use ignore::overrides::{Override, OverrideBuilder};
use ignore::WalkBuilder;

use crate::error::ChanReportError;
use crate::ReportOptions;

/// Cached accept-filter used by both the initial walk and the
/// incremental `Index::update` path. Built once at scan time.
pub(crate) struct Filter {
    include_hidden: bool,
    overrides: Option<Override>,
    gitignore: Option<Gitignore>,
}

impl Filter {
    pub(crate) fn build(opts: &ReportOptions) -> Result<Self, ChanReportError> {
        let overrides = if opts.exclude_globs.is_empty() {
            None
        } else {
            let mut b = OverrideBuilder::new(&opts.root);
            for g in &opts.exclude_globs {
                // ignore::Override patterns are gitignore-style but
                // semantically inverted: unprefixed = whitelist,
                // `!` = ignore. exclude_globs is an exclude list,
                // so prefix every entry with `!`. Strip any caller-
                // provided leading `!` first so double-negation
                // does not flip the meaning.
                let pat = format!("!{}", g.trim_start_matches('!'));
                b.add(&pat)
                    .map_err(|e| ChanReportError::Walk(e.to_string()))?;
            }
            Some(
                b.build()
                    .map_err(|e| ChanReportError::Walk(e.to_string()))?,
            )
        };

        let gitignore = if opts.respect_gitignore {
            let p = opts.root.join(".gitignore");
            if p.exists() {
                let (gi, _maybe_err) = Gitignore::new(&p);
                // `Gitignore::new` may return parse warnings via the
                // second tuple element; we drop them. A malformed
                // .gitignore should not block the scan.
                Some(gi)
            } else {
                None
            }
        } else {
            None
        };

        Ok(Self {
            include_hidden: opts.include_hidden,
            overrides,
            gitignore,
        })
    }

    /// Returns true when `rel` (POSIX, drive-relative, no leading
    /// slash) should be tracked. Ancestors are checked as
    /// directories so gitignore rules like `target/` reject the
    /// whole subtree the way the walker would during descent.
    pub(crate) fn accepts(&self, rel: &str) -> bool {
        if !self.include_hidden {
            for part in rel.split('/') {
                if part.starts_with('.') {
                    return false;
                }
            }
        }
        let parts: Vec<&str> = rel.split('/').collect();
        for i in 0..parts.len() {
            let sub = parts[..=i].join("/");
            let is_dir = i + 1 < parts.len();
            if let Some(ov) = &self.overrides {
                if ov.matched(&sub, is_dir).is_ignore() {
                    return false;
                }
            }
            if let Some(gi) = &self.gitignore {
                if gi.matched(&sub, is_dir).is_ignore() {
                    return false;
                }
            }
        }
        true
    }
}

/// Walk the configured root and return every accepted relative
/// path. POSIX-style, no leading slash, no `..`. The walker's
/// own gitignore handling matches `Filter::accepts` for the root
/// `.gitignore`; nested ignore files take effect inside the walk
/// but are not reapplied by the cached filter.
pub(crate) fn walk_root(opts: &ReportOptions) -> Result<Vec<String>, ChanReportError> {
    let mut builder = WalkBuilder::new(&opts.root);
    builder
        .follow_links(opts.follow_symlinks)
        .hidden(!opts.include_hidden)
        .git_ignore(opts.respect_gitignore)
        .git_global(opts.respect_gitignore)
        .git_exclude(opts.respect_gitignore)
        .ignore(opts.respect_gitignore)
        .parents(opts.respect_gitignore);
    if opts.respect_gitignore {
        // The ignore crate only honors .gitignore files inside a
        // git repo (i.e. when a `.git/` directory is present).
        // chan-report's drives are not always git repos, so treat
        // any .gitignore we find as a regular ignore file too.
        // This is read in addition to (not instead of) git_ignore,
        // so nested .gitignore files inside a real repo continue
        // to work the way users expect.
        builder.add_custom_ignore_filename(".gitignore");
    }

    if !opts.exclude_globs.is_empty() {
        let mut ob = OverrideBuilder::new(&opts.root);
        for g in &opts.exclude_globs {
            let pat = format!("!{}", g.trim_start_matches('!'));
            ob.add(&pat)
                .map_err(|e| ChanReportError::Walk(e.to_string()))?;
        }
        builder.overrides(
            ob.build()
                .map_err(|e| ChanReportError::Walk(e.to_string()))?,
        );
    }

    let walker = builder.build();
    let mut out = Vec::new();
    for entry in walker {
        let entry = entry.map_err(|e| ChanReportError::Walk(e.to_string()))?;
        if !entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
            continue;
        }
        let abs = entry.path();
        let rel = abs
            .strip_prefix(&opts.root)
            .map_err(|_| ChanReportError::PathEscapesRoot(abs.display().to_string()))?;
        let rel_str = rel
            .to_str()
            .ok_or_else(|| ChanReportError::InvalidUtf8Path(rel.display().to_string()))?
            .replace('\\', "/");
        out.push(rel_str);
    }
    Ok(out)
}
