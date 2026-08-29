//! Resource endpoints: read-only documents backed by the engine crates.
//!
//! The `ServerHandler` resource methods live on the single trait impl in
//! `handler.rs` and delegate to the free functions here:
//! * `ink://analysis/{root}` — Repository Intelligence JSON document.
//! * `ink://graph/{root}` — dependency graph JSON document.

use rmcp::{
    model::{
        ErrorData, ListResourceTemplatesResult, ListResourcesResult, ReadResourceResult,
        ResourceContents, ResourceTemplate,
    },
    service::RequestContext,
    RoleServer,
};

use crate::engine::{notify_progress, run_analysis, run_graph};
use crate::reporting::Reporter;

pub fn list_resources() -> ListResourcesResult {
    ListResourcesResult::default()
}

pub fn list_resource_templates() -> ListResourceTemplatesResult {
    ListResourceTemplatesResult::with_all_items(vec![
        ResourceTemplate::new(
            "ink://analysis/{root}",
            "Repository Intelligence analysis document",
        )
        .with_description("Repository Intelligence analysis JSON for the repository at {root}.")
        .with_mime_type("application/json"),
        ResourceTemplate::new("ink://graph/{root}", "Dependency graph document")
            .with_description("Dependency graph JSON for the repository at {root}.")
            .with_mime_type("application/json"),
    ])
}

pub async fn read_resource(
    uri: &str,
    ctx: &RequestContext<RoleServer>,
) -> Result<ReadResourceResult, ErrorData> {
    let (_, contents) = if let Some(root) = uri.strip_prefix("ink://analysis/") {
        if root.is_empty() {
            return Err(ErrorData::invalid_params(
                "ink://analysis resource requires a repository root",
                None,
            ));
        }
        notify_progress(ctx, 0.5, "analyzing repository").await;
        (root.to_owned(), run_analysis(root))
    } else if let Some(root) = uri.strip_prefix("ink://graph/") {
        if root.is_empty() {
            return Err(ErrorData::invalid_params(
                "ink://graph resource requires a repository root",
                None,
            ));
        }
        notify_progress(ctx, 0.3, "analyzing repository").await;
        notify_progress(ctx, 0.6, "building dependency graph").await;
        (root.to_owned(), run_graph(root))
    } else {
        return Err(ErrorData::invalid_params(
            format!("unsupported resource URI: {uri}"),
            None,
        ));
    };

    notify_progress(ctx, 1.0, "resource ready").await;
    let json = contents.map_err(|message| {
        ErrorData::internal_error(format!("failed to read resource: {message}"), None)
    })?;
    Reporter::from_env().record("read_resource", &json);
    Ok(ReadResourceResult::new(vec![ResourceContents::text(
        json,
        uri.to_owned(),
    )
    .with_mime_type("application/json")]))
}
