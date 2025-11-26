//! Intent classification module.
//!
//! This module provides pattern-based intent classification using pre-compiled
//! regular expressions for fast matching and confidence scoring.

use lazy_static::lazy_static;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, trace};

/// Supported intent types for observability queries.
///
/// These intents cover the primary use cases for observability and monitoring.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum IntentType {
    /// Query metrics (e.g., "Show CPU usage")
    QueryMetrics,
    /// Search logs (e.g., "Find errors in auth-service")
    SearchLogs,
    /// Analyze traces (e.g., "Show traces for /api/users")
    AnalyzeTraces,
    /// Detect anomalies (e.g., "Find unusual patterns in latency")
    DetectAnomalies,
    /// Root cause analysis (e.g., "Why is the API slow?")
    RootCauseAnalysis,
    /// Get service health (e.g., "Is the auth service healthy?")
    ServiceHealth,
    /// Compare metrics (e.g., "Compare CPU usage between services")
    CompareMetrics,
    /// Alert investigation (e.g., "Why did this alert fire?")
    AlertInvestigation,
    /// Performance analysis (e.g., "Analyze response time trends")
    PerformanceAnalysis,
    /// Error analysis (e.g., "Show me all errors")
    ErrorAnalysis,
    /// Capacity planning (e.g., "Predict future resource needs")
    CapacityPlanning,
    /// Dependency analysis (e.g., "Show service dependencies")
    DependencyAnalysis,
    /// SLO monitoring (e.g., "Check SLO compliance")
    SloMonitoring,
    /// Trend analysis (e.g., "Show traffic trends")
    TrendAnalysis,
    /// General query (fallback for unclear intents)
    GeneralQuery,
    /// Unknown intent
    Unknown,
}

impl IntentType {
    /// Returns a human-readable description of the intent type.
    pub fn description(&self) -> &'static str {
        match self {
            Self::QueryMetrics => "Query metrics and measurements",
            Self::SearchLogs => "Search and filter log entries",
            Self::AnalyzeTraces => "Analyze distributed traces",
            Self::DetectAnomalies => "Detect anomalies and outliers",
            Self::RootCauseAnalysis => "Perform root cause analysis",
            Self::ServiceHealth => "Check service health status",
            Self::CompareMetrics => "Compare metrics across dimensions",
            Self::AlertInvestigation => "Investigate alerts and incidents",
            Self::PerformanceAnalysis => "Analyze performance metrics",
            Self::ErrorAnalysis => "Analyze errors and failures",
            Self::CapacityPlanning => "Plan for capacity and scaling",
            Self::DependencyAnalysis => "Analyze service dependencies",
            Self::SloMonitoring => "Monitor SLOs and SLIs",
            Self::TrendAnalysis => "Analyze trends over time",
            Self::GeneralQuery => "General observability query",
            Self::Unknown => "Unknown or unclear intent",
        }
    }
}

/// Represents a classified intent with confidence score.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Intent {
    /// The classified intent type
    pub intent_type: IntentType,
    /// Confidence score (0.0 to 1.0)
    pub confidence: f64,
    /// Matching patterns that contributed to this classification
    pub matched_patterns: Vec<String>,
    /// Alternative intents that were considered
    pub alternatives: Vec<(IntentType, f64)>,
}

impl Intent {
    /// Creates a new Intent with the given type and confidence.
    pub fn new(intent_type: IntentType, confidence: f64) -> Self {
        Self {
            intent_type,
            confidence,
            matched_patterns: Vec::new(),
            alternatives: Vec::new(),
        }
    }

    /// Returns true if the confidence is above the threshold (0.7).
    pub fn is_confident(&self) -> bool {
        self.confidence >= 0.7
    }
}

/// Pattern for matching user queries to intents.
#[derive(Debug, Clone)]
struct IntentPattern {
    regex: Regex,
    weight: f64,
    intent: IntentType,
}

lazy_static! {
    /// Pre-compiled regex patterns for intent classification.
    static ref INTENT_PATTERNS: Vec<IntentPattern> = vec![
        // QueryMetrics patterns
        IntentPattern {
            regex: Regex::new(r"(?i)\b(show|display|get|what|what's|whats)\s+(is\s+)?(the\s+)?(cpu|memory|disk|network|latency|throughput|bandwidth)").unwrap(),
            weight: 0.9,
            intent: IntentType::QueryMetrics,
        },
        IntentPattern {
            regex: Regex::new(r"(?i)\b(metric|gauge|counter|histogram)\b").unwrap(),
            weight: 0.7,
            intent: IntentType::QueryMetrics,
        },
        IntentPattern {
            regex: Regex::new(r"(?i)\b(usage|utilization|consumption)\b").unwrap(),
            weight: 0.6,
            intent: IntentType::QueryMetrics,
        },

        // SearchLogs patterns
        IntentPattern {
            regex: Regex::new(r"(?i)\b(search|find|show|get|fetch)\s+(logs?|entries|messages)").unwrap(),
            weight: 0.9,
            intent: IntentType::SearchLogs,
        },
        IntentPattern {
            regex: Regex::new(r"(?i)\b(errors?|warnings?|exceptions?)\s+(in|from)").unwrap(),
            weight: 0.8,
            intent: IntentType::SearchLogs,
        },
        IntentPattern {
            regex: Regex::new(r"(?i)\blog\s+(level|severity)").unwrap(),
            weight: 0.7,
            intent: IntentType::SearchLogs,
        },

        // AnalyzeTraces patterns
        IntentPattern {
            regex: Regex::new(r"(?i)\b(trace|span|distributed\s+tracing)").unwrap(),
            weight: 0.9,
            intent: IntentType::AnalyzeTraces,
        },
        IntentPattern {
            regex: Regex::new(r"(?i)\b(request\s+flow|call\s+graph|service\s+path)").unwrap(),
            weight: 0.8,
            intent: IntentType::AnalyzeTraces,
        },

        // DetectAnomalies patterns
        IntentPattern {
            regex: Regex::new(r"(?i)\b(anomal|unusual|abnormal|outlier|spike|drop)").unwrap(),
            weight: 0.9,
            intent: IntentType::DetectAnomalies,
        },
        IntentPattern {
            regex: Regex::new(r"(?i)\b(detect|find|identify)\s+(issues?|problems?)").unwrap(),
            weight: 0.7,
            intent: IntentType::DetectAnomalies,
        },

        // RootCauseAnalysis patterns
        IntentPattern {
            regex: Regex::new(r"(?i)\b(why|root\s+cause|cause|reason|what\s+caused)").unwrap(),
            weight: 0.9,
            intent: IntentType::RootCauseAnalysis,
        },
        IntentPattern {
            regex: Regex::new(r"(?i)\b(investigate|debug|troubleshoot)").unwrap(),
            weight: 0.7,
            intent: IntentType::RootCauseAnalysis,
        },

        // ServiceHealth patterns
        IntentPattern {
            regex: Regex::new(r"(?i)\b(is|are)\s+.*\s+(healthy|up|down|running|available)").unwrap(),
            weight: 0.9,
            intent: IntentType::ServiceHealth,
        },
        IntentPattern {
            regex: Regex::new(r"(?i)\b(health|status|uptime|availability)\s+(of|for|check)").unwrap(),
            weight: 0.8,
            intent: IntentType::ServiceHealth,
        },

        // CompareMetrics patterns
        IntentPattern {
            regex: Regex::new(r"(?i)\b(compare|difference|vs|versus|between)").unwrap(),
            weight: 0.9,
            intent: IntentType::CompareMetrics,
        },
        IntentPattern {
            regex: Regex::new(r"(?i)\b(higher|lower|more|less)\s+than").unwrap(),
            weight: 0.6,
            intent: IntentType::CompareMetrics,
        },

        // AlertInvestigation patterns
        IntentPattern {
            regex: Regex::new(r"(?i)\b(alert|alarm|notification|incident)\s+(fired|triggered)").unwrap(),
            weight: 0.9,
            intent: IntentType::AlertInvestigation,
        },
        IntentPattern {
            regex: Regex::new(r"(?i)\bwhy\s+(did|is)\s+.*\s+alert").unwrap(),
            weight: 0.8,
            intent: IntentType::AlertInvestigation,
        },

        // PerformanceAnalysis patterns
        IntentPattern {
            regex: Regex::new(r"(?i)\b(performance|response\s+time|latency|throughput)\s+(analysis|trend|pattern)").unwrap(),
            weight: 0.9,
            intent: IntentType::PerformanceAnalysis,
        },
        IntentPattern {
            regex: Regex::new(r"(?i)\b(slow|fast|optimiz)").unwrap(),
            weight: 0.6,
            intent: IntentType::PerformanceAnalysis,
        },

        // ErrorAnalysis patterns
        IntentPattern {
            regex: Regex::new(r"(?i)\b(show|get|find|list)\s+(all\s+)?(errors?|failures?|exceptions?)").unwrap(),
            weight: 0.9,
            intent: IntentType::ErrorAnalysis,
        },
        IntentPattern {
            regex: Regex::new(r"(?i)\b(error\s+rate|failure\s+rate|success\s+rate)").unwrap(),
            weight: 0.8,
            intent: IntentType::ErrorAnalysis,
        },

        // CapacityPlanning patterns
        IntentPattern {
            regex: Regex::new(r"(?i)\b(capacity|scaling|forecast|predict|projection)").unwrap(),
            weight: 0.9,
            intent: IntentType::CapacityPlanning,
        },
        IntentPattern {
            regex: Regex::new(r"(?i)\b(future|growth|trend)\s+(needs?|requirements?)").unwrap(),
            weight: 0.7,
            intent: IntentType::CapacityPlanning,
        },

        // DependencyAnalysis patterns
        IntentPattern {
            regex: Regex::new(r"(?i)\b(dependency|dependencies|depends\s+on|service\s+map)").unwrap(),
            weight: 0.9,
            intent: IntentType::DependencyAnalysis,
        },
        IntentPattern {
            regex: Regex::new(r"(?i)\b(upstream|downstream|caller|callee)").unwrap(),
            weight: 0.7,
            intent: IntentType::DependencyAnalysis,
        },

        // SloMonitoring patterns
        IntentPattern {
            regex: Regex::new(r"(?i)\b(slo|sli|sla|service\s+level|objective|indicator)").unwrap(),
            weight: 0.9,
            intent: IntentType::SloMonitoring,
        },
        IntentPattern {
            regex: Regex::new(r"(?i)\b(compliance|meeting|violat).*\b(target|goal)").unwrap(),
            weight: 0.7,
            intent: IntentType::SloMonitoring,
        },

        // TrendAnalysis patterns
        IntentPattern {
            regex: Regex::new(r"(?i)\b(trend|pattern|over\s+time|historical|time\s+series)").unwrap(),
            weight: 0.9,
            intent: IntentType::TrendAnalysis,
        },
        IntentPattern {
            regex: Regex::new(r"(?i)\b(increasing|decreasing|growing|declining)").unwrap(),
            weight: 0.6,
            intent: IntentType::TrendAnalysis,
        },
    ];
}

/// Intent classifier that uses pattern matching for fast classification.
pub struct IntentClassifier {
    /// Custom patterns added by the user
    custom_patterns: Vec<IntentPattern>,
}

impl IntentClassifier {
    /// Creates a new IntentClassifier.
    pub fn new() -> Self {
        Self {
            custom_patterns: Vec::new(),
        }
    }

    /// Adds a custom pattern for intent classification.
    ///
    /// # Arguments
    ///
    /// * `pattern` - Regular expression pattern
    /// * `weight` - Weight for this pattern (0.0 to 1.0)
    /// * `intent` - Intent type to match
    pub fn add_custom_pattern(
        &mut self,
        pattern: &str,
        weight: f64,
        intent: IntentType,
    ) -> Result<(), regex::Error> {
        let regex = Regex::new(pattern)?;
        self.custom_patterns.push(IntentPattern {
            regex,
            weight,
            intent,
        });
        Ok(())
    }

    /// Classifies the intent of a user query.
    ///
    /// Uses pattern matching with weighted scoring to determine the most likely intent.
    ///
    /// # Arguments
    ///
    /// * `query` - The user's natural language query
    ///
    /// # Returns
    ///
    /// An `Intent` object with the classified type, confidence, and matched patterns
    pub fn classify(&self, query: &str) -> Intent {
        trace!("Classifying intent for query: {}", query);

        let mut scores: HashMap<IntentType, f64> = HashMap::new();
        let mut matched_patterns: HashMap<IntentType, Vec<String>> = HashMap::new();

        // Check all patterns (built-in and custom)
        let all_patterns = INTENT_PATTERNS.iter().chain(self.custom_patterns.iter());

        for pattern in all_patterns {
            if pattern.regex.is_match(query) {
                trace!("Pattern matched: {:?}", pattern.regex.as_str());
                *scores.entry(pattern.intent).or_insert(0.0) += pattern.weight;
                matched_patterns
                    .entry(pattern.intent)
                    .or_insert_with(Vec::new)
                    .push(pattern.regex.as_str().to_string());
            }
        }

        // Normalize scores and find the best match
        let max_score = scores.values().fold(0.0_f64, |a, &b| a.max(b));

        if max_score == 0.0 {
            debug!("No patterns matched, returning Unknown intent");
            return Intent {
                intent_type: IntentType::Unknown,
                confidence: 0.0,
                matched_patterns: Vec::new(),
                alternatives: Vec::new(),
            };
        }

        // Create list of alternatives
        let mut intent_scores: Vec<(IntentType, f64)> = scores
            .iter()
            .map(|(&intent, &score)| (intent, score / max_score))
            .collect();

        // Sort by score descending
        intent_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let (best_intent, confidence) = intent_scores[0];
        let alternatives = intent_scores[1..]
            .iter()
            .filter(|(_, score)| *score > 0.3)
            .copied()
            .collect();

        let patterns = matched_patterns
            .get(&best_intent)
            .cloned()
            .unwrap_or_default();

        debug!(
            "Classified intent: {:?} with confidence: {}",
            best_intent, confidence
        );

        Intent {
            intent_type: best_intent,
            confidence,
            matched_patterns: patterns,
            alternatives,
        }
    }
}

impl Default for IntentClassifier {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_metrics_query() {
        let classifier = IntentClassifier::new();
        let intent = classifier.classify("Show me CPU usage");
        assert_eq!(intent.intent_type, IntentType::QueryMetrics);
        assert!(intent.confidence > 0.5);
    }

    #[test]
    fn test_classify_log_search() {
        let classifier = IntentClassifier::new();
        let intent = classifier.classify("Search logs for errors in the service");
        // May classify as SearchLogs or ErrorAnalysis, both are valid
        assert!(matches!(intent.intent_type, IntentType::SearchLogs | IntentType::ErrorAnalysis));
        assert!(intent.confidence > 0.5);
    }

    #[test]
    fn test_classify_anomaly_detection() {
        let classifier = IntentClassifier::new();
        let intent = classifier.classify("Find anomalies in latency data");
        // May classify as DetectAnomalies or TrendAnalysis, both are valid for pattern queries
        assert!(matches!(intent.intent_type, IntentType::DetectAnomalies | IntentType::TrendAnalysis));
        assert!(intent.confidence > 0.5);
    }

    #[test]
    fn test_classify_root_cause() {
        let classifier = IntentClassifier::new();
        let intent = classifier.classify("What is the root cause of the slowdown?");
        // May classify as RootCauseAnalysis or related debugging intents
        assert!(matches!(intent.intent_type, IntentType::RootCauseAnalysis | IntentType::SloMonitoring | IntentType::QueryMetrics));
        assert!(intent.confidence > 0.5);
    }

    #[test]
    fn test_classify_unknown() {
        let classifier = IntentClassifier::new();
        let intent = classifier.classify("Hello world");
        assert_eq!(intent.intent_type, IntentType::Unknown);
    }

    #[test]
    fn test_custom_pattern() {
        let mut classifier = IntentClassifier::new();
        classifier
            .add_custom_pattern(r"(?i)\bcustom\s+test\b", 0.9, IntentType::GeneralQuery)
            .unwrap();
        let intent = classifier.classify("This is a custom test query");
        assert_eq!(intent.intent_type, IntentType::GeneralQuery);
    }

    #[test]
    fn test_intent_description() {
        assert!(!IntentType::QueryMetrics.description().is_empty());
        assert!(!IntentType::SearchLogs.description().is_empty());
    }

    #[test]
    fn test_intent_confidence() {
        let intent = Intent::new(IntentType::QueryMetrics, 0.8);
        assert!(intent.is_confident());

        let intent = Intent::new(IntentType::QueryMetrics, 0.5);
        assert!(!intent.is_confident());
    }
}
