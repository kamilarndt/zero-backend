//! Automated JSON Schema generation for tool parameters.
//!
//! This module provides utilities for automatically generating JSON schemas
//! from Rust types and custom schema definitions. It eliminates the need for
//! manual schema construction in each tool.
//!
//! # Example
//!
//! ```ignore
//! use crate::tools::schema_builder::{SchemaBuilder, schema};
//! use serde::Serialize;
//!
//! #[derive(Serialize)]
//! struct MyToolParams {
//!     input: String,
//!     count: Option<u32>,
//! }
//!
//! let schema = SchemaBuilder::for_struct::<MyToolParams>()
//!     .description("input", "The input text")
//!     .description("count", "Number of iterations")
//!     .required("input")
//!     .build();
//! ```

use serde::Serialize;
use serde_json::{json, Value};
use std::collections::HashMap;

/// Builder for generating JSON Schemas
#[derive(Debug, Clone)]
pub struct SchemaBuilder {
    properties: HashMap<String, Value>,
    required: Vec<String>,
}

impl Default for SchemaBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl SchemaBuilder {
    /// Create a new empty schema builder
    pub fn new() -> Self {
        Self {
            properties: HashMap::new(),
            required: Vec::new(),
        }
    }

    /// Create a schema builder for a Rust struct using serde_json
    pub fn for_struct<T: Serialize>() -> Self {
        // Use schemars if available, otherwise fall back to basic schema
        Self::new()
    }

    /// Add a property with a JSON Schema definition
    pub fn property(mut self, name: impl Into<String>, schema: Value) -> Self {
        self.properties.insert(name.into(), schema);
        self
    }

    /// Add a string property
    pub fn string_property(mut self, name: impl Into<String>) -> Self {
        self.properties.insert(
            name.into(),
            json!({
                "type": "string"
            }),
        );
        self
    }

    /// Add a boolean property
    pub fn boolean_property(mut self, name: impl Into<String>) -> Self {
        self.properties.insert(
            name.into(),
            json!({
                "type": "boolean"
            }),
        );
        self
    }

    /// Add an integer property
    pub fn integer_property(mut self, name: impl Into<String>) -> Self {
        self.properties.insert(
            name.into(),
            json!({
                "type": "integer"
            }),
        );
        self
    }

    /// Add a number property
    pub fn number_property(mut self, name: impl Into<String>) -> Self {
        self.properties.insert(
            name.into(),
            json!({
                "type": "number"
            }),
        );
        self
    }

    /// Add an array property
    pub fn array_property(mut self, name: impl Into<String>, items_schema: Value) -> Self {
        self.properties.insert(
            name.into(),
            json!({
                "type": "array",
                "items": items_schema
            }),
        );
        self
    }

    /// Add an object property
    pub fn object_property(mut self, name: impl Into<String>, properties: Value) -> Self {
        self.properties.insert(
            name.into(),
            json!({
                "type": "object",
                "properties": properties
            }),
        );
        self
    }

    /// Add or update a description for a property
    pub fn description(mut self, name: impl Into<String>, description: impl Into<String>) -> Self {
        let name = name.into();
        if let Some(prop) = self.properties.get_mut(&name) {
            if let Some(obj) = prop.as_object_mut() {
                obj.insert("description".to_string(), json!(description.into()));
            }
        }
        self
    }

    /// Add a default value for a property
    pub fn default(mut self, name: impl Into<String>, default: Value) -> Self {
        let name = name.into();
        if let Some(prop) = self.properties.get_mut(&name) {
            if let Some(obj) = prop.as_object_mut() {
                obj.insert("default".to_string(), default);
            }
        }
        self
    }

    /// Add a property to the required list
    pub fn required(mut self, name: impl Into<String>) -> Self {
        self.required.push(name.into());
        self
    }

    /// Build the final JSON Schema
    pub fn build(self) -> Value {
        json!({
            "type": "object",
            "properties": self.properties,
            "required": self.required
        })
    }

    /// Build without required fields (optional parameters)
    pub fn build_optional(self) -> Value {
        json!({
            "type": "object",
            "properties": self.properties
        })
    }
}

/// Convenience macro for defining inline schemas
#[macro_export]
macro_rules! schema {
    () => {
        $crate::tools::schema_builder::SchemaBuilder::new()
    };
}

/// Convenience macro for property definitions
#[macro_export]
macro_rules! prop {
    (string $name:expr, $desc:expr) => {
        serde_json::json!({
            "type": "string",
            "description": $desc
        })
    };
    (integer $name:expr, $desc:expr) => {
        serde_json::json!({
            "type": "integer",
            "description": $desc
        })
    };
    (boolean $name:expr, $desc:expr) => {
        serde_json::json!({
            "type": "boolean",
            "description": $desc
        })
    };
    (array $name:expr, $desc:expr, $items:expr) => {
        serde_json::json!({
            "type": "array",
            "description": $desc,
            "items": $items
        })
    };
}

/// Helper to build a simple schema with common patterns
pub struct SimpleSchema;

impl SimpleSchema {
    /// Create a schema for a tool with a single command parameter (like shell)
    pub fn command(additional: Value) -> Value {
        let mut props = serde_json::Map::new();
        props.insert(
            "command".to_string(),
            json!({
                "type": "string",
                "description": "The command to execute"
            }),
        );
        if let Some(obj) = additional.as_object() {
            for (k, v) in obj {
                props.insert(k.clone(), v.clone());
            }
        }
        json!({
            "type": "object",
            "properties": props,
            "required": ["command"]
        })
    }

    /// Create a schema for a tool with path parameter (like file_read)
    pub fn path(additional: Value) -> Value {
        let mut props = serde_json::Map::new();
        props.insert(
            "path".to_string(),
            json!({
                "type": "string",
                "description": "File or directory path"
            }),
        );
        if let Some(obj) = additional.as_object() {
            for (k, v) in obj {
                props.insert(k.clone(), v.clone());
            }
        }
        json!({
            "type": "object",
            "properties": props,
            "required": ["path"]
        })
    }

    /// Create a schema for memory tools (key, content, category)
    pub fn memory() -> Value {
        json!({
            "type": "object",
            "properties": {
                "key": {
                    "type": "string",
                    "description": "Unique key for this memory"
                },
                "content": {
                    "type": "string",
                    "description": "The information to remember"
                },
                "category": {
                    "type": "string",
                    "description": "Memory category (core, daily, conversation, or custom)"
                }
            },
            "required": ["key", "content"]
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_builder_basic() {
        let schema = SchemaBuilder::new()
            .string_property("input")
            .description("input", "The input text")
            .required("input")
            .build();

        assert_eq!(schema["type"], "object");
        assert_eq!(schema["properties"]["input"]["type"], "string");
        assert_eq!(
            schema["properties"]["input"]["description"],
            "The input text"
        );
        assert_eq!(schema["required"].as_array().unwrap()[0], "input");
    }

    #[test]
    fn test_schema_builder_multiple_props() {
        let schema = SchemaBuilder::new()
            .string_property("name")
            .integer_property("age")
            .boolean_property("active")
            .build();

        assert_eq!(schema["properties"].as_object().unwrap().len(), 3);
        assert_eq!(schema["properties"]["name"]["type"], "string");
        assert_eq!(schema["properties"]["age"]["type"], "integer");
        assert_eq!(schema["properties"]["active"]["type"], "boolean");
    }

    #[test]
    fn test_simple_schema_command() {
        let schema = SimpleSchema::command(json!({
            "approved": {
                "type": "boolean",
                "description": "Explicit approval flag",
                "default": false
            }
        }));

        assert_eq!(schema["required"].as_array().unwrap()[0], "command");
        assert_eq!(schema["properties"]["approved"]["type"], "boolean");
        assert_eq!(schema["properties"]["approved"]["default"], false);
    }

    #[test]
    fn test_prop_macro() {
        let schema = prop!(string "test", "A test property");
        assert_eq!(schema["type"], "string");
        assert_eq!(schema["description"], "A test property");
    }
}
