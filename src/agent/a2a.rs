use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// Role classification for agents in the swarm hierarchy.
/// Used to determine capabilities, permissions, and routing behavior.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum AgentRole {
    /// Planning agent - decomposes tasks, creates dependencies
    Planner,
    /// Execution agent - performs assigned tasks
    Executor,
    /// Reviewer agent - validates results and quality
    Reviewer,
}

impl FromStr for AgentRole {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "planner" => Ok(AgentRole::Planner),
            "executor" => Ok(AgentRole::Executor),
            "reviewer" => Ok(AgentRole::Reviewer),
            _ => Err(format!("Unknown agent role: {}", s)),
        }
    }
}

impl AgentRole {
    /// Returns the string representation of the role
    pub fn as_str(&self) -> &'static str {
        match self {
            AgentRole::Planner => "planner",
            AgentRole::Executor => "executor",
            AgentRole::Reviewer => "reviewer",
        }
    }
}

/// Message types for Agent-to-Agent (A2A) communication.
/// Each variant represents a specific interaction pattern in the swarm.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", content = "payload")]
pub enum A2AMessageType {
    /// Initial task assignment from planner to executor
    TaskAssignment {
        /// Unique task identifier
        task_id: String,
        /// Human-readable instructions for the task
        instructions: String,
        /// List of task_ids that must complete before this task starts
        dependencies: Vec<String>,
    },

    /// Progress update from executor to planner/reviewer
    TaskProgress {
        /// Task identifier being updated
        task_id: String,
        /// Completion percentage (0-100)
        percentage: u8,
        /// Current step description
        current_step: String,
    },

    /// Final result submission from executor to reviewer
    TaskCompletion {
        /// Completed task identifier
        task_id: String,
        /// JSON-serialized result data
        result_json: String,
        /// List of artifact paths/URIs created
        artifacts: Vec<String>,
    },

    /// Request for clarification from any agent to planner
    ClarificationRequest {
        /// Task requiring clarification
        task_id: String,
        /// Specific question or ambiguity
        question: String,
    },
}

impl A2AMessageType {
    /// Returns the message type tag for serialization
    pub fn type_tag(&self) -> &'static str {
        match self {
            A2AMessageType::TaskAssignment { .. } => "task_assignment",
            A2AMessageType::TaskProgress { .. } => "task_progress",
            A2AMessageType::TaskCompletion { .. } => "task_completion",
            A2AMessageType::ClarificationRequest { .. } => "clarification_request",
        }
    }

    /// Validates the message payload
    pub fn validate(&self) -> Result<(), String> {
        match self {
            A2AMessageType::TaskAssignment {
                task_id,
                instructions,
                dependencies,
            } => {
                if task_id.trim().is_empty() {
                    return Err("task_id cannot be empty".to_string());
                }
                if instructions.trim().is_empty() {
                    return Err("instructions cannot be empty".to_string());
                }
                // Validate dependencies don't contain empty strings
                if dependencies.iter().any(|d| d.trim().is_empty()) {
                    return Err("dependencies cannot contain empty strings".to_string());
                }
                Ok(())
            }

            A2AMessageType::TaskProgress {
                task_id,
                percentage,
                current_step,
            } => {
                if task_id.trim().is_empty() {
                    return Err("task_id cannot be empty".to_string());
                }
                if *percentage > 100 {
                    return Err("percentage cannot exceed 100".to_string());
                }
                if current_step.trim().is_empty() {
                    return Err("current_step cannot be empty".to_string());
                }
                Ok(())
            }

            A2AMessageType::TaskCompletion {
                task_id,
                result_json,
                artifacts,
            } => {
                if task_id.trim().is_empty() {
                    return Err("task_id cannot be empty".to_string());
                }
                // Validate result_json is valid JSON
                if serde_json::from_str::<serde_json::Value>(result_json).is_err() {
                    return Err("result_json must be valid JSON".to_string());
                }
                // Validate artifacts don't contain empty strings
                if artifacts.iter().any(|a| a.trim().is_empty()) {
                    return Err("artifacts cannot contain empty strings".to_string());
                }
                Ok(())
            }

            A2AMessageType::ClarificationRequest {
                task_id,
                question,
            } => {
                if task_id.trim().is_empty() {
                    return Err("task_id cannot be empty".to_string());
                }
                if question.trim().is_empty() {
                    return Err("question cannot be empty".to_string());
                }
                Ok(())
            }
        }
    }
}

/// Complete packet for A2A communication.
/// Wraps message with routing metadata and timestamp.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct A2APacket {
    /// Source agent identifier
    pub source_id: String,
    /// Target agent identifier
    pub target_id: String,
    /// Unix timestamp in milliseconds
    pub timestamp: u64,
    /// The message payload
    pub message: A2AMessageType,
}

impl A2APacket {
    /// Creates a new A2APacket with current timestamp
    pub fn new(
        source_id: String,
        target_id: String,
        message: A2AMessageType,
    ) -> Self {
        Self {
            source_id,
            target_id,
            timestamp: Self::current_timestamp(),
            message,
        }
    }

    /// Creates a new A2APacket with custom timestamp (for testing)
    pub fn with_timestamp(
        source_id: String,
        target_id: String,
        timestamp: u64,
        message: A2AMessageType,
    ) -> Self {
        Self {
            source_id,
            target_id,
            timestamp,
            message,
        }
    }

    /// Returns current Unix timestamp in milliseconds
    pub fn current_timestamp() -> u64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        u64::try_from(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis(),
        )
        .unwrap_or_default()
    }

    /// Validates the entire packet
    pub fn validate(&self) -> Result<(), String> {
        if self.source_id.trim().is_empty() {
            return Err("source_id cannot be empty".to_string());
        }
        if self.target_id.trim().is_empty() {
            return Err("target_id cannot be empty".to_string());
        }
        if self.timestamp == 0 {
            return Err("timestamp cannot be zero".to_string());
        }
        self.message.validate()
    }

    /// Serializes the packet to JSON
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Deserializes a packet from JSON
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Creates a reply packet with swapped source/target
    pub fn reply(&self, reply_message: A2AMessageType) -> Self {
        Self {
            source_id: self.target_id.clone(),
            target_id: self.source_id.clone(),
            timestamp: Self::current_timestamp(),
            message: reply_message,
        }
    }

    /// Returns a summary string for logging
    pub fn summary(&self) -> String {
        format!(
            "[{} -> {}] {} at {}",
            self.source_id,
            self.target_id,
            self.message.type_tag(),
            self.timestamp
        )
    }
}

/// Builder for constructing A2AMessageType payloads
pub struct A2AMessageBuilder;

impl A2AMessageBuilder {
    /// Creates a TaskAssignment message
    pub fn task_assignment(
        task_id: impl Into<String>,
        instructions: impl Into<String>,
        dependencies: Vec<String>,
    ) -> A2AMessageType {
        A2AMessageType::TaskAssignment {
            task_id: task_id.into(),
            instructions: instructions.into(),
            dependencies,
        }
    }

    /// Creates a TaskProgress message
    pub fn task_progress(
        task_id: impl Into<String>,
        percentage: u8,
        current_step: impl Into<String>,
    ) -> A2AMessageType {
        A2AMessageType::TaskProgress {
            task_id: task_id.into(),
            percentage,
            current_step: current_step.into(),
        }
    }

    /// Creates a TaskCompletion message
    pub fn task_completion(
        task_id: impl Into<String>,
        result_json: impl Into<String>,
        artifacts: Vec<String>,
    ) -> A2AMessageType {
        A2AMessageType::TaskCompletion {
            task_id: task_id.into(),
            result_json: result_json.into(),
            artifacts,
        }
    }

    /// Creates a ClarificationRequest message
    pub fn clarification_request(
        task_id: impl Into<String>,
        question: impl Into<String>,
    ) -> A2AMessageType {
        A2AMessageType::ClarificationRequest {
            task_id: task_id.into(),
            question: question.into(),
        }
    }
}

/// Helper functions for working with A2A protocol
pub struct A2AHelper;

impl A2AHelper {
    /// Generates a unique task ID
    pub fn generate_task_id() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_micros();
        format!("task_{}", timestamp)
    }

    /// Generates a unique agent ID
    pub fn generate_agent_id(role: AgentRole) -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_micros();
        format!("{}_{}", role.as_str(), timestamp)
    }

    /// Extracts task_id from any message type
    pub fn extract_task_id(message: &A2AMessageType) -> Option<String> {
        match message {
            A2AMessageType::TaskAssignment { task_id, .. }
            | A2AMessageType::TaskProgress { task_id, .. }
            | A2AMessageType::TaskCompletion { task_id, .. }
            | A2AMessageType::ClarificationRequest { task_id, .. } => Some(task_id.clone()),
        }
    }

    /// Checks if a message is a request (requires response)
    pub fn is_request(message: &A2AMessageType) -> bool {
        matches!(
            message,
            A2AMessageType::TaskAssignment { .. }
                | A2AMessageType::ClarificationRequest { .. }
        )
    }

    /// Checks if a message is a response
    pub fn is_response(message: &A2AMessageType) -> bool {
        matches!(
            message,
            A2AMessageType::TaskProgress { .. } | A2AMessageType::TaskCompletion { .. }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_role_as_str() {
        assert_eq!(AgentRole::Planner.as_str(), "planner");
        assert_eq!(AgentRole::Executor.as_str(), "executor");
        assert_eq!(AgentRole::Reviewer.as_str(), "reviewer");
    }

    #[test]
    fn test_agent_role_from_str() {
        assert_eq!(AgentRole::from_str("planner"), Ok(AgentRole::Planner));
        assert_eq!(AgentRole::from_str("EXECUTOR"), Ok(AgentRole::Executor));
        assert_eq!(AgentRole::from_str("Reviewer"), Ok(AgentRole::Reviewer));
        assert!(AgentRole::from_str("invalid").is_err());
    }

    #[test]
    fn test_message_type_tag() {
        let msg = A2AMessageType::TaskAssignment {
            task_id: "t1".to_string(),
            instructions: "test".to_string(),
            dependencies: vec![],
        };
        assert_eq!(msg.type_tag(), "task_assignment");
    }

    #[test]
    fn test_task_assignment_validation() {
        let valid = A2AMessageType::TaskAssignment {
            task_id: "t1".to_string(),
            instructions: "do something".to_string(),
            dependencies: vec!["t0".to_string()],
        };
        assert!(valid.validate().is_ok());

        let empty_id = A2AMessageType::TaskAssignment {
            task_id: "".to_string(),
            instructions: "test".to_string(),
            dependencies: vec![],
        };
        assert!(empty_id.validate().is_err());
    }

    #[test]
    fn test_task_progress_validation() {
        let valid = A2AMessageType::TaskProgress {
            task_id: "t1".to_string(),
            percentage: 50,
            current_step: "working".to_string(),
        };
        assert!(valid.validate().is_ok());

        let over_100 = A2AMessageType::TaskProgress {
            task_id: "t1".to_string(),
            percentage: 150,
            current_step: "working".to_string(),
        };
        assert!(over_100.validate().is_err());
    }

    #[test]
    fn test_task_completion_validation() {
        let valid = A2AMessageType::TaskCompletion {
            task_id: "t1".to_string(),
            result_json: "{\"status\":\"ok\"}".to_string(),
            artifacts: vec!["/path/to/artifact".to_string()],
        };
        assert!(valid.validate().is_ok());

        let invalid_json = A2AMessageType::TaskCompletion {
            task_id: "t1".to_string(),
            result_json: "{not json}".to_string(),
            artifacts: vec![],
        };
        assert!(invalid_json.validate().is_err());
    }

    #[test]
    fn test_packet_creation() {
        let msg = A2AMessageBuilder::task_assignment("t1", "test", vec![]);
        let packet = A2APacket::new("agent1".to_string(), "agent2".to_string(), msg);

        assert_eq!(packet.source_id, "agent1");
        assert_eq!(packet.target_id, "agent2");
        assert!(packet.timestamp > 0);
        assert!(packet.validate().is_ok());
    }

    #[test]
    fn test_packet_validation() {
        let msg = A2AMessageBuilder::task_assignment("t1", "test", vec![]);
        let valid = A2APacket::new("a1".to_string(), "a2".to_string(), msg.clone());
        assert!(valid.validate().is_ok());

        let empty_source = A2APacket {
            source_id: "".to_string(),
            target_id: "a2".to_string(),
            timestamp: 1000,
            message: msg.clone(),
        };
        assert!(empty_source.validate().is_err());

        let empty_target = A2APacket {
            source_id: "a1".to_string(),
            target_id: "".to_string(),
            timestamp: 1000,
            message: msg.clone(),
        };
        assert!(empty_target.validate().is_err());
    }

    #[test]
    fn test_packet_serialization() {
        let msg = A2AMessageBuilder::task_completion("t1", "{\"ok\":true}", vec![]);
        let packet = A2APacket::new("a1".to_string(), "a2".to_string(), msg);

        let json = packet.to_json().unwrap();
        let deserialized = A2APacket::from_json(&json).unwrap();

        assert_eq!(deserialized.source_id, packet.source_id);
        assert_eq!(deserialized.target_id, packet.target_id);
        assert_eq!(deserialized.timestamp, packet.timestamp);
    }

    #[test]
    fn test_packet_reply() {
        let original_msg = A2AMessageBuilder::task_assignment("t1", "test", vec![]);
        let original = A2APacket::new("planner".to_string(), "executor".to_string(), original_msg);

        let reply_msg = A2AMessageBuilder::task_progress("t1", 100, "done");
        let reply = original.reply(reply_msg);

        assert_eq!(reply.source_id, "executor");
        assert_eq!(reply.target_id, "planner");
    }

    #[test]
    fn test_helper_functions() {
        let task_id = A2AHelper::generate_task_id();
        assert!(task_id.starts_with("task_"));

        let agent_id = A2AHelper::generate_agent_id(AgentRole::Executor);
        assert!(agent_id.starts_with("executor_"));

        let msg = A2AMessageBuilder::task_assignment("test_task", "test", vec![]);
        let extracted = A2AHelper::extract_task_id(&msg);
        assert_eq!(extracted, Some("test_task".to_string()));

        assert!(A2AHelper::is_request(&msg));
        assert!(!A2AHelper::is_response(&msg));

        let progress = A2AMessageBuilder::task_progress("test_task", 50, "working");
        assert!(!A2AHelper::is_request(&progress));
        assert!(A2AHelper::is_response(&progress));
    }

    #[test]
    fn test_packet_summary() {
        let msg = A2AMessageBuilder::task_assignment("t1", "test", vec![]);
        let packet = A2APacket::with_timestamp(
            "agent1".to_string(),
            "agent2".to_string(),
            12345,
            msg,
        );

        let summary = packet.summary();
        assert!(summary.contains("agent1"));
        assert!(summary.contains("agent2"));
        assert!(summary.contains("task_assignment"));
        assert!(summary.contains("12345"));
    }
}
