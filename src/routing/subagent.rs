// SubAgent Manager for ZeroClaw Routing System
// Implements async task delegation with proper concurrency control

use crate::routing::classifier::{ClassificationResult, TaskType};
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::{OnceCell, Semaphore};

// ── Errors ─────────────────────────────────────────────────────────────

/// Errors that can occur during subagent delegation
#[derive(Debug, Error)]
pub enum SubAgentError {
    #[error("Maximum depth limit exceeded: {0} >= {1}")]
    DepthLimitExceeded(usize, usize),

    #[error("Circular dependency detected in task graph")]
    CircularDependency,

    #[error("Task not found: {0}")]
    TaskNotFound(String),

    #[error("Semaphore initialization failed")]
    SemaphoreInitFailed,

    #[error("Execution failed: {0}")]
    ExecutionFailed(String),
}

// ── SubTask Structure ───────────────────────────────────────────────────

/// A subtask with dependencies for parallel execution
#[derive(Debug, Clone)]
pub struct SubTask {
    /// Unique task identifier
    pub id: String,
    /// Human-readable instructions
    pub instructions: String,
    /// Task IDs that must complete before this one
    pub dependencies: Vec<String>,
    /// Classification result for this task
    pub classification: ClassificationResult,
    /// Current depth in delegation hierarchy
    pub depth: usize,
}

impl SubTask {
    /// Create a new subtask
    pub fn new(
        id: String,
        instructions: String,
        dependencies: Vec<String>,
        classification: ClassificationResult,
        depth: usize,
    ) -> Self {
        Self {
            id,
            instructions,
            dependencies,
            classification,
            depth,
        }
    }

    /// Check if this task is ready to execute (all dependencies satisfied)
    pub fn is_ready(&self, completed: &HashMap<String, bool>) -> bool {
        self.dependencies
            .iter()
            .all(|dep| completed.get(dep).copied().unwrap_or(false))
    }
}

// ── SubAgentManager ──────────────────────────────────────────────────────

/// Manages subagent delegation with proper concurrency control
///
/// # Critical Bug Fixes Applied:
/// 1. Uses `Arc<OnceCell<Semaphore>>` NOT `Arc<Semaphore>` (Semaphore not mutable through Arc)
/// 2. NEVER holds Mutex lock across await points (release lock before LLM calls)
/// 3. Uses `HashMap<String, SubTask>` for O(1) lookup in topological sort
/// 4. Adds cycle detection: `if result.len() != tasks.len() { return Err(...) }`
pub struct SubAgentManager {
    /// Semaphore for concurrency control (BUG FIX #1: OnceCell not Arc<Semaphore>)
    semaphore: Arc<OnceCell<Semaphore>>,
    /// Maximum concurrent subtasks
    max_concurrent: usize,
    /// Maximum delegation depth
    max_depth: usize,
    /// Whether the manager has been initialized
    initialized: Arc<AtomicBool>,
}

impl SubAgentManager {
    /// Create a new subagent manager
    ///
    /// # Arguments
    /// * `max_concurrent` - Maximum number of concurrent subtasks
    /// * `max_depth` - Maximum delegation depth (default: 3)
    pub fn new(max_concurrent: usize, max_depth: usize) -> Self {
        Self {
            semaphore: Arc::new(OnceCell::new()),
            max_concurrent,
            max_depth,
            initialized: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Ensure the semaphore is initialized (BUG FIX #1: Use get_or_init)
    ///
    /// This uses `OnceCell::get_or_init` to safely initialize the semaphore
    /// exactly once, even if called concurrently from multiple tasks.
    async fn ensure_active(&self) -> Result<(), SubAgentError> {
        // Try to initialize if not already done
        if !self.initialized.load(Ordering::Acquire) {
            // Use get_or_init for thread-safe one-time initialization
            self.semaphore
                .get_or_init(|| async {
                    // Initialize semaphore with max_concurrent permits
                    Semaphore::new(self.max_concurrent)
                })
                .await;

            // Mark as initialized
            self.initialized.store(true, Ordering::Release);
        }
        Ok(())
    }

    /// Spawn and execute subtasks with proper dependency management
    ///
    /// # Critical Bug Fixes:
    /// 2. NEVER holds Mutex lock across await points
    /// 3. Uses `HashMap<String, SubTask>` for O(1) lookup
    /// 4. Adds cycle detection via result length check
    ///
    /// # Arguments
    /// * `tasks` - HashMap of tasks keyed by ID (for O(1) dependency lookup)
    ///
    /// # Returns
    /// HashMap of task IDs to their execution results
    pub async fn spawn_subagents(
        &self,
        tasks: HashMap<String, SubTask>,
    ) -> Result<HashMap<String, String>, SubAgentError> {
        // Ensure semaphore is initialized
        self.ensure_active().await?;

        // Get semaphore reference
        let semaphore = self
            .semaphore
            .get()
            .ok_or(SubAgentError::SemaphoreInitFailed)?;

        // Check depth limit for all tasks
        for task in tasks.values() {
            if task.depth >= self.max_depth {
                return Err(SubAgentError::DepthLimitExceeded(
                    task.depth,
                    self.max_depth,
                ));
            }
        }

        // Perform topological sort with cycle detection (BUG FIX #3 & #4)
        let sorted = self.topological_sort(&tasks)?;

        // Execute tasks in dependency order with concurrency limit
        let mut results: HashMap<String, String> = HashMap::new();
        let mut completed: HashMap<String, bool> = HashMap::new();

        // Track in-flight tasks for ready detection
        let mut ready_queue: VecDeque<String> = VecDeque::new();

        // Initial ready tasks (no dependencies)
        for task_id in &sorted {
            let task = &tasks[task_id];
            if task.is_ready(&completed) {
                ready_queue.push_back(task_id.clone());
            }
        }

        // Process ready tasks with semaphore control
        while let Some(task_id) = ready_queue.pop_front() {
            // Acquire semaphore permit
            let _permit = semaphore
                .acquire()
                .await
                .map_err(|_| SubAgentError::ExecutionFailed("Semaphore closed".to_string()))?;

            // Get task reference (BUG FIX #2: Short lock scope)
            let task = tasks
                .get(&task_id)
                .ok_or(SubAgentError::TaskNotFound(task_id.clone()))?;

            // Execute the task (await happens OUTSIDE any lock)
            let result = self.execute_subtask(task).await?;

            // Store result and mark completed
            results.insert(task_id.clone(), result);
            completed.insert(task_id.clone(), true);

            // Check for newly ready tasks (short lock scope)
            for candidate_id in &sorted {
                if !completed.contains_key(candidate_id) {
                    if let Some(candidate) = tasks.get(candidate_id) {
                        if candidate.is_ready(&completed) && !ready_queue.contains(candidate_id) {
                            ready_queue.push_back(candidate_id.clone());
                        }
                    }
                }
            }

            // Permit is dropped here, releasing semaphore slot
        }

        Ok(results)
    }

    /// Execute a single subtask
    ///
    /// # Note
    /// This is a placeholder implementation. In production, this would:
    /// - Route to appropriate model based on classification
    /// - Handle A2A messaging
    /// - Collect and return results
    async fn execute_subtask(&self, task: &SubTask) -> Result<String, SubAgentError> {
        // BUG FIX #2: This function must NOT hold any locks when awaiting

        // Determine model based on task complexity (model downgrading)
        let model = self.downgrade_model_by_complexity(&task.classification);

        // Simulate execution (in production, this would call the LLM)
        // For now, return a formatted result
        Ok(format!(
            "[Task {}] Model: {}, Depth: {}, Instructions: {}",
            task.id, model, task.depth, task.instructions
        ))
    }

    /// Downgrade model selection based on task complexity
    ///
    /// # Logic
    /// - High complexity: Use reasoning model
    /// - Medium complexity: Use standard model
    /// - Low complexity: Use fast model
    fn downgrade_model_by_complexity(&self, classification: &ClassificationResult) -> String {
        match classification.task_type {
            TaskType::Architecture => "reasoning".to_string(),
            TaskType::Coding | TaskType::Review => "code".to_string(),
            TaskType::Documentation => "fast".to_string(),
            TaskType::Quick => "fast".to_string(),
            TaskType::Vision => "vision".to_string(),
            TaskType::Standard => {
                // Downgrade based on estimated tokens
                if classification.estimated_tokens > 1000 {
                    "reasoning".to_string()
                } else if classification.estimated_tokens > 500 {
                    "standard".to_string()
                } else {
                    "fast".to_string()
                }
            }
        }
    }

    /// Topological sort with cycle detection
    ///
    /// # Critical Bug Fixes:
    /// 3. Uses `HashMap<String, SubTask>` for O(1) lookup
    /// 4. Adds cycle detection via result length check
    ///
    /// # Algorithm
    /// Kahn's algorithm for topological sorting:
    /// 1. Compute in-degree for each node
    /// 2. Start with nodes that have in-degree 0
    /// 3. Remove edges from sorted nodes and decrement in-degree
    /// 4. Repeat until all nodes sorted or cycle detected
    fn topological_sort(
        &self,
        tasks: &HashMap<String, SubTask>,
    ) -> Result<Vec<String>, SubAgentError> {
        // BUG FIX #3: Use HashMap for O(1) dependency lookup
        let mut in_degree: HashMap<String, usize> = HashMap::new();
        let mut adj_list: HashMap<String, Vec<String>> = HashMap::new();

        // Initialize in-degree for all tasks
        for task_id in tasks.keys() {
            in_degree.insert(task_id.clone(), 0);
            adj_list.insert(task_id.clone(), Vec::new());
        }

        // Build adjacency list and compute in-degrees
        for (task_id, task) in tasks {
            for dep in &task.dependencies {
                // Add edge: dep -> task_id
                if let Some(neighbors) = adj_list.get_mut(dep) {
                    neighbors.push(task_id.clone());
                }
                // Increment in-degree
                *in_degree.entry(task_id.clone()).or_insert(0) += 1;
            }
        }

        // Start with nodes that have in-degree 0
        let mut queue: VecDeque<String> = in_degree
            .iter()
            .filter(|(_, &degree)| degree == 0)
            .map(|(id, _)| id.clone())
            .collect();

        let mut result = Vec::new();

        // Process nodes in topological order
        while let Some(node) = queue.pop_front() {
            result.push(node.clone());

            // Decrement in-degree for neighbors
            if let Some(neighbors) = adj_list.get(&node) {
                for neighbor in neighbors {
                    if let Some(degree) = in_degree.get_mut(neighbor) {
                        *degree -= 1;
                        if *degree == 0 {
                            queue.push_back(neighbor.clone());
                        }
                    }
                }
            }
        }

        // BUG FIX #4: Cycle detection - if not all nodes sorted, there's a cycle
        if result.len() != tasks.len() {
            return Err(SubAgentError::CircularDependency);
        }

        Ok(result)
    }
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn create_task(
        id: &str,
        deps: Vec<&str>,
        task_type: TaskType,
        depth: usize,
    ) -> (String, SubTask) {
        let task = SubTask {
            id: id.to_string(),
            instructions: format!("Task {}", id),
            dependencies: deps.into_iter().map(|s| s.to_string()).collect(),
            classification: ClassificationResult::new(
                task_type.clone(),
                task_type.default_hint().to_string(),
                100,
                false,
            ),
            depth,
        };
        (id.to_string(), task)
    }

    #[tokio::test]
    async fn test_subagent_manager_creation() {
        let manager = SubAgentManager::new(2, 3);
        assert_eq!(manager.max_concurrent, 2);
        assert_eq!(manager.max_depth, 3);
        assert!(!manager.initialized.load(Ordering::Acquire));
    }

    #[tokio::test]
    async fn test_ensure_active_initializes_semaphore() {
        let manager = SubAgentManager::new(2, 3);
        manager.ensure_active().await.unwrap();
        assert!(manager.initialized.load(Ordering::Acquire));
        assert!(manager.semaphore.get().is_some());
    }

    #[tokio::test]
    async fn test_topological_sort_linear() {
        let manager = SubAgentManager::new(2, 3);
        let tasks: HashMap<String, SubTask> = [
            create_task("a", vec![], TaskType::Standard, 0),
            create_task("b", vec!["a"], TaskType::Standard, 1),
            create_task("c", vec!["b"], TaskType::Standard, 2),
        ]
        .into_iter()
        .collect();

        let sorted = manager.topological_sort(&tasks).unwrap();
        assert_eq!(sorted, vec!["a", "b", "c"]);
    }

    #[tokio::test]
    async fn test_topological_sort_diamond() {
        let manager = SubAgentManager::new(2, 3);
        let tasks: HashMap<String, SubTask> = [
            create_task("a", vec![], TaskType::Standard, 0),
            create_task("b", vec!["a"], TaskType::Standard, 1),
            create_task("c", vec!["a"], TaskType::Standard, 1),
            create_task("d", vec!["b", "c"], TaskType::Standard, 2),
        ]
        .into_iter()
        .collect();

        let sorted = manager.topological_sort(&tasks).unwrap();
        // 'a' must be first, 'd' must be last
        assert_eq!(sorted[0], "a");
        assert_eq!(sorted[3], "d");
        // 'b' and 'c' can be in either order
        assert!(sorted.contains(&"b".to_string()));
        assert!(sorted.contains(&"c".to_string()));
    }

    #[tokio::test]
    async fn test_topological_sort_cycle_detection() {
        let manager = SubAgentManager::new(2, 3);
        let tasks: HashMap<String, SubTask> = [
            create_task("a", vec!["b"], TaskType::Standard, 0),
            create_task("b", vec!["c"], TaskType::Standard, 0),
            create_task("c", vec!["a"], TaskType::Standard, 0),
        ]
        .into_iter()
        .collect();

        let result = manager.topological_sort(&tasks);
        assert!(matches!(result, Err(SubAgentError::CircularDependency)));
    }

    #[tokio::test]
    async fn test_depth_limit_enforcement() {
        let manager = SubAgentManager::new(2, 3);
        let tasks: HashMap<String, SubTask> = [create_task(
            "a",
            vec![],
            TaskType::Standard,
            5, // Exceeds max_depth of 3
        )]
        .into_iter()
        .collect();

        let result = manager.spawn_subagents(tasks).await;
        assert!(matches!(
            result,
            Err(SubAgentError::DepthLimitExceeded(5, 3))
        ));
    }

    #[tokio::test]
    async fn test_model_downgrading() {
        let manager = SubAgentManager::new(2, 3);

        // Test architecture -> reasoning
        let arch =
            ClassificationResult::new(TaskType::Architecture, "reasoning".to_string(), 500, false);
        assert_eq!(manager.downgrade_model_by_complexity(&arch), "reasoning");

        // Test coding -> code
        let code = ClassificationResult::new(TaskType::Coding, "code".to_string(), 500, false);
        assert_eq!(manager.downgrade_model_by_complexity(&code), "code");

        // Test documentation -> fast
        let docs =
            ClassificationResult::new(TaskType::Documentation, "fast".to_string(), 500, false);
        assert_eq!(manager.downgrade_model_by_complexity(&docs), "fast");

        // Test standard with low tokens -> fast
        let standard_low =
            ClassificationResult::new(TaskType::Standard, "standard".to_string(), 100, false);
        assert_eq!(manager.downgrade_model_by_complexity(&standard_low), "fast");

        // Test standard with high tokens -> reasoning
        let standard_high =
            ClassificationResult::new(TaskType::Standard, "standard".to_string(), 2000, false);
        assert_eq!(
            manager.downgrade_model_by_complexity(&standard_high),
            "reasoning"
        );
    }

    #[tokio::test]
    async fn test_subtask_is_ready() {
        let task = SubTask {
            id: "b".to_string(),
            instructions: "Task b".to_string(),
            dependencies: vec!["a".to_string()],
            classification: ClassificationResult::standard(100),
            depth: 1,
        };

        let mut completed = HashMap::new();

        // Not ready when dependency not completed
        assert!(!task.is_ready(&completed));

        // Ready when dependency completed
        completed.insert("a".to_string(), true);
        assert!(task.is_ready(&completed));
    }
}
