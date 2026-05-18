//! HTTP route handlers, organized by area.
//!
//! Per-area submodules host the handlers, request / response shapes,
//! and any helpers specific to that area. Cross-area types (e.g.
//! `PreferencesView`) live in the module that owns them and are
//! re-exported here. `lib.rs::router()` wires every endpoint.

mod attachments;
mod build_info;
mod contacts;
mod drive;
mod files;
mod fs_graph;
mod graph;
mod health;
mod inspector;
mod preferences;
mod report;
mod search;
mod sessions;
mod storage;
mod terminal;
mod ws;

pub use attachments::api_post_attachment;
pub use build_info::api_build_info;
pub use contacts::{api_get_contacts, api_post_contacts_import};
pub use drive::{api_cloud_drives, api_get_drive, api_patch_drive};
pub use files::{
    api_create_file, api_delete_file, api_list_files, api_move, api_read_file, api_write_file,
};
pub use fs_graph::{api_fs_graph, build_fs_graph, FsGraphResponse, FsGraphScope};
pub use graph::{
    api_backlinks, api_graph, api_headings, api_language_graph, api_link_targets, api_links,
    api_resolve_link,
};
pub use health::api_health;
pub use inspector::api_inspector;
pub use preferences::{
    api_get_config, api_get_server_config, api_patch_config, api_patch_server_config,
};
pub use report::{api_report_file, api_report_prefix};
pub use search::{api_index_rebuild, api_index_status, api_search_content, api_search_files};
pub use sessions::{api_delete_session, api_get_session, api_list_sessions, api_put_session};
pub use storage::api_storage_reset;
pub use terminal::api_terminal_ws;
pub use ws::ws_upgrade;
