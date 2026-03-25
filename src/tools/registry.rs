//! Dynamic tool registration system.
//!
//! This module provides a registry-based system for tools to register themselves
//! automatically, eliminating the need for manual initialization and registration
//! in mod.rs. Tools use the [`register_tool!`] macro to self-register.
//!
//! # Example
//!
//! ```ignore
//! use crate::tools::registry::register_tool;
//! use crate::tools::traits::Tool;
//! use crate::security::SecurityPolicy;
//! use std::sync::Arc;
//!
//! register_tool! {
//!     name: "my_tool",
//!     description: "My custom tool",
//!     parameters: {
//!         "type": "object",
//!         "properties": {
//!             "input": { "type": "string" }
//!         },
//!         "required": ["input"]
//!     },
//!     factory: |security: Arc<SecurityPolicy>| -> Arc<dyn Tool> {
//!         Arc::new(MyTool::new(security))
//!     }
//! }
//! ```

use crate::security::SecurityPolicy;
use crate::tools::traits::Tool;
use inventory;
use serde_json::Value;
use std::fmt;
use std::sync::Arc;

inventory::collect!(ToolFactory);

/// Factory function for creating tool instances
pub type ToolFactoryFn = dyn Fn(Arc<SecurityPolicy>) -> Arc<dyn Tool> + Send + Sync;

/// Metadata about a tool for registration
#[derive(Clone)]
pub struct ToolMetadata {
    /// Tool name (e.g., "shell", "file_read")
    pub name: &'static str,
    /// Human-readable description
    pub description: &'static str,
    /// JSON Schema for parameters
    pub parameters_schema: Value,
    /// Whether this tool is always enabled
    pub always_enabled: bool,
}

impl fmt::Debug for ToolMetadata {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ToolMetadata")
            .field("name", &self.name)
            .field("description", &self.description)
            .field("always_enabled", &self.always_enabled)
            .finish()
    }
}

/// Factory for creating tool instances
pub struct ToolFactory {
    /// Tool metadata
    pub metadata: ToolMetadata,
    /// Factory function to create tool instances
    pub factory: Box<ToolFactoryFn>,
}

impl fmt::Debug for ToolFactory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ToolFactory")
            .field("metadata", &self.metadata)
            .finish()
    }
}

/// Trait for tool registries
pub trait ToolRegistry: Send + Sync {
    /// Register a tool factory with this registry
    fn register(&mut self, factory: ToolFactory);

    /// Create all tool instances from registered factories
    fn create_tools(&self, security: Arc<SecurityPolicy>) -> Vec<Arc<dyn Tool>>;

    /// Get metadata for all registered tools
    fn metadata(&self) -> Vec<&ToolMetadata>;
}

/// Default in-memory tool registry
#[derive(Debug, Default)]
pub struct DefaultToolRegistry {
    factories: Vec<ToolFactory>,
}

impl DefaultToolRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a tool factory
    pub fn register(&mut self, factory: ToolFactory) {
        self.factories.push(factory);
    }

    /// Create all tool instances from registered factories
    pub fn create_tools(&self, security: Arc<SecurityPolicy>) -> Vec<Arc<dyn Tool>> {
        self.factories
            .iter()
            .map(|factory| (factory.factory)(security.clone()))
            .collect()
    }

    /// Get metadata for all registered tools
    pub fn metadata(&self) -> Vec<&ToolMetadata> {
        self.factories.iter().map(|f| &f.metadata).collect()
    }

    /// Filter tools by predicate
    pub fn filter<F>(&self, security: Arc<SecurityPolicy>, predicate: F) -> Vec<Arc<dyn Tool>>
    where
        F: Fn(&ToolMetadata) -> bool,
    {
        self.factories
            .iter()
            .filter(|f| predicate(&f.metadata))
            .map(|factory| (factory.factory)(Arc::clone(&security)))
            .collect()
    }
}

/// Collect all tool factories from the inventory
/// Note: This is currently unused due to ownership issues with Box<dyn Fn>
/// Kept for potential future use if we can make ToolFactory cloneable
#[allow(dead_code)]
pub fn collect_tool_factories() -> Vec<&'static ToolFactory> {
    // Collect into Vec first since inventory::iter returns a custom iterator
    inventory::iter::<ToolFactory>
        .into_iter()
        .collect()
}

/// Create a registry from all globally registered tools
pub fn create_global_registry() -> DefaultToolRegistry {
    let mut registry = DefaultToolRegistry::new();
    // Iterate directly over inventory to avoid ownership issues
    for factory in inventory::iter::<ToolFactory>.into_iter() {
        // We need to clone the metadata but can't clone the factory function
        // Instead, we'll create a wrapper that calls the original factory
        let metadata = factory.metadata.clone();
        let factory_fn = &factory.factory;

        // Create a new ToolFactory with a wrapper function
        registry.register(ToolFactory {
            metadata,
            factory: Box::new({
                // SAFETY: The factory function comes from static inventory and lives for 'static
                // We extend the lifetime to allow moving into the closure
                let factory_fn: &'static Box<ToolFactoryFn> = unsafe {
                    std::mem::transmute_copy::<&Box<ToolFactoryFn>, &'static Box<ToolFactoryFn>>(&factory_fn)
                };
                move |security| (factory_fn)(security)
            }),
        });
    }
    registry
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_creation() {
        let registry = DefaultToolRegistry::new();
        assert_eq!(registry.metadata().len(), 0);
    }

    #[test]
    fn test_registry_register() {
        let mut registry = DefaultToolRegistry::new();
        registry.register(ToolFactory {
            metadata: ToolMetadata {
                name: "test_tool",
                description: "A test tool",
                parameters_schema: serde_json::json!({}),
                always_enabled: true,
            },
            factory: Box::new(|_| unimplemented!()),
        });

        let metadata = registry.metadata();
        assert_eq!(metadata.len(), 1);
        assert_eq!(metadata[0].name, "test_tool");
    }
}
