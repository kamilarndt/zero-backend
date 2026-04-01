//! Performance benchmarks for ZeroClaw TUI
//!
//! This module provides benchmarks for measuring the performance of various TUI operations,
//! including rendering speed, update frequency, and memory usage.

use std::time::{Duration, Instant};
use std::sync::Arc;
use tokio::sync::Mutex;
use ratatui::backend::TestBackend;
use ratatui::Terminal;
use ratatui::layout::Rect;

use crate::state::AppState;
use crate::state::dirty_manager::DirtyManager;
use crate::state::cache::RequestCache;
use crate::state::connection_pool::ConnectionPool;

/// Benchmark results
#[derive(Debug, Clone)]
pub struct BenchmarkResults {
    /// Total execution time in milliseconds
    pub total_time_ms: u64,
    /// Number of operations performed
    pub operations: u64,
    /// Average time per operation in milliseconds
    pub avg_time_per_op_ms: f64,
    /// Memory usage in bytes (if available)
    pub memory_bytes: Option<u64>,
    /// CPU usage percentage (if available)
    pub cpu_percent: Option<f64>,
}

/// TUI performance benchmark suite
pub struct TuiBenchmarks {
    /// App state for testing
    pub app_state: Arc<Mutex<AppState>>,
    /// Dirty manager for testing
    pub dirty_manager: Arc<DirtyManager>,
    /// Cache for testing
    pub cache: Arc<RequestCache>,
    /// Connection pool for testing
    pub connection_pool: Arc<ConnectionPool>,
}

impl TuiBenchmarks {
    /// Create a new benchmark suite
    pub fn new() -> Self {
        let app_state = Arc::new(Mutex::new(AppState::default()));
        let dirty_manager = Arc::new(DirtyManager::new());
        let cache = Arc::new(RequestCache::new(Duration::from_secs(5)));
        let connection_pool = Arc::new(ConnectionPool::default());

        Self {
            app_state,
            dirty_manager,
            cache,
            connection_pool,
        }
    }

    /// Benchmark rendering performance
    pub async fn benchmark_rendering(&self, iterations: u64) -> BenchmarkResults {
        let mut backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        let start_time = Instant::now();

        for _ in 0..iterations {
            // Simulate a small change to force redraw
            let mut app = self.app_state.lock().await;
            app.dirty_manager.mark_all().await;
            drop(app);

            // Measure rendering time
            let render_start = Instant::now();

            // Create a dummy frame for benchmarking
            let size = terminal.size().unwrap();
            let mut frame = terminal.get_frame();

            // Import the rendering function
            use super::ui_conditional::render;
            render(&mut frame, &self.app_state.lock().await, &self.dirty_manager);

            let render_duration = render_start.elapsed();

            // Sleep to simulate real-world conditions
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        let total_time = start_time.elapsed();

        BenchmarkResults {
            total_time_ms: total_time.as_millis() as u64,
            operations: iterations,
            avg_time_per_op_ms: total_time.as_millis() as f64 / iterations as f64,
            memory_bytes: None, // Would need to implement memory tracking
            cpu_percent: None, // Would need to implement CPU tracking
        }
    }

    /// Benchmark dirty flag operations
    pub async fn benchmark_dirty_flags(&self, iterations: u64) -> BenchmarkResults {
        let start_time = Instant::now();

        for _ in 0..iterations {
            // Test marking all panels dirty
            self.dirty_manager.mark_all().await;

            // Test checking dirty flags
            let _ = self.dirty_manager.is_any_dirty().await;
            let _ = self.dirty_manager.is_swarm_dirty().await;
            let _ = self.dirty_manager.is_cost_dirty().await;
            let _ = self.dirty_manager.is_memory_dirty().await;
            let _ = self.dirty_manager.is_logs_dirty().await;

            // Test clearing flags
            self.dirty_manager.clear_all().await;

            // Test individual operations
            self.dirty_manager.mark_swarm().await;
            self.dirty_manager.mark_cost().await;
            self.dirty_manager.mark_memory().await;
            self.dirty_manager.mark_logs().await;
            self.dirty_manager.clear_swarm().await;
            self.dirty_manager.clear_cost().await;
            self.dirty_manager.clear_memory().await;
            self.dirty_manager.clear_logs().await;
        }

        let total_time = start_time.elapsed();

        BenchmarkResults {
            total_time_ms: total_time.as_millis() as u64,
            operations: iterations * 12, // 12 operations per iteration
            avg_time_per_op_ms: total_time.as_millis() as f64 / (iterations * 12) as f64,
            memory_bytes: None,
            cpu_percent: None,
        }
    }

    /// Benchmark cache operations
    pub async fn benchmark_cache(&self, iterations: u64) -> BenchmarkResults {
        let key = "benchmark_key";
        let test_data = serde_json::json!({"test": true, "value": 42});

        let start_time = Instant::now();

        for _ in 0..iterations {
            // Test cache get and mark
            let _ = self.cache.try_get_or_mark_pending(key);

            // Test cache put
            self.cache.put_and_ready(key, test_data.clone());

            // Test cache failure
            self.cache.mark_failed(key);
        }

        let total_time = start_time.elapsed();

        BenchmarkResults {
            total_time_ms: total_time.as_millis() as u64,
            operations: iterations * 3, // 3 operations per iteration
            avg_time_per_op_ms: total_time.as_millis() as f64 / (iterations * 3) as f64,
            memory_bytes: None,
            cpu_percent: None,
        }
    }

    /// Benchmark connection pool operations
    pub async fn benchmark_connection_pool(&self, iterations: u64) -> BenchmarkResults {
        let start_time = Instant::now();

        for _ in 0..iterations {
            // Get a connection
            let _conn = match self.connection_pool.get_client().await {
                Ok(conn) => conn,
                Err(_) => continue, // Skip if connection fails
            };

            // Don't actually use the connection to avoid network calls
            // The pool wrapper will be dropped when it goes out of scope
        }

        let total_time = start_time.elapsed();

        BenchmarkResults {
            total_time_ms: total_time.as_millis() as u64,
            operations: iterations,
            avg_time_per_op_ms: total_time.as_millis() as f64 / iterations as f64,
            memory_bytes: None,
            cpu_percent: None,
        }
    }

    /// Run all benchmarks and return comprehensive results
    pub async fn run_all_benchmarks(&self) -> BenchmarkSuite {
        let rendering = self.benchmark_rendering(100).await;
        let dirty_flags = self.benchmark_dirty_flags(1000).await;
        let cache = self.benchmark_cache(1000).await;
        let connection_pool = self.benchmark_connection_pool(100).await;

        BenchmarkSuite {
            rendering,
            dirty_flags,
            cache,
            connection_pool,
        }
    }
}

/// Comprehensive benchmark suite results
#[derive(Debug, Clone)]
pub struct BenchmarkSuite {
    pub rendering: BenchmarkResults,
    pub dirty_flags: BenchmarkResults,
    pub cache: BenchmarkResults,
    pub connection_pool: BenchmarkResults,
}

impl BenchmarkSuite {
    /// Print benchmark results in a human-readable format
    pub fn print_results(&self) {
        println!("=== ZeroClaw TUI Performance Benchmarks ===\n");

        println!("Rendering Performance:");
        println!("  Total time: {} ms", self.rendering.total_time_ms);
        println!("  Operations: {}", self.rendering.operations);
        println!("  Avg per operation: {:.3} ms\n", self.rendering.avg_time_per_op_ms);

        println!("Dirty Flags Performance:");
        println!("  Total time: {} ms", self.dirty_flags.total_time_ms);
        println!("  Operations: {}", self.dirty_flags.operations);
        println!("  Avg per operation: {:.3} ms\n", self.dirty_flags.avg_time_per_op_ms);

        println!("Cache Performance:");
        println!("  Total time: {} ms", self.cache.total_time_ms);
        println!("  Operations: {}", self.cache.operations);
        println!("  Avg per operation: {:.3} ms\n", self.cache.avg_time_per_op_ms);

        println!("Connection Pool Performance:");
        println!("  Total time: {} ms", self.connection_pool.total_time_ms);
        println!("  Operations: {}", self.connection_pool.operations);
        println!("  Avg per operation: {:.3} ms\n", self.connection_pool.avg_time_per_op_ms);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_benchmark_creation() {
        let benchmarks = TuiBenchmarks::new();
        assert!(!benchmarks.app_state.is_empty());
        assert!(!benchmarks.dirty_manager.is_any_dirty().await);
    }

    #[tokio::test]
    async fn test_dirty_flag_benchmark() {
        let benchmarks = TuiBenchmarks::new();
        let results = benchmarks.benchmark_dirty_flags(10).await;

        assert!(results.total_time_ms > 0);
        assert!(results.operations > 0);
        assert!(results.avg_time_per_op_ms > 0.0);
    }

    #[tokio::test]
    async fn test_cache_benchmark() {
        let benchmarks = TuiBenchmarks::new();
        let results = benchmarks.benchmark_cache(10).await;

        assert!(results.total_time_ms > 0);
        assert!(results.operations > 0);
        assert!(results.avg_time_per_op_ms > 0.0);
    }

    #[tokio::test]
    async fn test_connection_pool_benchmark() {
        let benchmarks = TuiBenchmarks::new();
        let results = benchmarks.benchmark_connection_pool(10).await;

        assert!(results.total_time_ms > 0);
        assert!(results.operations > 0);
        assert!(results.avg_time_per_op_ms > 0.0);
    }

    #[tokio::test]
    async fn test_full_benchmark_suite() {
        let benchmarks = TuiBenchmarks::new();
        let suite = benchmarks.run_all_benchmarks().await;

        // Verify all benchmarks were run
        assert!(suite.rendering.total_time_ms > 0);
        assert!(suite.dirty_flags.total_time_ms > 0);
        assert!(suite.cache.total_time_ms > 0);
        assert!(suite.connection_pool.total_time_ms > 0);

        // Print results for manual inspection
        suite.print_results();
    }
}
