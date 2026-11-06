//! Read the newest content of a single artifact by id.

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{json, Value};
use tinytools::{Tool, ToolResult};

use crate::service::Artifacts;
use crate::tools::read_required_str;
use crate::types::ArtifactRecord;

/// Fetch one artifact with its full version history.
pub struct ReadArtifactTool {
    artifacts: Arc<Artifacts>,
}

impl std::fmt::Debug for ReadArtifactTool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReadArtifactTool").finish_non_exhaustive()
    }
}

impl ReadArtifactTool {
    /// Constructs the tool bound to one service.
    #[must_use]
    pub fn new(artifacts: Arc<Artifacts>) -> Self {
        Self { artifacts }
    }
}

impl ReadArtifactTool {
    /// What a model reads of `record`: its newest content, or why there is none.
    async fn read(&self, record: &ArtifactRecord) -> anyhow::Result<ToolResult> {
        let mut out = json!({
            "artifact_id": record.id,
            "title": record.title,
            "kind": record.kind.as_str(),
            "version": record.latest().map(|version| version.version),
        });
        match self.artifacts.readable_text(record).await {
            Ok(Some(content)) => out["content"] = json!(content),
            Ok(None) => out["unreadable"] = json!("this file has no text to read"),
            Err(error) => out["unreadable"] = json!(error.to_string()),
        }
        Ok(ToolResult::success(serde_json::to_string(&out)?))
    }
}

#[async_trait]
impl Tool for ReadArtifactTool {
    fn name(&self) -> &'static str {
        "read_artifact"
    }

    fn description(&self) -> &'static str {
        "Fetch the newest content of one artifact by its `artifact_id`: text as \
         written, HTML as its source, and the text of a PDF, DOCX or PPTX \
         file. Formatting in a file is not returned. The id is the one \
         `create_artifact` returned. Read an artifact before changing it: the \
         record is current, and your memory of it may not be."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "artifact_id": {
                    "type": "string",
                    "description": "The artifact id (UUID) to fetch."
                }
            },
            "required": ["artifact_id"]
        })
    }

    async fn execute(&self, args: Value) -> anyhow::Result<ToolResult> {
        let id = read_required_str(&args, "artifact_id")?;
        match self.artifacts.store().get(&id).await? {
            Some(record) => self.read(&record).await,
            None => Ok(ToolResult::error(format!("artifact `{id}` not found"))),
        }
    }

    fn is_concurrency_safe(&self, _args: &Value) -> bool {
        true
    }
}
