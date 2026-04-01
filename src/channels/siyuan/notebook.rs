//! SiYuan notebook operations.
//!
//! This module provides functions for managing notebooks in SiYuan:
//! - List, open, close notebooks
//! - Create, rename, remove notebooks
//! - Get and set notebook configuration

use super::client::SiyuanClient;
use super::types::{EmptyResponse, Notebook};
use serde::Serialize;
use std::collections::HashMap;

/// Request to list all notebooks.
#[derive(Debug, Clone, Serialize)]
pub struct ListNotebooksRequest;

/// Response containing a list of notebooks.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ListNotebooksResponse {
    pub notebooks: Vec<Notebook>,
}

/// Request to open a notebook.
#[derive(Debug, Clone, Serialize)]
pub struct OpenNotebookRequest {
    pub notebook: String,
}

/// Request to close a notebook.
#[derive(Debug, Clone, Serialize)]
pub struct CloseNotebookRequest {
    pub notebook: String,
}

/// Request to create a new notebook.
#[derive(Debug, Clone, Serialize)]
pub struct CreateNotebookRequest {
    pub name: String,
}

/// Response after creating a notebook.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct CreateNotebookResponse {
    pub notebook: Notebook,
}

/// Request to rename a notebook.
#[derive(Debug, Clone, Serialize)]
pub struct RenameNotebookRequest {
    pub notebook: String,
    pub name: String,
}

/// Request to remove a notebook.
#[derive(Debug, Clone, Serialize)]
pub struct RemoveNotebookRequest {
    pub notebook: String,
}

/// Request to get notebook configuration.
#[derive(Debug, Clone, Serialize)]
pub struct GetNotebookConfRequest {
    pub notebook: String,
}

/// Notebook configuration data.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct NotebookConf {
    pub name: String,
    pub closed: bool,
    #[serde(rename = "refCreateSavePath")]
    pub ref_create_save_path: String,
    #[serde(rename = "createDocNameTemplate")]
    pub create_doc_name_template: String,
    #[serde(rename = "dailyNoteSavePath")]
    pub daily_note_save_path: String,
    #[serde(rename = "dailyNoteTemplatePath")]
    pub daily_note_template_path: String,
}

/// Response containing notebook configuration.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct GetNotebookConfResponse {
    pub box_id: String,
    pub conf: NotebookConf,
    pub name: String,
}

/// Request to set notebook configuration.
#[derive(Debug, Clone, Serialize)]
pub struct SetNotebookConfRequest {
    pub notebook: String,
    pub conf: NotebookConfUpdate,
}

/// Notebook configuration update (same fields as conf for setting).
#[derive(Debug, Clone, Serialize)]
pub struct NotebookConfUpdate {
    pub name: String,
    pub closed: bool,
    #[serde(rename = "refCreateSavePath")]
    pub ref_create_save_path: String,
    #[serde(rename = "createDocNameTemplate")]
    pub create_doc_name_template: String,
    #[serde(rename = "dailyNoteSavePath")]
    pub daily_note_save_path: String,
    #[serde(rename = "dailyNoteTemplatePath")]
    pub daily_note_template_path: String,
}

impl SiyuanClient {
    /// List all notebooks in the SiYuan instance.
    ///
    /// # Returns
    /// A vector of notebook information including ID, name, icon, sort order, and closed status.
    pub async fn list_notebooks(&self) -> anyhow::Result<Vec<Notebook>> {
        #[derive(serde::Deserialize)]
        struct Response {
            notebooks: Vec<Notebook>,
        }

        let result: Response = self.post_data("/api/notebook/lsNotebooks", &ListNotebooksRequest).await?;
        Ok(result.notebooks)
    }

    /// Open a notebook by ID.
    ///
    /// # Arguments
    /// * `notebook_id` - The ID of the notebook to open.
    pub async fn open_notebook(&self, notebook_id: &str) -> anyhow::Result<()> {
        self.post_data::<_, EmptyResponse>(
            "/api/notebook/openNotebook",
            &OpenNotebookRequest {
                notebook: notebook_id.to_string(),
            },
        )
        .await
    }

    /// Close a notebook by ID.
    ///
    /// # Arguments
    /// * `notebook_id` - The ID of the notebook to close.
    pub async fn close_notebook(&self, notebook_id: &str) -> anyhow::Result<()> {
        self.post_data::<_, EmptyResponse>(
            "/api/notebook/closeNotebook",
            &CloseNotebookRequest {
                notebook: notebook_id.to_string(),
            },
        )
        .await
    }

    /// Create a new notebook.
    ///
    /// # Arguments
    /// * `name` - The name for the new notebook.
    ///
    /// # Returns
    /// The created notebook information including its assigned ID.
    pub async fn create_notebook(&self, name: &str) -> anyhow::Result<Notebook> {
        #[derive(serde::Deserialize)]
        struct Response {
            notebook: Notebook,
        }

        let result: Response = self
            .post_data(
                "/api/notebook/createNotebook",
                &CreateNotebookRequest {
                    name: name.to_string(),
                },
            )
            .await?;
        Ok(result.notebook)
    }

    /// Rename a notebook.
    ///
    /// # Arguments
    /// * `notebook_id` - The ID of the notebook to rename.
    /// * `new_name` - The new name for the notebook.
    pub async fn rename_notebook(&self, notebook_id: &str, new_name: &str) -> anyhow::Result<()> {
        self.post_data::<_, EmptyResponse>(
            "/api/notebook/renameNotebook",
            &RenameNotebookRequest {
                notebook: notebook_id.to_string(),
                name: new_name.to_string(),
            },
        )
        .await
    }

    /// Remove a notebook.
    ///
    /// # Arguments
    /// * `notebook_id` - The ID of the notebook to remove.
    ///
    /// # Warning
    /// This operation is irreversible. All documents in the notebook will be deleted.
    pub async fn remove_notebook(&self, notebook_id: &str) -> anyhow::Result<()> {
        self.post_data::<_, EmptyResponse>(
            "/api/notebook/removeNotebook",
            &RemoveNotebookRequest {
                notebook: notebook_id.to_string(),
            },
        )
        .await
    }

    /// Get notebook configuration.
    ///
    /// # Arguments
    /// * `notebook_id` - The ID of the notebook.
    pub async fn get_notebook_conf(&self, notebook_id: &str) -> anyhow::Result<NotebookConf> {
        #[derive(serde::Deserialize)]
        struct Response {
            #[serde(rename = "conf")]
            conf: NotebookConf,
        }

        let result: Response = self
            .post_data(
                "/api/notebook/getNotebookConf",
                &GetNotebookConfRequest {
                    notebook: notebook_id.to_string(),
                },
            )
            .await?;
        Ok(result.conf)
    }

    /// Set notebook configuration.
    ///
    /// # Arguments
    /// * `notebook_id` - The ID of the notebook.
    /// * `conf` - The new configuration to apply.
    pub async fn set_notebook_conf(
        &self,
        notebook_id: &str,
        conf: NotebookConfUpdate,
    ) -> anyhow::Result<()> {
        self.post_data::<_, EmptyResponse>(
            "/api/notebook/setNotebookConf",
            &SetNotebookConfRequest {
                notebook: notebook_id.to_string(),
                conf,
            },
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_notebooks_request_serialization() {
        let result = serde_json::to_string(&ListNotebooksRequest);
        assert!(result.is_ok());
    }
}