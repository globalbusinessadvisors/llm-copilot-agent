//! Benchmark adapters module
//!
//! This module contains adapters that expose CoPilot-Agent operations
//! as benchmark targets through the canonical BenchTarget trait.

pub mod intent_classification;
pub mod context_retrieval;
pub mod conversation;
pub mod workflow;
pub mod sandbox_execution;
pub mod ingestion;
pub mod observability;

use crate::traits::BoxedBenchTarget;

/// Returns all registered benchmark targets
///
/// This is the canonical registry function that returns all available
/// benchmark targets as a Vec<Box<dyn BenchTarget>>.
pub fn all_targets() -> Vec<BoxedBenchTarget> {
    let mut targets: Vec<BoxedBenchTarget> = Vec::new();

    // Intent Classification benchmarks
    targets.push(Box::new(intent_classification::SimpleIntentBenchmark::new()));
    targets.push(Box::new(intent_classification::ComplexIntentBenchmark::new()));
    targets.push(Box::new(intent_classification::BatchIntentBenchmark::new()));

    // Context Retrieval benchmarks
    targets.push(Box::new(context_retrieval::SimpleRetrievalBenchmark::new()));
    targets.push(Box::new(context_retrieval::LargeCorpusRetrievalBenchmark::new()));

    // Conversation benchmarks
    targets.push(Box::new(conversation::SimpleResponseBenchmark::new()));
    targets.push(Box::new(conversation::MultiTurnBenchmark::new()));

    // Workflow benchmarks
    targets.push(Box::new(workflow::WorkflowExecutionBenchmark::new()));
    targets.push(Box::new(workflow::WorkflowValidationBenchmark::new()));

    // Sandbox Execution benchmarks
    targets.push(Box::new(sandbox_execution::PythonExecutionBenchmark::new()));
    targets.push(Box::new(sandbox_execution::NodeExecutionBenchmark::new()));

    // Ingestion benchmarks
    targets.push(Box::new(ingestion::DocumentIngestionBenchmark::new()));
    targets.push(Box::new(ingestion::ChunkProcessingBenchmark::new()));

    // Observability benchmarks
    targets.push(Box::new(observability::MetricsCollectionBenchmark::new()));
    targets.push(Box::new(observability::TracingBenchmark::new()));

    targets
}

/// Returns the count of all registered benchmark targets
pub fn target_count() -> usize {
    all_targets().len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_targets_not_empty() {
        let targets = all_targets();
        assert!(!targets.is_empty());
    }

    #[test]
    fn test_all_targets_have_unique_ids() {
        let targets = all_targets();
        let mut ids: Vec<&str> = targets.iter().map(|t| t.id()).collect();
        let original_len = ids.len();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), original_len, "All target IDs should be unique");
    }

    #[tokio::test]
    async fn test_all_targets_can_run() {
        let targets = all_targets();
        for target in targets.iter().take(3) {
            // Test first few to verify they can run
            let result = target.run().await;
            assert!(!result.target_id.is_empty());
        }
    }
}
