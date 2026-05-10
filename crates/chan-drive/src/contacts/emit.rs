// Render a Contact as a markdown note.
//
// Frontmatter shape (YAML):
//
//   chan:
//     kind: contact
//     provider: google
//     imported_at: 2026-05-10T12:34:56Z
//     frontmatter_version: 1
//   contact:
//     display_name: Jane Q. Doe
//     given_name: Jane
//     family_name: Doe
//     emails:
//       - { value: jane@example.com, label: work }
//     ...
//
// We hand-format the YAML rather than pulling a serializer dep.
// The shape is small and fixed, escaping rules are mechanical, and
// adding `serde_yaml` would import an unmaintained crate.
//
// String quoting: double-quoted always, with `\\`, `\"`, `\n`,
// `\r`, `\t` escaped. Robust at the cost of slightly noisier
// output for plain ASCII names. Worth the simplicity.

use chrono::{DateTime, Utc};

use super::{Contact, EmailAddress, Organization, PhoneNumber};

/// Caller-supplied context. `imported_at` is plumbed in (rather
/// than read from the system clock) so tests can assert exact
/// output and so a single batch shares one timestamp.
#[derive(Debug, Clone)]
pub struct EmitContext {
    pub imported_at: DateTime<Utc>,
}

const FRONTMATTER_VERSION: u32 = 1;

pub fn render_markdown(c: &Contact, ctx: &EmitContext) -> String {
    let mut s = String::with_capacity(512);
    s.push_str("---\n");
    write_chan_block(&mut s, c, ctx);
    write_contact_block(&mut s, c);
    s.push_str("---\n\n");
    s.push_str("# ");
    s.push_str(c.display_name.trim());
    s.push('\n');
    if let Some(notes) = c.notes.as_ref() {
        let notes = notes.trim();
        if !notes.is_empty() {
            s.push('\n');
            s.push_str(notes);
            if !notes.ends_with('\n') {
                s.push('\n');
            }
        }
    }
    s
}

fn write_chan_block(s: &mut String, c: &Contact, ctx: &EmitContext) {
    s.push_str("chan:\n");
    s.push_str("  kind: contact\n");
    s.push_str("  provider: ");
    s.push_str(c.provider.as_str());
    s.push('\n');
    s.push_str("  imported_at: ");
    // RFC3339 with second precision; chrono's `to_rfc3339_opts`
    // would let us pin the format, but the default is good enough
    // and round-trips through gray_matter / serde_yaml parsers.
    s.push_str(&ctx.imported_at.to_rfc3339());
    s.push('\n');
    s.push_str("  frontmatter_version: ");
    s.push_str(&FRONTMATTER_VERSION.to_string());
    s.push('\n');
    if let Some(rid) = c.remote_id.as_ref() {
        s.push_str("  remote_id: ");
        s.push_str(&yaml_string(rid));
        s.push('\n');
    }
}

fn write_contact_block(s: &mut String, c: &Contact) {
    s.push_str("contact:\n");
    s.push_str("  display_name: ");
    s.push_str(&yaml_string(&c.display_name));
    s.push('\n');
    if let Some(g) = c.given_name.as_ref() {
        s.push_str("  given_name: ");
        s.push_str(&yaml_string(g));
        s.push('\n');
    }
    if let Some(f) = c.family_name.as_ref() {
        s.push_str("  family_name: ");
        s.push_str(&yaml_string(f));
        s.push('\n');
    }
    write_emails(s, &c.emails);
    write_phones(s, &c.phones);
    write_orgs(s, &c.organizations);
    write_labels(s, &c.labels);
}

fn write_emails(s: &mut String, emails: &[EmailAddress]) {
    if emails.is_empty() {
        return;
    }
    s.push_str("  emails:\n");
    for e in emails {
        s.push_str("    - { value: ");
        s.push_str(&yaml_string(&e.value));
        if let Some(l) = e.label.as_ref() {
            s.push_str(", label: ");
            s.push_str(&yaml_string(l));
        }
        s.push_str(" }\n");
    }
}

fn write_phones(s: &mut String, phones: &[PhoneNumber]) {
    if phones.is_empty() {
        return;
    }
    s.push_str("  phones:\n");
    for p in phones {
        s.push_str("    - { value: ");
        s.push_str(&yaml_string(&p.value));
        if let Some(l) = p.label.as_ref() {
            s.push_str(", label: ");
            s.push_str(&yaml_string(l));
        }
        s.push_str(" }\n");
    }
}

fn write_orgs(s: &mut String, orgs: &[Organization]) {
    if orgs.is_empty() {
        return;
    }
    s.push_str("  organizations:\n");
    for o in orgs {
        s.push_str("    - { name: ");
        s.push_str(&yaml_string(&o.name));
        if let Some(t) = o.title.as_ref() {
            s.push_str(", title: ");
            s.push_str(&yaml_string(t));
        }
        s.push_str(" }\n");
    }
}

fn write_labels(s: &mut String, labels: &[String]) {
    if labels.is_empty() {
        return;
    }
    s.push_str("  labels: [");
    for (i, l) in labels.iter().enumerate() {
        if i > 0 {
            s.push_str(", ");
        }
        s.push_str(&yaml_string(l));
    }
    s.push_str("]\n");
}

/// Double-quoted YAML string with the four escapes that matter for
/// arbitrary contact data. Newlines / carriage returns / tabs land
/// as escapes; everything else passes through.
fn yaml_string(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => {
                // Other C0 controls. Use \xHH (YAML supports it).
                out.push_str(&format!("\\x{:02X}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contacts::{EmailAddress, Organization, PhoneNumber, ProviderKind};
    use chrono::TimeZone;

    fn ctx() -> EmitContext {
        EmitContext {
            imported_at: Utc.with_ymd_and_hms(2026, 5, 10, 12, 34, 56).unwrap(),
        }
    }

    #[test]
    fn full_contact_round_trip_through_gray_matter() {
        let c = Contact {
            provider: ProviderKind::Google,
            remote_id: Some("people/c1".into()),
            display_name: "Jane Q. Doe".into(),
            given_name: Some("Jane".into()),
            family_name: Some("Doe".into()),
            emails: vec![EmailAddress {
                value: "jane@example.com".into(),
                label: Some("work".into()),
            }],
            phones: vec![PhoneNumber {
                value: "+1-555-0100".into(),
                label: Some("mobile".into()),
            }],
            organizations: vec![Organization {
                name: "Acme Corp".into(),
                title: Some("Engineer".into()),
            }],
            notes: Some("Met at FOSDEM 2026.".into()),
            labels: vec!["Friends".into(), "Work".into()],
        };
        let md = render_markdown(&c, &ctx());

        // Body present.
        assert!(md.contains("# Jane Q. Doe"));
        assert!(md.contains("Met at FOSDEM 2026."));

        // Frontmatter parses cleanly via the same parser the
        // markdown indexer uses; confirms shape isn't broken.
        let fm = crate::markdown::parse_frontmatter(&md);
        assert!(fm.body_offset > 0, "frontmatter not detected");
        let chan = fm.data.get("chan").expect("chan block");
        assert_eq!(chan.get("kind").and_then(|v| v.as_str()), Some("contact"));
        assert_eq!(
            chan.get("provider").and_then(|v| v.as_str()),
            Some("google")
        );
        assert_eq!(
            chan.get("frontmatter_version").and_then(|v| v.as_u64()),
            Some(1)
        );
        let contact = fm.data.get("contact").expect("contact block");
        assert_eq!(
            contact.get("display_name").and_then(|v| v.as_str()),
            Some("Jane Q. Doe")
        );
    }

    #[test]
    fn minimal_contact_omits_optional_blocks() {
        let c = Contact {
            display_name: "X".into(),
            ..Default::default()
        };
        let md = render_markdown(&c, &ctx());
        assert!(md.contains("display_name: \"X\""));
        assert!(!md.contains("emails:"));
        assert!(!md.contains("phones:"));
        assert!(!md.contains("organizations:"));
        assert!(!md.contains("labels:"));
        assert!(!md.contains("remote_id"));
    }

    #[test]
    fn quoted_strings_escape_specials() {
        let c = Contact {
            display_name: "Quote \" Backslash \\ Newline\nTab\tEnd".into(),
            ..Default::default()
        };
        let md = render_markdown(&c, &ctx());
        // The display_name frontmatter value must escape these.
        assert!(md.contains(r#"display_name: "Quote \" Backslash \\ Newline\nTab\tEnd""#));
    }

    #[test]
    fn body_starts_with_h1_of_display_name() {
        let c = Contact {
            display_name: "Some Body".into(),
            ..Default::default()
        };
        let md = render_markdown(&c, &ctx());
        let body_start = md.split("---\n").nth(2).unwrap().trim_start();
        assert!(body_start.starts_with("# Some Body"));
    }
}
