//! Conversation Benchmark Adapters
//!
//! Exposes conversation management operations as benchmark targets.

use async_trait::async_trait;
use std::time::Instant;
use crate::result::BenchmarkResult;
use crate::traits::BenchTarget;

/// Benchmark for simple response generation
pub struct SimpleResponseBenchmark {
    id: String,
    iterations: usize,
}

impl SimpleResponseBenchmark {
    pub fn new() -> Self {
        Self {
            id: "conversation::response::simple".to_string(),
            iterations: 20,
        }
    }
}

impl Default for SimpleResponseBenchmark {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl BenchTarget for SimpleResponseBenchmark {
    fn id(&self) -> &str {
        &self.id
    }

    fn description(&self) -> Option<&str> {
        Some("Benchmarks simple response generation without context")
    }

    fn expected_duration_ms(&self) -> Option<(u64, u64)> {
        Some((50, 500))
    }

    async fn run(&self) -> BenchmarkResult {
        let start = Instant::now();

        let prompts = vec![
            "What is the current CPU usage?",
            "Show me the error count",
            "List active services",
            "Describe the deployment status",
        ];

        let mut response_times = Vec::new();
        let mut total_tokens = 0;

        for _ in 0..self.iterations {
            for prompt in &prompts {
                let response_start = Instant::now();
                let (response, tokens) = simulate_response_generation(prompt, &[]).await;
                response_times.push(response_start.elapsed().as_micros());
                total_tokens += tokens;
                std::hint::black_box(response);
            }
        }

        let total_duration = start.elapsed();
        let total_responses = self.iterations * prompts.len();

        BenchmarkResult::new(
            &self.id,
            serde_json::json!({
                "success": true,
                "duration_ms": total_duration.as_millis() as u64,
                "iterations": self.iterations,
                "total_responses": total_responses,
                "total_tokens": total_tokens,
                "avg_response_time_us": response_times.iter().sum::<u128>() as f64 / response_times.len() as f64,
                "tokens_per_second": total_tokens as f64 / total_duration.as_secs_f64()
            }),
        )
    }
}

/// Benchmark for multi-turn conversation
pub struct MultiTurnBenchmark {
    id: String,
    turns: usize,
    conversations: usize,
}

impl MultiTurnBenchmark {
    pub fn new() -> Self {
        Self {
            id: "conversation::multi_turn".to_string(),
            turns: 5,
            conversations: 10,
        }
    }
}

impl Default for MultiTurnBenchmark {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl BenchTarget for MultiTurnBenchmark {
    fn id(&self) -> &str {
        &self.id
    }

    fn description(&self) -> Option<&str> {
        Some("Benchmarks multi-turn conversation handling with context accumulation")
    }

    fn expected_duration_ms(&self) -> Option<(u64, u64)> {
        Some((200, 2000))
    }

    async fn run(&self) -> BenchmarkResult {
        let start = Instant::now();

        let turn_sequence = vec![
            "Show me the service health",
            "What about the database connections?",
            "Are there any errors?",
            "Compare with yesterday",
            "Generate a summary report",
        ];

        let mut conversation_times = Vec::new();
        let mut total_context_growth = 0;

        for _ in 0..self.conversations {
            let conv_start = Instant::now();
            let mut context = Vec::new();

            for (turn_idx, prompt) in turn_sequence.iter().take(self.turns).enumerate() {
                let (response, _tokens) = simulate_response_generation(prompt, &context).await;

                // Accumulate context
                context.push(format!("User: {}", prompt));
                context.push(format!("Assistant: {}", response));

                total_context_growth += context.len();
                std::hint::black_box(turn_idx);
            }

            conversation_times.push(conv_start.elapsed().as_millis());
        }

        let total_duration = start.elapsed();

        BenchmarkResult::new(
            &self.id,
            serde_json::json!({
                "success": true,
                "duration_ms": total_duration.as_millis() as u64,
                "conversations": self.conversations,
                "turns_per_conversation": self.turns,
                "total_turns": self.conversations * self.turns,
                "avg_conversation_ms": conversation_times.iter().sum::<u128>() as f64 / conversation_times.len() as f64,
                "avg_context_growth": total_context_growth as f64 / (self.conversations * self.turns) as f64
            }),
        )
    }
}

// Simulation functions

async fn simulate_response_generation(prompt: &str, context: &[String]) -> (String, usize) {
    // Simulate async processing
    tokio::task::yield_now().await;

    // Simulate work proportional to input size
    let input_size = prompt.len() + context.iter().map(|c| c.len()).sum::<usize>();
    std::hint::black_box(input_size);

    // Generate mock response
    let response = format!(
        "Based on your query about '{}' with {} context items, here is the response.",
        &prompt[..prompt.len().min(30)],
        context.len()
    );

    let tokens = response.split_whitespace().count();

    (response, tokens)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_simple_response_benchmark() {
        let benchmark = SimpleResponseBenchmark::new();
        assert_eq!(benchmark.id(), "conversation::response::simple");

        let result = benchmark.run().await;
        assert!(result.is_success());
    }

    #[tokio::test]
    async fn test_multi_turn_benchmark() {
        let benchmark = MultiTurnBenchmark::new();
        let result = benchmark.run().await;
        assert!(result.is_success());
    }
}
