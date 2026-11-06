//! Update an existing artifact by appending a new version.

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{json, Value};
use tinytools::{PermissionLevel, Tool, ToolResult};

use crate::service::{Artifacts, Author, Content};
use crate::tools::{content_for, optional_str, read_required_str, saved_result, tool_error};
use crate::types::ArtifactRecord;

/// Append a new revision to an existing artifact.
pub struct UpdateArtifactTool {
    artifacts: Arc<Artifacts>,
    author: Author,
}

impl std::fmt::Debug for UpdateArtifactTool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UpdateArtifactTool").finish_non_exhaustive()
    }
}

impl UpdateArtifactTool {
    /// Constructs the tool bound to one service, writing versions as
    /// `author` (the agent the tool is built for).
    #[must_use]
    pub fn new(artifacts: Arc<Artifacts>, author: Author) -> Self {
        Self { artifacts, author }
    }
}

#[async_trait]
impl Tool for UpdateArtifactTool {
    fn name(&self) -> &'static str {
        "update_artifact"
    }

    fn description(&self) -> &'static str {
        "Append a new revision to an existing artifact; earlier versions are \
         kept. Call `read_artifact` first. To add to the end of a text \
         artifact, pass only the new lines as `append`: the existing content \
         is kept. To change existing content, pass the full new `body`, \
         written from what `read_artifact` returned, never from memory. Image \
         and file artifacts take the new file's `path`. A body identical to \
         the newest version adds nothing. Use this, not `create_artifact`, to \
         change an artifact: it stays the same artifact under the same id. \
         Returns the new version number."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "artifact_id": {
                    "type": "string",
                    "description": "The artifact id to update."
                },
                "body": {
                    "type": "string",
                    "description": "For text artifacts: the full new content. Read the artifact first."
                },
                "append": {
                    "type": "string",
                    "description": "For text artifacts: only the lines to add to the end. Not with `body`."
                },
                "path": {
                    "type": "string",
                    "description": "For image and file artifacts: the new file, relative to your working folder."
                },
                "note": {
                    "type": "string",
                    "description": "Optional note about this revision (e.g. why it was changed)."
                }
            },
            "required": ["artifact_id"]
        })
    }

    fn permission_level(&self) -> PermissionLevel {
        PermissionLevel::Write
    }

    async fn execute(&self, args: Value) -> anyhow::Result<ToolResult> {
        let id = read_required_str(&args, "artifact_id")?;
        let record = match self.artifacts.get(&id).await {
            Ok(Some(record)) => record,
            Ok(None) => return Ok(ToolResult::error(format!("artifact `{id}` not found"))),
            Err(err) => return tool_error(err),
        };
        let content = match args.get("append").and_then(Value::as_str) {
            Some(lines) => appended(&record, lines, &args)?,
            None => content_for(record.kind, &args)?,
        };
        match self
            .artifacts
            .revise(&id, content, &self.author, optional_str(&args, "note"))
            .await
        {
            Ok(saved) => Ok(saved_result(&saved)),
            Err(err) => tool_error(err),
        }
    }

    fn is_concurrency_safe(&self, _args: &Value) -> bool {
        false
    }
}

/// The newest version of `record` with `lines` added to its end.
///
/// A call that mixes `append` with a whole `body` says two things, and a file
/// has no lines to add to, so both are malformed.
fn appended(record: &ArtifactRecord, lines: &str, args: &Value) -> anyhow::Result<Content> {
    anyhow::ensure!(
        args.get("body").is_none(),
        "pass `append` or `body`, not both"
    );
    anyhow::ensure!(!lines.trim().is_empty(), "`append` needs the lines to add");
    anyhow::ensure!(
        !record.holds_file(),
        "a `{}` artifact needs `path`: `append` is for text",
        record.kind.as_str()
    );
    let existing = record.latest().map_or("", |version| version.body.as_str());
    Ok(Content::Text(joined(existing, lines)))
}

/// `lines` after `existing`, on a line of their own, keeping whether the text
/// ended in a newline.
fn joined(existing: &str, lines: &str) -> String {
    let mut body = existing.to_string();
    if !body.is_empty() && !body.ends_with('\n') {
        body.push('\n');
    }
    body.push_str(lines);
    if existing.ends_with('\n') && !lines.ends_with('\n') {
        body.push('\n');
    }
    body
}
