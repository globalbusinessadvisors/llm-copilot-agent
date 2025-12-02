//! Ingestion Pipeline Benchmark Adapters
//!
//! Exposes document ingestion operations as benchmark targets.

use async_trait::async_trait;
use std::time::Instant;
use crate::result::BenchmarkResult;
use crate::traits::BenchTarget;

/// Benchmark for document ingestion
pub struct DocumentIngestionBenchmark {
    id: String,
    document_count: usize,
    avg_document_size: usize,
}

impl DocumentIngestionBenchmark {
    pub fn new() -> Self {
        Self {
            id: "ingestion::document".to_string(),
            document_count: 50,
            avg_document_size: 5000,
        }
    }
}

impl Default for DocumentIngestionBenchmark {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl BenchTarget for DocumentIngestionBenchmark {
    fn id(&self) -> &str {
        &self.id
    }

    fn description(&self) -> Option<&str> {
        Some("Benchmarks document ingestion pipeline throughput")
    }

    fn expected_duration_ms(&self) -> Option<(u64, u64)> {
        Some((500, 5000))
    }

    async fn run(&self) -> BenchmarkResult {
        let start = Instant::now();

        let mut ingestion_times = Vec::new();
        let mut total_bytes_processed = 0;
        let mut total_chunks_created = 0;

        for doc_idx in 0..self.document_count {
            let doc_start = Instant::now();

            // Generate mock document content
            let content = generate_mock_document(self.avg_document_size);
            total_bytes_processed += content.len();

            // Simulate document ingestion
            let chunks = simulate_document_ingestion(&content, doc_idx).await;
            total_chunks_created += chunks;

            ingestion_times.push(doc_start.elapsed().as_millis());
        }

        let total_duration = start.elapsed();

        BenchmarkResult::new(
            &self.id,
            serde_json::json!({
                "success": true,
                "duration_ms": total_duration.as_millis() as u64,
                "documents_processed": self.document_count,
                "total_bytes": total_bytes_processed,
                "total_chunks": total_chunks_created,
                "avg_ingestion_ms": ingestion_times.iter().sum::<u128>() as f64 / ingestion_times.len() as f64,
                "throughput_docs_per_sec": self.document_count as f64 / total_duration.as_secs_f64(),
                "throughput_mb_per_sec": (total_bytes_processed as f64 / 1_000_000.0) / total_duration.as_secs_f64()
            }),
        )
    }
}

/// Benchmark for chunk processing
pub struct ChunkProcessingBenchmark {
    id: String,
    chunk_sizes: Vec<usize>,
}

impl ChunkProcessingBenchmark {
    pub fn new() -> Self {
        Self {
            id: "ingestion::chunking".to_string(),
            chunk_sizes: vec![512, 1024, 2048, 4096],
        }
    }
}

impl Default for ChunkProcessingBenchmark {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl BenchTarget for ChunkProcessingBenchmark {
    fn id(&self) -> &str {
        &self.id
    }

    fn description(&self) -> Option<&str> {
        Some("Benchmarks text chunking with varying chunk sizes")
    }

    fn expected_duration_ms(&self) -> Option<(u64, u64)> {
        Some((100, 1000))
    }

    async fn run(&self) -> BenchmarkResult {
        let start = Instant::now();

        // Generate a large document to chunk
        let document = generate_mock_document(50000);
        let mut chunk_results = Vec::new();

        for &chunk_size in &self.chunk_sizes {
            let chunk_start = Instant::now();

            let chunks = simulate_text_chunking(&document, chunk_size);
            let chunk_duration = chunk_start.elapsed();

            chunk_results.push(serde_json::json!({
                "chunk_size": chunk_size,
                "chunks_created": chunks.len(),
                "duration_ms": chunk_duration.as_millis(),
                "chunks_per_second": chunks.len() as f64 / chunk_duration.as_secs_f64()
            }));
        }

        let total_duration = start.elapsed();

        BenchmarkResult::new(
            &self.id,
            serde_json::json!({
                "success": true,
                "duration_ms": total_duration.as_millis() as u64,
                "document_size_bytes": document.len(),
                "chunk_sizes_tested": self.chunk_sizes,
                "results": chunk_results
            }),
        )
    }
}

// Helper functions

fn generate_mock_document(size: usize) -> String {
    let paragraph = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. \
        Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. \
        Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris. ";

    let mut content = String::with_capacity(size);
    while content.len() < size {
        content.push_str(paragraph);
    }
    content.truncate(size);
    content
}

async fn simulate_document_ingestion(content: &str, _doc_idx: usize) -> usize {
    tokio::task::yield_now().await;

    // Simulate chunking
    let chunk_size = 1024;
    let chunks = (content.len() + chunk_size - 1) / chunk_size;

    std::hint::black_box(content.len());

    chunks
}

fn simulate_text_chunking(content: &str, chunk_size: usize) -> Vec<&str> {
    let mut chunks = Vec::new();
    let mut start = 0;

    while start < content.len() {
        let end = (start + chunk_size).min(content.len());
        chunks.push(&content[start..end]);
        start = end;
    }

    chunks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_document_ingestion_benchmark() {
        let benchmark = DocumentIngestionBenchmark::new();
        assert_eq!(benchmark.id(), "ingestion::document");

        let result = benchmark.run().await;
        assert!(result.is_success());
    }

    #[tokio::test]
    async fn test_chunk_processing_benchmark() {
        let benchmark = ChunkProcessingBenchmark::new();
        let result = benchmark.run().await;
        assert!(result.is_success());
    }
}
