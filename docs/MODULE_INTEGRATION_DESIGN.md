# Module Integration Interfaces Design
## LLM-CoPilot-Agent Integration Architecture

---

## Table of Contents
1. [Test-Bench Integration](#1-test-bench-integration)
2. [Observatory Integration](#2-observatory-integration)
3. [Incident-Manager Integration](#3-incident-manager-integration)
4. [Orchestrator Integration](#4-orchestrator-integration)
5. [Module Registry and Discovery](#5-module-registry-and-discovery)
6. [Common Infrastructure](#6-common-infrastructure)

---

## 1. Test-Bench Integration

### 1.1 Core Trait Definition

```rust
// Core trait for LLM-Test-Bench integration
trait TestBenchIntegration {
    async fn generate_tests(&self, spec: TestSpec) -> Result<TestSuite>;
    async fn execute_suite(&self, suite: TestSuite) -> Result<TestResults>;
    async fn get_coverage(&self, project: ProjectId) -> Result<CoverageReport>;
}

// Data structures
struct TestSpec {
    description: String,              // Natural language description
    target_files: Vec<FilePath>,      // Files to test
    test_types: Vec<TestType>,        // Unit, Integration, E2E, etc.
    constraints: TestConstraints,      // Time, resource constraints
    context: HashMap<String, Value>,   // Additional context
}

struct TestSuite {
    id: TestSuiteId,
    name: String,
    tests: Vec<Test>,
    setup: Option<SetupScript>,
    teardown: Option<TeardownScript>,
    metadata: TestMetadata,
}

struct TestResults {
    suite_id: TestSuiteId,
    passed: usize,
    failed: usize,
    skipped: usize,
    total_duration: Duration,
    test_outputs: Vec<TestOutput>,
    coverage_delta: Option<CoverageDelta>,
}

struct CoverageReport {
    project_id: ProjectId,
    overall_coverage: f64,
    line_coverage: f64,
    branch_coverage: f64,
    function_coverage: f64,
    file_reports: Vec<FileCoverage>,
    uncovered_areas: Vec<UncoveredArea>,
}
```

### 1.2 Implementation with Circuit Breaker

```rust
struct TestBenchClient {
    grpc_client: TestBenchGrpcClient,
    circuit_breaker: CircuitBreaker,
    retry_policy: RetryPolicy,
    metrics_collector: MetricsCollector,
    cache: TestCache,
}

impl TestBenchClient {
    // Initialize with health check
    async fn new(config: TestBenchConfig) -> Result<Self> {
        BEGIN
            // Establish gRPC connection
            grpc_client = connect_grpc(config.endpoint, config.tls_config)

            // Initialize circuit breaker with thresholds
            circuit_breaker = CircuitBreaker::new(
                failure_threshold: 5,
                timeout: Duration::seconds(30),
                reset_timeout: Duration::minutes(1)
            )

            // Configure retry policy
            retry_policy = RetryPolicy::exponential(
                initial_delay: Duration::milliseconds(100),
                max_delay: Duration::seconds(10),
                max_attempts: 3
            )

            // Verify connection health
            health_status = check_health(grpc_client)
            IF NOT health_status.is_healthy THEN
                RETURN Error("TestBench service unhealthy")
            END IF

            RETURN TestBenchClient {
                grpc_client,
                circuit_breaker,
                retry_policy,
                metrics_collector: MetricsCollector::new("test_bench"),
                cache: TestCache::new(ttl: Duration::minutes(5))
            }
        END
    }
}
```

### 1.3 Test Specification Builder from Natural Language

```rust
impl TestBenchClient {
    async fn generate_tests(&self, spec: TestSpec) -> Result<TestSuite> {
        BEGIN
            start_time = now()

            // Check circuit breaker state
            IF circuit_breaker.is_open() THEN
                RETURN Error("Circuit breaker open - TestBench unavailable")
            END IF

            // Parse natural language to structured spec
            structured_spec = WITH_RETRY(retry_policy) DO
                parse_result = self.parse_natural_language_spec(spec.description)

                // Enhance with static analysis
                analyzed_files = self.analyze_target_files(spec.target_files)

                // Merge specifications
                merged_spec = TestSpecBuilder::new()
                    .with_parsed_requirements(parse_result.requirements)
                    .with_test_types(spec.test_types)
                    .with_file_analysis(analyzed_files)
                    .with_constraints(spec.constraints)
                    .build()

                RETURN merged_spec
            END WITH_RETRY

            // Generate test suite via gRPC
            test_suite = WITH_CIRCUIT_BREAKER(circuit_breaker) DO
                request = GenerateTestsRequest {
                    spec: structured_spec,
                    llm_config: self.get_llm_config(),
                    generation_mode: "intelligent" // or "exhaustive"
                }

                // Stream generation progress
                response_stream = grpc_client.generate_tests_streaming(request)

                accumulated_tests = []
                FOR EACH chunk IN response_stream DO
                    MATCH chunk {
                        Progress(info) => {
                            emit_progress_event(info)
                        }
                        TestGenerated(test) => {
                            accumulated_tests.push(test)
                            cache.store_test(test.id, test)
                        }
                        Error(err) => {
                            circuit_breaker.record_failure()
                            RETURN Error(err)
                        }
                        Complete(suite_metadata) => {
                            BREAK
                        }
                    }
                END FOR

                final_suite = TestSuite {
                    id: generate_uuid(),
                    name: structured_spec.name,
                    tests: accumulated_tests,
                    setup: structured_spec.setup,
                    teardown: structured_spec.teardown,
                    metadata: suite_metadata
                }

                RETURN final_suite
            END WITH_CIRCUIT_BREAKER

            // Record success metrics
            circuit_breaker.record_success()
            metrics_collector.record_latency("generate_tests", now() - start_time)

            RETURN test_suite

        CATCH error AS e THEN
            circuit_breaker.record_failure()
            metrics_collector.increment_error("generate_tests")
            RETURN Error(e)
        END
    }

    // Helper: Parse natural language to structured requirements
    fn parse_natural_language_spec(&self, description: String) -> Result<ParsedSpec> {
        BEGIN
            // Use LLM to extract structured information
            prompt = format!("""
                Parse the following test description into structured requirements:

                Description: {description}

                Extract:
                1. Test scenarios (what to test)
                2. Expected behaviors (assertions)
                3. Input conditions (test data)
                4. Edge cases to cover
                5. Performance requirements

                Return as JSON.
            """)

            llm_response = call_llm(prompt, temperature: 0.2)
            parsed = json_parse(llm_response)

            validated_spec = ParsedSpec::validate_and_build(parsed)
            RETURN validated_spec
        END
    }
}
```

### 1.4 Test Execution Orchestration with Streaming Progress

```rust
impl TestBenchClient {
    async fn execute_suite(&self, suite: TestSuite) -> Result<TestResults> {
        BEGIN
            execution_id = generate_uuid()
            start_time = now()

            // Validate suite before execution
            validation_result = self.validate_suite(suite)
            IF NOT validation_result.is_valid THEN
                RETURN Error("Invalid test suite: " + validation_result.errors)
            END IF

            // Check resource availability
            resources = WITH_RETRY(retry_policy) DO
                self.check_execution_resources(suite.resource_requirements)
            END WITH_RETRY

            IF NOT resources.available THEN
                RETURN Error("Insufficient resources: " + resources.missing)
            END IF

            // Execute with streaming progress
            results = WITH_CIRCUIT_BREAKER(circuit_breaker) DO
                request = ExecuteSuiteRequest {
                    suite_id: suite.id,
                    execution_id: execution_id,
                    parallel_mode: determine_parallel_mode(suite),
                    isolation_level: "process", // or "container", "vm"
                    timeout: suite.constraints.max_duration
                }

                // Stream execution results
                execution_stream = grpc_client.execute_suite_streaming(request)

                test_outputs = []
                stats = ExecutionStats::new()

                FOR EACH event IN execution_stream DO
                    MATCH event {
                        Started(test_id) => {
                            emit_event("test_started", test_id)
                            stats.mark_started(test_id)
                        }
                        Progress(test_id, progress) => {
                            emit_event("test_progress", {test_id, progress})
                            update_dashboard(test_id, progress)
                        }
                        Completed(test_id, output) => {
                            test_outputs.push(output)
                            stats.mark_completed(test_id, output.status)
                            emit_event("test_completed", {test_id, output})

                            // Real-time analysis
                            IF output.status == "failed" THEN
                                failure_analysis = self.analyze_failure(output)
                                emit_event("failure_analyzed", failure_analysis)
                            END IF
                        }
                        ResourceWarning(warning) => {
                            emit_event("resource_warning", warning)
                            IF warning.severity == "critical" THEN
                                consider_throttling()
                            END IF
                        }
                        Error(test_id, error) => {
                            stats.mark_error(test_id, error)
                            emit_event("test_error", {test_id, error})
                        }
                        SuiteComplete(summary) => {
                            stats.finalize(summary)
                            BREAK
                        }
                    }
                END FOR

                // Aggregate results
                final_results = TestResults {
                    suite_id: suite.id,
                    execution_id: execution_id,
                    passed: stats.passed_count,
                    failed: stats.failed_count,
                    skipped: stats.skipped_count,
                    total_duration: now() - start_time,
                    test_outputs: test_outputs,
                    coverage_delta: compute_coverage_delta(suite.id),
                    resource_usage: stats.resource_usage
                }

                RETURN final_results
            END WITH_CIRCUIT_BREAKER

            // Post-execution analysis
            recommendations = self.generate_recommendations(results)
            results.recommendations = recommendations

            // Cache results
            cache.store_results(execution_id, results, ttl: Duration::hours(24))

            // Record metrics
            circuit_breaker.record_success()
            metrics_collector.record_test_execution(results)

            RETURN results

        CATCH error AS e THEN
            circuit_breaker.record_failure()
            metrics_collector.increment_error("execute_suite")

            // Attempt cleanup
            self.cleanup_failed_execution(execution_id)

            RETURN Error(e)
        END
    }

    // Helper: Determine optimal parallelization strategy
    fn determine_parallel_mode(&self, suite: TestSuite) -> ParallelMode {
        BEGIN
            // Analyze test dependencies
            dependency_graph = build_test_dependency_graph(suite.tests)

            IF dependency_graph.is_fully_independent() THEN
                max_parallel = min(
                    suite.tests.len(),
                    available_cpu_cores(),
                    suite.constraints.max_parallel_tests
                )
                RETURN ParallelMode::Full(max_parallel)
            ELSE IF dependency_graph.has_layers() THEN
                layers = dependency_graph.topological_layers()
                RETURN ParallelMode::Layered(layers)
            ELSE
                RETURN ParallelMode::Sequential
            END IF
        END
    }
}
```

### 1.5 Coverage Analysis and Gap Identification

```rust
impl TestBenchClient {
    async fn get_coverage(&self, project: ProjectId) -> Result<CoverageReport> {
        BEGIN
            // Check cache first
            IF cache.has_coverage(project) THEN
                cached = cache.get_coverage(project)
                IF cached.age < Duration::minutes(5) THEN
                    RETURN cached.report
                END IF
            END IF

            // Fetch fresh coverage data
            coverage = WITH_CIRCUIT_BREAKER(circuit_breaker) DO
                request = GetCoverageRequest {
                    project_id: project,
                    include_historical: true,
                    granularity: "line" // or "branch", "function"
                }

                response = grpc_client.get_coverage(request)
                RETURN response.report
            END WITH_CIRCUIT_BREAKER

            // Analyze coverage gaps
            gaps = self.identify_coverage_gaps(coverage)

            // Generate improvement suggestions
            suggestions = self.generate_coverage_suggestions(gaps)

            enhanced_report = CoverageReport {
                ...coverage,
                uncovered_areas: gaps,
                improvement_suggestions: suggestions,
                trend_analysis: self.analyze_coverage_trends(project)
            }

            // Cache enhanced report
            cache.store_coverage(project, enhanced_report)

            RETURN enhanced_report
        END
    }

    // Helper: Identify critical coverage gaps
    fn identify_coverage_gaps(&self, coverage: RawCoverage) -> Vec<UncoveredArea> {
        BEGIN
            gaps = []

            FOR EACH file IN coverage.files DO
                // Identify uncovered critical paths
                critical_paths = analyze_critical_code_paths(file)

                FOR EACH path IN critical_paths DO
                    IF NOT path.is_covered THEN
                        severity = assess_gap_severity(path)

                        gap = UncoveredArea {
                            file: file.path,
                            lines: path.line_range,
                            code_type: path.code_type, // error_handling, business_logic, etc.
                            severity: severity,
                            impact_score: calculate_impact_score(path),
                            suggested_tests: generate_test_suggestions(path)
                        }

                        gaps.push(gap)
                    END IF
                END FOR
            END FOR

            // Sort by priority
            gaps.sort_by(|a, b| b.impact_score.cmp(a.impact_score))

            RETURN gaps
        END
    }

    // Helper: Generate actionable coverage improvement suggestions
    fn generate_coverage_suggestions(&self, gaps: Vec<UncoveredArea>) -> Vec<Suggestion> {
        BEGIN
            suggestions = []

            // Group gaps by category
            grouped = group_by(gaps, |gap| gap.code_type)

            FOR EACH (code_type, type_gaps) IN grouped DO
                // Generate type-specific suggestions
                MATCH code_type {
                    ErrorHandling => {
                        suggestion = Suggestion {
                            title: "Add error handling tests",
                            description: format_error_handling_suggestion(type_gaps),
                            priority: "high",
                            estimated_effort: estimate_effort(type_gaps),
                            test_templates: generate_error_test_templates(type_gaps)
                        }
                    }
                    BusinessLogic => {
                        suggestion = Suggestion {
                            title: "Cover business logic paths",
                            description: format_business_logic_suggestion(type_gaps),
                            priority: "critical",
                            estimated_effort: estimate_effort(type_gaps),
                            test_templates: generate_business_logic_templates(type_gaps)
                        }
                    }
                    EdgeCases => {
                        suggestion = Suggestion {
                            title: "Test edge cases",
                            description: format_edge_case_suggestion(type_gaps),
                            priority: "medium",
                            estimated_effort: estimate_effort(type_gaps),
                            test_templates: generate_edge_case_templates(type_gaps)
                        }
                    }
                }

                suggestions.push(suggestion)
            END FOR

            RETURN suggestions
        END
    }
}
```

---

## 2. Observatory Integration

### 2.1 Core Trait Definition

```rust
// Core trait for LLM-Observatory integration
trait ObservatoryIntegration {
    async fn query_metrics(&self, promql: &str, range: TimeRange) -> Result<MetricData>;
    async fn search_logs(&self, logql: &str, range: TimeRange) -> Result<LogData>;
    async fn query_traces(&self, traceql: &str, range: TimeRange) -> Result<TraceData>;
    async fn detect_anomalies(&self, config: AnomalyConfig) -> Result<Vec<Anomaly>>;
}

// Data structures
struct MetricData {
    query: String,
    range: TimeRange,
    result_type: ResultType, // vector, matrix, scalar
    results: Vec<MetricSeries>,
    metadata: QueryMetadata,
}

struct LogData {
    query: String,
    range: TimeRange,
    total_entries: usize,
    entries: Vec<LogEntry>,
    patterns: Vec<LogPattern>,
    metadata: QueryMetadata,
}

struct TraceData {
    query: String,
    range: TimeRange,
    traces: Vec<Trace>,
    service_graph: ServiceGraph,
    critical_path: Vec<Span>,
    metadata: QueryMetadata,
}

struct Anomaly {
    id: AnomalyId,
    detected_at: Timestamp,
    anomaly_type: AnomalyType,
    severity: Severity,
    affected_metrics: Vec<MetricRef>,
    context: AnomalyContext,
    explanation: String,
    suggested_actions: Vec<Action>,
}
```

### 2.2 Implementation with Connection Pooling

```rust
struct ObservatoryClient {
    rest_client: RestClient,
    otlp_client: OtlpClient,
    circuit_breaker: CircuitBreaker,
    retry_policy: RetryPolicy,
    query_cache: QueryCache,
    connection_pool: ConnectionPool,
}

impl ObservatoryClient {
    async fn new(config: ObservatoryConfig) -> Result<Self> {
        BEGIN
            // Create connection pool for REST API
            rest_client = RestClient::builder()
                .base_url(config.api_endpoint)
                .timeout(Duration::seconds(30))
                .connection_pool_size(config.pool_size)
                .tls_config(config.tls)
                .build()

            // Create OTLP client for telemetry
            otlp_client = OtlpClient::new(
                config.otlp_endpoint,
                config.otlp_protocol // grpc or http
            )

            // Initialize circuit breaker
            circuit_breaker = CircuitBreaker::new(
                failure_threshold: 5,
                timeout: Duration::seconds(30),
                reset_timeout: Duration::minutes(1)
            )

            // Configure retry with jitter
            retry_policy = RetryPolicy::exponential_with_jitter(
                initial_delay: Duration::milliseconds(100),
                max_delay: Duration::seconds(10),
                max_attempts: 3,
                jitter: 0.1
            )

            // Create query cache with LRU eviction
            query_cache = QueryCache::new(
                max_size: 1000,
                ttl: Duration::minutes(5),
                eviction_policy: "lru"
            )

            // Verify connectivity
            health = check_observatory_health(rest_client)
            IF NOT health.is_healthy THEN
                RETURN Error("Observatory service unhealthy")
            END IF

            RETURN ObservatoryClient {
                rest_client,
                otlp_client,
                circuit_breaker,
                retry_policy,
                query_cache,
                connection_pool: rest_client.pool
            }
        END
    }
}
```

### 2.3 Metric Aggregation and Analysis

```rust
impl ObservatoryClient {
    async fn query_metrics(&self, promql: &str, range: TimeRange) -> Result<MetricData> {
        BEGIN
            // Validate PromQL query
            validation = validate_promql(promql)
            IF NOT validation.is_valid THEN
                RETURN Error("Invalid PromQL: " + validation.errors)
            END IF

            // Check cache
            cache_key = hash(promql, range)
            IF query_cache.contains(cache_key) THEN
                cached_data = query_cache.get(cache_key)
                IF cached_data.is_fresh() THEN
                    RETURN cached_data.value
                END IF
            END IF

            // Execute query with circuit breaker
            metric_data = WITH_CIRCUIT_BREAKER(circuit_breaker) DO
                WITH_RETRY(retry_policy) DO
                    request = QueryRangeRequest {
                        query: promql,
                        start: range.start,
                        end: range.end,
                        step: determine_optimal_step(range),
                        timeout: Duration::seconds(30)
                    }

                    response = rest_client.post("/api/v1/query_range", request)

                    IF response.status != 200 THEN
                        circuit_breaker.record_failure()
                        RETURN Error("Query failed: " + response.error)
                    END IF

                    raw_data = response.json()
                    RETURN raw_data
                END WITH_RETRY
            END WITH_CIRCUIT_BREAKER

            // Parse and enrich results
            parsed_data = parse_prometheus_response(metric_data)

            // Perform statistical analysis
            analyzed_data = MetricData {
                query: promql,
                range: range,
                result_type: parsed_data.result_type,
                results: parsed_data.results,
                metadata: QueryMetadata {
                    execution_time: parsed_data.execution_time,
                    series_count: parsed_data.results.len(),
                    samples_count: count_total_samples(parsed_data.results)
                }
            }

            // Add statistical insights
            FOR EACH series IN analyzed_data.results DO
                series.statistics = calculate_statistics(series.values)
                series.trend = detect_trend(series.values)
                series.anomalies = detect_metric_anomalies(series.values)
            END FOR

            // Cache enriched data
            query_cache.put(cache_key, analyzed_data, ttl: Duration::minutes(5))

            circuit_breaker.record_success()

            RETURN analyzed_data

        CATCH error AS e THEN
            circuit_breaker.record_failure()
            RETURN Error(e)
        END
    }

    // Helper: Calculate comprehensive statistics
    fn calculate_statistics(&self, values: Vec<Sample>) -> Statistics {
        BEGIN
            IF values.is_empty() THEN
                RETURN Statistics::empty()
            END IF

            sorted = sort(values.map(|s| s.value))

            stats = Statistics {
                count: values.len(),
                min: sorted.first(),
                max: sorted.last(),
                mean: sum(sorted) / sorted.len(),
                median: percentile(sorted, 0.5),
                p95: percentile(sorted, 0.95),
                p99: percentile(sorted, 0.99),
                stddev: calculate_stddev(sorted),
                variance: calculate_variance(sorted)
            }

            RETURN stats
        END
    }

    // Helper: Detect metric trends
    fn detect_trend(&self, values: Vec<Sample>) -> Trend {
        BEGIN
            IF values.len() < 2 THEN
                RETURN Trend::Insufficient
            END IF

            // Simple linear regression
            n = values.len()
            sum_x = sum(0..n)
            sum_y = sum(values.map(|v| v.value))
            sum_xy = sum(values.enumerate().map(|(i, v)| i * v.value))
            sum_x2 = sum((0..n).map(|i| i * i))

            slope = (n * sum_xy - sum_x * sum_y) / (n * sum_x2 - sum_x * sum_x)

            // Classify trend
            IF abs(slope) < 0.01 THEN
                RETURN Trend::Stable
            ELSE IF slope > 0 THEN
                rate = calculate_growth_rate(slope, values)
                RETURN Trend::Increasing(rate)
            ELSE
                rate = calculate_decline_rate(slope, values)
                RETURN Trend::Decreasing(rate)
            END IF
        END
    }
}
```

### 2.4 Log Pattern Detection

```rust
impl ObservatoryClient {
    async fn search_logs(&self, logql: &str, range: TimeRange) -> Result<LogData> {
        BEGIN
            // Validate LogQL
            validation = validate_logql(logql)
            IF NOT validation.is_valid THEN
                RETURN Error("Invalid LogQL: " + validation.errors)
            END IF

            // Execute query with streaming for large results
            log_data = WITH_CIRCUIT_BREAKER(circuit_breaker) DO
                WITH_RETRY(retry_policy) DO
                    request = LogQueryRequest {
                        query: logql,
                        start: range.start,
                        end: range.end,
                        limit: 5000,
                        direction: "backward" // newest first
                    }

                    // Stream log entries
                    stream = rest_client.post_streaming("/loki/api/v1/query_range", request)

                    entries = []
                    FOR EACH chunk IN stream DO
                        parsed = parse_log_chunk(chunk)
                        entries.extend(parsed.entries)

                        // Check size limits
                        IF entries.len() >= request.limit THEN
                            BREAK
                        END IF
                    END FOR

                    RETURN entries
                END WITH_RETRY
            END WITH_CIRCUIT_BREAKER

            // Detect patterns in logs
            patterns = self.detect_log_patterns(log_data)

            // Classify log entries
            classified = self.classify_log_entries(log_data)

            result = LogData {
                query: logql,
                range: range,
                total_entries: log_data.len(),
                entries: classified,
                patterns: patterns,
                metadata: QueryMetadata {
                    execution_time: measure_execution_time(),
                    data_size: calculate_data_size(log_data)
                }
            }

            circuit_breaker.record_success()

            RETURN result

        CATCH error AS e THEN
            circuit_breaker.record_failure()
            RETURN Error(e)
        END
    }

    // Helper: Detect common patterns in logs
    fn detect_log_patterns(&self, entries: Vec<LogEntry>) -> Vec<LogPattern> {
        BEGIN
            patterns = []

            // Group by message template
            message_groups = group_logs_by_template(entries)

            FOR EACH (template, group_entries) IN message_groups DO
                // Calculate frequency
                frequency = group_entries.len() / entries.len()

                // Detect temporal patterns
                temporal = analyze_temporal_pattern(group_entries)

                // Extract key variables
                variables = extract_pattern_variables(template, group_entries)

                pattern = LogPattern {
                    template: template,
                    frequency: frequency,
                    occurrences: group_entries.len(),
                    temporal_pattern: temporal,
                    severity_distribution: group_by_severity(group_entries),
                    common_variables: variables,
                    example_entries: sample(group_entries, 5)
                }

                patterns.push(pattern)
            END FOR

            // Sort by significance
            patterns.sort_by(|a, b| b.occurrences.cmp(a.occurrences))

            RETURN patterns
        END
    }

    // Helper: Group logs by message template
    fn group_logs_by_template(&self, entries: Vec<LogEntry>) -> HashMap<String, Vec<LogEntry>> {
        BEGIN
            groups = HashMap::new()

            FOR EACH entry IN entries DO
                // Extract template by replacing variables with placeholders
                template = templatize_message(entry.message)

                IF NOT groups.contains_key(template) THEN
                    groups.insert(template, [])
                END IF

                groups.get_mut(template).push(entry)
            END FOR

            RETURN groups
        END
    }

    // Helper: Templatize log message
    fn templatize_message(&self, message: String) -> String {
        BEGIN
            template = message

            // Replace common variable patterns
            patterns = [
                (r"\d+", "<NUMBER>"),
                (r"[a-f0-9-]{36}", "<UUID>"),
                (r"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}", "<TIMESTAMP>"),
                (r"/[a-zA-Z0-9/_-]+", "<PATH>"),
                (r"\b(?:\d{1,3}\.){3}\d{1,3}\b", "<IP>")
            ]

            FOR EACH (pattern, replacement) IN patterns DO
                template = regex_replace_all(template, pattern, replacement)
            END FOR

            RETURN template
        END
    }
}
```

### 2.5 Trace Correlation Algorithm

```rust
impl ObservatoryClient {
    async fn query_traces(&self, traceql: &str, range: TimeRange) -> Result<TraceData> {
        BEGIN
            // Execute TraceQL query
            traces = WITH_CIRCUIT_BREAKER(circuit_breaker) DO
                WITH_RETRY(retry_policy) DO
                    request = TraceQueryRequest {
                        query: traceql,
                        start: range.start,
                        end: range.end,
                        limit: 1000
                    }

                    response = rest_client.post("/api/v1/traces", request)
                    RETURN parse_trace_response(response)
                END WITH_RETRY
            END WITH_CIRCUIT_BREAKER

            // Build service dependency graph
            service_graph = self.build_service_graph(traces)

            // Identify critical path for each trace
            FOR EACH trace IN traces DO
                trace.critical_path = self.compute_critical_path(trace)
                trace.bottlenecks = self.identify_bottlenecks(trace)
            END FOR

            // Correlate with metrics and logs
            correlated_data = self.correlate_telemetry(traces, range)

            result = TraceData {
                query: traceql,
                range: range,
                traces: traces,
                service_graph: service_graph,
                critical_path: aggregate_critical_paths(traces),
                metadata: QueryMetadata {
                    trace_count: traces.len(),
                    span_count: count_total_spans(traces),
                    services_involved: service_graph.services.len()
                }
            }

            circuit_breaker.record_success()

            RETURN result
        END
    }

    // Helper: Build service dependency graph from traces
    fn build_service_graph(&self, traces: Vec<Trace>) -> ServiceGraph {
        BEGIN
            graph = ServiceGraph::new()

            FOR EACH trace IN traces DO
                FOR EACH span IN trace.spans DO
                    service = span.service_name

                    // Add service node
                    IF NOT graph.has_service(service) THEN
                        graph.add_service(Service {
                            name: service,
                            span_count: 0,
                            total_duration: Duration::zero(),
                            error_rate: 0.0
                        })
                    END IF

                    // Update service metrics
                    graph.get_service_mut(service).span_count += 1
                    graph.get_service_mut(service).total_duration += span.duration
                    IF span.has_error THEN
                        graph.get_service_mut(service).error_count += 1
                    END IF

                    // Add service dependency edge
                    IF span.has_parent THEN
                        parent_span = trace.find_span(span.parent_id)
                        parent_service = parent_span.service_name

                        IF parent_service != service THEN
                            graph.add_edge(
                                from: parent_service,
                                to: service,
                                calls: 1,
                                avg_latency: span.duration
                            )
                        END IF
                    END IF
                END FOR
            END FOR

            // Calculate derived metrics
            FOR EACH service IN graph.services DO
                service.error_rate = service.error_count / service.span_count
                service.avg_duration = service.total_duration / service.span_count
            END FOR

            RETURN graph
        END
    }

    // Helper: Compute critical path (longest latency path)
    fn compute_critical_path(&self, trace: Trace) -> Vec<Span> {
        BEGIN
            // Build span tree
            root_spans = trace.spans.filter(|s| s.parent_id.is_none())

            IF root_spans.is_empty() THEN
                RETURN []
            END IF

            // DFS to find longest path
            longest_path = []
            max_duration = Duration::zero()

            FUNCTION dfs(span: Span, current_path: Vec<Span>, accumulated_duration: Duration) {
                current_path.push(span)
                accumulated_duration += span.duration

                children = trace.spans.filter(|s| s.parent_id == span.id)

                IF children.is_empty() THEN
                    // Leaf node - check if this is the longest path
                    IF accumulated_duration > max_duration THEN
                        max_duration = accumulated_duration
                        longest_path = current_path.clone()
                    END IF
                ELSE
                    FOR EACH child IN children DO
                        dfs(child, current_path.clone(), accumulated_duration)
                    END FOR
                END IF
            }

            FOR EACH root IN root_spans DO
                dfs(root, [], Duration::zero())
            END FOR

            RETURN longest_path
        END
    }

    // Helper: Identify performance bottlenecks
    fn identify_bottlenecks(&self, trace: Trace) -> Vec<Bottleneck> {
        BEGIN
            bottlenecks = []

            total_duration = trace.duration

            FOR EACH span IN trace.spans DO
                // Calculate exclusive time (time not spent in children)
                children = trace.spans.filter(|s| s.parent_id == span.id)
                children_duration = sum(children.map(|c| c.duration))
                exclusive_time = span.duration - children_duration

                // Calculate percentage of total trace time
                percentage = (exclusive_time / total_duration) * 100

                // Flag as bottleneck if > 20% of total time
                IF percentage > 20.0 THEN
                    bottleneck = Bottleneck {
                        span: span,
                        exclusive_time: exclusive_time,
                        percentage: percentage,
                        severity: classify_severity(percentage),
                        recommendations: generate_optimization_suggestions(span)
                    }

                    bottlenecks.push(bottleneck)
                END IF
            END FOR

            // Sort by severity
            bottlenecks.sort_by(|a, b| b.percentage.cmp(a.percentage))

            RETURN bottlenecks
        END
    }
}
```

### 2.6 Anomaly Detection with Contextual Explanation

```rust
impl ObservatoryClient {
    async fn detect_anomalies(&self, config: AnomalyConfig) -> Result<Vec<Anomaly>> {
        BEGIN
            anomalies = []

            // Fetch baseline metrics
            baseline_range = TimeRange {
                start: config.range.start - config.baseline_period,
                end: config.range.start
            }

            baseline_data = self.query_metrics(
                config.metric_query,
                baseline_range
            ).await?

            // Fetch current metrics
            current_data = self.query_metrics(
                config.metric_query,
                config.range
            ).await?

            // Apply anomaly detection algorithms
            FOR EACH series IN current_data.results DO
                baseline_series = find_matching_baseline(series, baseline_data)

                IF baseline_series.is_none() THEN
                    CONTINUE
                END IF

                // Statistical anomaly detection
                detected = MATCH config.detection_method {
                    StatisticalThreshold => {
                        self.detect_statistical_anomalies(
                            series,
                            baseline_series,
                            config.sensitivity
                        )
                    }
                    MachineLearning => {
                        self.detect_ml_anomalies(
                            series,
                            baseline_series,
                            config.ml_model
                        )
                    }
                    Hybrid => {
                        statistical = self.detect_statistical_anomalies(...)
                        ml = self.detect_ml_anomalies(...)
                        intersect(statistical, ml) // High confidence
                    }
                }

                // Enrich with context
                FOR EACH anomaly_point IN detected DO
                    context = self.gather_anomaly_context(anomaly_point, config.range)
                    explanation = self.generate_explanation(anomaly_point, context)
                    actions = self.suggest_actions(anomaly_point, context)

                    anomaly = Anomaly {
                        id: generate_uuid(),
                        detected_at: anomaly_point.timestamp,
                        anomaly_type: classify_anomaly_type(anomaly_point),
                        severity: calculate_severity(anomaly_point, baseline_series),
                        affected_metrics: vec![series.metric_ref],
                        context: context,
                        explanation: explanation,
                        suggested_actions: actions
                    }

                    anomalies.push(anomaly)
                END FOR
            END FOR

            // Correlate anomalies across metrics
            correlated = self.correlate_anomalies(anomalies)

            RETURN correlated
        END
    }

    // Helper: Statistical anomaly detection using Z-score
    fn detect_statistical_anomalies(
        &self,
        series: MetricSeries,
        baseline: MetricSeries,
        sensitivity: f64
    ) -> Vec<AnomalyPoint> {
        BEGIN
            anomalies = []

            // Calculate baseline statistics
            baseline_values = baseline.values.map(|v| v.value)
            mean = calculate_mean(baseline_values)
            stddev = calculate_stddev(baseline_values)

            // Z-score threshold based on sensitivity
            threshold = MATCH sensitivity {
                Low => 3.0,      // 99.7% confidence
                Medium => 2.5,   // 98.8% confidence
                High => 2.0      // 95.4% confidence
            }

            FOR EACH sample IN series.values DO
                z_score = abs((sample.value - mean) / stddev)

                IF z_score > threshold THEN
                    anomaly = AnomalyPoint {
                        timestamp: sample.timestamp,
                        value: sample.value,
                        expected_value: mean,
                        deviation: z_score,
                        confidence: calculate_confidence(z_score, threshold)
                    }

                    anomalies.push(anomaly)
                END IF
            END FOR

            RETURN anomalies
        END
    }

    // Helper: Gather contextual information about anomaly
    fn gather_anomaly_context(
        &self,
        anomaly_point: AnomalyPoint,
        range: TimeRange
    ) -> AnomalyContext {
        BEGIN
            context_window = TimeRange {
                start: anomaly_point.timestamp - Duration::minutes(5),
                end: anomaly_point.timestamp + Duration::minutes(5)
            }

            // Gather correlated logs
            logs = self.search_logs(
                format!("{{severity=~\"error|warn\"}}"),
                context_window
            ).await.unwrap_or_default()

            // Gather related traces
            traces = self.query_traces(
                format!("{{status=error}}"),
                context_window
            ).await.unwrap_or_default()

            // Check for deployments or changes
            deployments = query_deployment_events(context_window)

            // Check related metrics
            related_metrics = self.query_related_metrics(
                anomaly_point.metric,
                context_window
            ).await.unwrap_or_default()

            context = AnomalyContext {
                error_logs: logs.entries.filter(|e| e.severity == "error"),
                warning_logs: logs.entries.filter(|e| e.severity == "warn"),
                failed_traces: traces.traces.filter(|t| t.has_error),
                recent_deployments: deployments,
                related_metric_anomalies: find_related_anomalies(related_metrics),
                system_events: query_system_events(context_window)
            }

            RETURN context
        END
    }

    // Helper: Generate human-readable explanation
    fn generate_explanation(
        &self,
        anomaly: AnomalyPoint,
        context: AnomalyContext
    ) -> String {
        BEGIN
            // Use LLM to generate contextual explanation
            prompt = format!("""
                Analyze this performance anomaly and provide a clear explanation:

                Anomaly Details:
                - Metric: {anomaly.metric}
                - Timestamp: {anomaly.timestamp}
                - Value: {anomaly.value} (expected: {anomaly.expected_value})
                - Deviation: {anomaly.deviation}σ

                Context:
                - Error logs: {context.error_logs.len()} entries
                - Warning logs: {context.warning_logs.len()} entries
                - Failed traces: {context.failed_traces.len()}
                - Recent deployments: {context.recent_deployments}
                - Related anomalies: {context.related_metric_anomalies}

                Provide:
                1. Most likely root cause
                2. Contributing factors
                3. Impact assessment
                4. Confidence level

                Be concise and actionable.
            """)

            explanation = call_llm(prompt, temperature: 0.3)

            RETURN explanation
        END
    }
}
```

---

## 3. Incident-Manager Integration

### 3.1 Core Trait Definition

```rust
// Core trait for LLM-Incident-Manager integration
trait IncidentManagerIntegration {
    async fn create_incident(&self, details: IncidentDetails) -> Result<IncidentId>;
    async fn update_status(&self, id: IncidentId, status: Status) -> Result<()>;
    async fn execute_runbook(&self, runbook: RunbookId, params: Params) -> Result<ExecutionId>;
    async fn generate_postmortem(&self, id: IncidentId) -> Result<PostmortemReport>;
}

// Data structures
struct IncidentDetails {
    title: String,
    description: String,
    severity: Severity,
    affected_services: Vec<ServiceId>,
    detected_by: DetectionSource,
    evidence: Evidence,
    initial_impact: Impact,
}

struct Evidence {
    anomalies: Vec<Anomaly>,
    error_logs: Vec<LogEntry>,
    failed_traces: Vec<Trace>,
    metric_snapshots: Vec<MetricSnapshot>,
}

struct PostmortemReport {
    incident_id: IncidentId,
    timeline: Vec<TimelineEvent>,
    root_cause: RootCause,
    contributing_factors: Vec<Factor>,
    impact_analysis: ImpactAnalysis,
    resolution_steps: Vec<ResolutionStep>,
    lessons_learned: Vec<Lesson>,
    action_items: Vec<ActionItem>,
}
```

### 3.2 Implementation with Event Bus

```rust
struct IncidentManagerClient {
    rest_client: RestClient,
    event_bus: EventBusClient,
    circuit_breaker: CircuitBreaker,
    retry_policy: RetryPolicy,
    incident_cache: IncidentCache,
}

impl IncidentManagerClient {
    async fn new(config: IncidentManagerConfig) -> Result<Self> {
        BEGIN
            // REST client for API calls
            rest_client = RestClient::builder()
                .base_url(config.api_endpoint)
                .timeout(Duration::seconds(30))
                .auth(config.auth_token)
                .build()

            // Event bus for real-time updates
            event_bus = EventBusClient::connect(
                config.event_bus_url,
                topics: ["incidents", "runbooks", "alerts"]
            )

            // Circuit breaker with custom thresholds
            circuit_breaker = CircuitBreaker::new(
                failure_threshold: 3,
                timeout: Duration::seconds(20),
                reset_timeout: Duration::seconds(30) // Shorter for critical service
            )

            // Aggressive retry for critical operations
            retry_policy = RetryPolicy::exponential(
                initial_delay: Duration::milliseconds(50),
                max_delay: Duration::seconds(5),
                max_attempts: 5
            )

            // Subscribe to incident events
            event_bus.subscribe("incidents.*", handle_incident_event)

            RETURN IncidentManagerClient {
                rest_client,
                event_bus,
                circuit_breaker,
                retry_policy,
                incident_cache: IncidentCache::new()
            }
        END
    }
}
```

### 3.3 Incident Severity Classification Algorithm

```rust
impl IncidentManagerClient {
    async fn create_incident(&self, details: IncidentDetails) -> Result<IncidentId> {
        BEGIN
            // Automatically classify severity if not provided
            classified_severity = IF details.severity.is_unspecified() THEN
                self.classify_incident_severity(details)
            ELSE
                details.severity
            END IF

            // Enrich incident details with automated analysis
            enriched_details = IncidentDetails {
                severity: classified_severity,
                priority: calculate_priority(classified_severity, details.affected_services),
                tags: extract_tags(details),
                similar_incidents: find_similar_incidents(details),
                recommended_responders: suggest_responders(details),
                ...details
            }

            // Create incident via API
            incident_id = WITH_CIRCUIT_BREAKER(circuit_breaker) DO
                WITH_RETRY(retry_policy) DO
                    request = CreateIncidentRequest {
                        details: enriched_details,
                        auto_triage: true,
                        notify_oncall: should_notify_oncall(classified_severity)
                    }

                    response = rest_client.post("/api/v1/incidents", request)

                    IF response.status != 201 THEN
                        RETURN Error("Failed to create incident: " + response.error)
                    END IF

                    incident_id = response.json().incident_id
                    RETURN incident_id
                END WITH_RETRY
            END WITH_CIRCUIT_BREAKER

            // Publish incident created event
            event_bus.publish("incidents.created", {
                incident_id: incident_id,
                severity: classified_severity,
                timestamp: now()
            })

            // Cache incident
            incident_cache.put(incident_id, enriched_details)

            // Trigger automated triage workflow
            self.trigger_triage_workflow(incident_id).await?

            circuit_breaker.record_success()

            RETURN incident_id

        CATCH error AS e THEN
            circuit_breaker.record_failure()
            RETURN Error(e)
        END
    }

    // Helper: Classify incident severity using multiple signals
    fn classify_incident_severity(&self, details: IncidentDetails) -> Severity {
        BEGIN
            score = 0.0
            weights = SeverityWeights::default()

            // Factor 1: Affected services criticality
            FOR EACH service IN details.affected_services DO
                service_criticality = get_service_criticality(service)
                score += service_criticality * weights.service_weight
            END FOR

            // Factor 2: Error rate and volume
            IF details.evidence.error_logs.len() > 0 THEN
                error_rate = calculate_error_rate(details.evidence.error_logs)
                score += normalize(error_rate, 0, 1000) * weights.error_rate_weight
            END IF

            // Factor 3: Anomaly severity
            IF details.evidence.anomalies.len() > 0 THEN
                max_anomaly_severity = max(
                    details.evidence.anomalies.map(|a| a.severity_score)
                )
                score += max_anomaly_severity * weights.anomaly_weight
            END IF

            // Factor 4: Failed trace percentage
            IF details.evidence.failed_traces.len() > 0 THEN
                failure_rate = details.evidence.failed_traces.len() / total_traces()
                score += failure_rate * weights.trace_failure_weight
            END IF

            // Factor 5: Customer impact (from metrics)
            customer_impact = estimate_customer_impact(details.evidence.metric_snapshots)
            score += customer_impact * weights.customer_impact_weight

            // Factor 6: Historical patterns
            similar_incidents = find_similar_incidents(details)
            IF similar_incidents.len() > 0 THEN
                avg_severity = average(similar_incidents.map(|i| i.severity_score))
                score += avg_severity * weights.historical_weight
            END IF

            // Classify based on score
            severity = MATCH score {
                s IF s >= 0.9 => Severity::Critical,
                s IF s >= 0.7 => Severity::High,
                s IF s >= 0.4 => Severity::Medium,
                _ => Severity::Low
            }

            RETURN severity
        END
    }
}
```

### 3.4 Automated Triage Workflow

```rust
impl IncidentManagerClient {
    async fn trigger_triage_workflow(&self, incident_id: IncidentId) -> Result<()> {
        BEGIN
            // Fetch full incident details
            incident = self.get_incident(incident_id).await?

            // Run automated triage steps in parallel
            triage_results = run_parallel([
                self.analyze_root_cause(incident),
                self.identify_affected_components(incident),
                self.assess_blast_radius(incident),
                self.find_relevant_runbooks(incident),
                self.gather_diagnostic_data(incident)
            ]).await

            // Aggregate triage results
            triage_summary = TriageSummary {
                root_cause_hypotheses: triage_results.root_cause_analysis,
                affected_components: triage_results.affected_components,
                blast_radius: triage_results.blast_radius,
                suggested_runbooks: triage_results.relevant_runbooks,
                diagnostic_data: triage_results.diagnostic_data,
                confidence: calculate_triage_confidence(triage_results)
            }

            // Update incident with triage results
            self.update_incident(incident_id, {
                triage: triage_summary,
                status: "triaged"
            }).await?

            // Auto-execute runbook if high confidence
            IF triage_summary.confidence > 0.85 AND
               triage_summary.suggested_runbooks.len() == 1 THEN

                runbook = triage_summary.suggested_runbooks[0]
                IF runbook.auto_executable THEN
                    self.execute_runbook(
                        runbook.id,
                        extract_params(incident)
                    ).await?
                END IF
            END IF

            // Publish triage completed event
            event_bus.publish("incidents.triaged", {
                incident_id: incident_id,
                triage: triage_summary
            })

            RETURN Ok(())
        END
    }

    // Helper: Analyze potential root causes
    async fn analyze_root_cause(&self, incident: Incident) -> RootCauseAnalysis {
        BEGIN
            hypotheses = []

            // Analyze deployment correlation
            recent_deployments = query_deployments(
                time_range: (incident.created_at - Duration::hours(2), incident.created_at)
            )

            IF recent_deployments.len() > 0 THEN
                FOR EACH deployment IN recent_deployments DO
                    IF deployment.service IN incident.affected_services THEN
                        hypotheses.push(RootCauseHypothesis {
                            type: "deployment",
                            description: format!(
                                "Recent deployment of {} at {}",
                                deployment.service,
                                deployment.timestamp
                            ),
                            confidence: 0.8,
                            evidence: deployment,
                            mitigation: "Rollback deployment"
                        })
                    END IF
                END FOR
            END IF

            // Analyze infrastructure changes
            infra_changes = query_infrastructure_changes(
                time_range: (incident.created_at - Duration::hours(1), incident.created_at)
            )

            IF infra_changes.len() > 0 THEN
                hypotheses.push(RootCauseHypothesis {
                    type: "infrastructure",
                    description: format!("{} infrastructure changes detected", infra_changes.len()),
                    confidence: 0.7,
                    evidence: infra_changes,
                    mitigation: "Review and revert changes"
                })
            END IF

            // Analyze dependency failures
            dependency_health = check_dependency_health(incident.affected_services)

            FOR EACH (service, health) IN dependency_health DO
                IF health.status == "unhealthy" THEN
                    hypotheses.push(RootCauseHypothesis {
                        type: "dependency_failure",
                        description: format!("Dependency {} is unhealthy", service),
                        confidence: 0.75,
                        evidence: health,
                        mitigation: format!("Investigate {} service", service)
                    })
                END IF
            END FOR

            // Analyze resource exhaustion
            resource_metrics = gather_resource_metrics(incident.affected_services)

            FOR EACH (resource_type, metrics) IN resource_metrics DO
                IF metrics.utilization > 0.95 THEN
                    hypotheses.push(RootCauseHypothesis {
                        type: "resource_exhaustion",
                        description: format!("{} exhaustion detected", resource_type),
                        confidence: 0.85,
                        evidence: metrics,
                        mitigation: format!("Scale {} resources", resource_type)
                    })
                END IF
            END FOR

            // Use ML model for pattern recognition
            ml_analysis = call_ml_root_cause_model(incident)
            hypotheses.extend(ml_analysis.hypotheses)

            // Sort by confidence
            hypotheses.sort_by(|a, b| b.confidence.cmp(a.confidence))

            RETURN RootCauseAnalysis {
                hypotheses: hypotheses,
                most_likely: hypotheses.first(),
                investigation_needed: hypotheses.all(|h| h.confidence < 0.7)
            }
        END
    }
}
```

### 3.5 Runbook Selection and Execution

```rust
impl IncidentManagerClient {
    async fn execute_runbook(
        &self,
        runbook: RunbookId,
        params: Params
    ) -> Result<ExecutionId> {
        BEGIN
            // Validate runbook exists and is executable
            runbook_def = self.get_runbook(runbook).await?

            IF NOT runbook_def.is_executable THEN
                RETURN Error("Runbook is not executable")
            END IF

            // Validate parameters
            validation = validate_runbook_params(runbook_def, params)
            IF NOT validation.is_valid THEN
                RETURN Error("Invalid parameters: " + validation.errors)
            END IF

            // Create execution
            execution_id = WITH_CIRCUIT_BREAKER(circuit_breaker) DO
                WITH_RETRY(retry_policy) DO
                    request = ExecuteRunbookRequest {
                        runbook_id: runbook,
                        params: params,
                        execution_mode: "automated",
                        approval_required: runbook_def.requires_approval,
                        rollback_on_failure: true
                    }

                    response = rest_client.post("/api/v1/runbooks/execute", request)
                    RETURN response.json().execution_id
                END WITH_RETRY
            END WITH_CIRCUIT_BREAKER

            // Subscribe to execution events
            event_bus.subscribe(
                format!("runbooks.execution.{}", execution_id),
                |event| handle_runbook_event(execution_id, event)
            )

            // Monitor execution
            self.monitor_runbook_execution(execution_id).await?

            RETURN execution_id
        END
    }

    // Helper: Monitor runbook execution and handle errors
    async fn monitor_runbook_execution(&self, execution_id: ExecutionId) -> Result<()> {
        BEGIN
            timeout = Duration::minutes(30)
            start = now()

            LOOP
                IF now() - start > timeout THEN
                    self.cancel_execution(execution_id).await?
                    RETURN Error("Runbook execution timeout")
                END IF

                status = self.get_execution_status(execution_id).await?

                MATCH status.state {
                    Running => {
                        emit_progress(status.current_step, status.total_steps)
                        sleep(Duration::seconds(5))
                    }
                    Completed => {
                        emit_success(status.result)
                        RETURN Ok(())
                    }
                    Failed => {
                        // Attempt rollback if enabled
                        IF status.rollback_available THEN
                            self.rollback_execution(execution_id).await?
                        END IF

                        RETURN Error("Runbook execution failed: " + status.error)
                    }
                    Paused => {
                        // Wait for approval or manual intervention
                        emit_waiting_approval()
                        sleep(Duration::seconds(10))
                    }
                    Cancelled => {
                        RETURN Error("Runbook execution cancelled")
                    }
                }
            END LOOP
        END
    }

    // Helper: Find relevant runbooks for incident
    async fn find_relevant_runbooks(&self, incident: Incident) -> Vec<RunbookMatch> {
        BEGIN
            all_runbooks = self.list_runbooks().await?
            matches = []

            FOR EACH runbook IN all_runbooks DO
                score = calculate_runbook_relevance(runbook, incident)

                IF score > 0.5 THEN
                    match = RunbookMatch {
                        runbook: runbook,
                        relevance_score: score,
                        reason: explain_relevance(runbook, incident),
                        auto_executable: can_auto_execute(runbook, incident)
                    }

                    matches.push(match)
                END IF
            END FOR

            // Sort by relevance
            matches.sort_by(|a, b| b.relevance_score.cmp(a.relevance_score))

            RETURN matches
        END
    }

    // Helper: Calculate runbook relevance to incident
    fn calculate_runbook_relevance(&self, runbook: Runbook, incident: Incident) -> f64 {
        BEGIN
            score = 0.0

            // Match on affected services
            service_overlap = intersection(
                runbook.applicable_services,
                incident.affected_services
            ).len()

            score += (service_overlap / incident.affected_services.len()) * 0.4

            // Match on incident type/category
            IF runbook.incident_types.contains(incident.type) THEN
                score += 0.3
            END IF

            // Match on symptoms/signals
            symptom_matches = count_symptom_matches(runbook.symptoms, incident.evidence)
            score += (symptom_matches / runbook.symptoms.len()) * 0.2

            // Historical success rate
            IF runbook.execution_history.len() > 0 THEN
                success_rate = runbook.execution_history.success_count /
                              runbook.execution_history.total_count
                score += success_rate * 0.1
            END IF

            RETURN clamp(score, 0.0, 1.0)
        END
    }
}
```

### 3.6 Post-mortem Generation with Root Cause Analysis

```rust
impl IncidentManagerClient {
    async fn generate_postmortem(&self, id: IncidentId) -> Result<PostmortemReport> {
        BEGIN
            // Fetch complete incident data
            incident = self.get_incident(id).await?

            IF NOT incident.is_resolved THEN
                RETURN Error("Cannot generate postmortem for unresolved incident")
            END IF

            // Gather all incident data
            timeline = self.build_incident_timeline(id).await?
            metrics = self.gather_incident_metrics(id).await?
            logs = self.gather_incident_logs(id).await?
            traces = self.gather_incident_traces(id).await?
            communications = self.get_incident_communications(id).await?

            // Perform deep root cause analysis
            root_cause = self.perform_root_cause_analysis(
                incident,
                timeline,
                metrics,
                logs,
                traces
            ).await?

            // Analyze impact
            impact = self.analyze_incident_impact(
                incident,
                metrics,
                timeline
            ).await?

            // Extract lessons learned using LLM
            lessons = self.extract_lessons_learned(
                incident,
                timeline,
                root_cause
            ).await?

            // Generate action items
            action_items = self.generate_action_items(
                root_cause,
                lessons,
                incident
            ).await?

            // Compile postmortem report
            report = PostmortemReport {
                incident_id: id,
                incident_summary: summarize_incident(incident),
                timeline: timeline,
                root_cause: root_cause,
                contributing_factors: identify_contributing_factors(root_cause, timeline),
                impact_analysis: impact,
                resolution_steps: extract_resolution_steps(timeline),
                lessons_learned: lessons,
                action_items: action_items,
                metadata: PostmortemMetadata {
                    generated_at: now(),
                    contributors: extract_contributors(communications),
                    duration: incident.resolved_at - incident.created_at
                }
            }

            // Store postmortem
            WITH_RETRY(retry_policy) DO
                rest_client.post(
                    format!("/api/v1/incidents/{}/postmortem", id),
                    report
                )
            END WITH_RETRY

            // Publish postmortem event
            event_bus.publish("incidents.postmortem_created", {
                incident_id: id,
                postmortem_url: format!("/postmortems/{}", id)
            })

            RETURN report
        END
    }

    // Helper: Perform comprehensive root cause analysis
    async fn perform_root_cause_analysis(
        &self,
        incident: Incident,
        timeline: Vec<TimelineEvent>,
        metrics: Vec<MetricSnapshot>,
        logs: Vec<LogEntry>,
        traces: Vec<Trace>
    ) -> Result<RootCause> {
        BEGIN
            // Apply "5 Whys" technique programmatically
            initial_symptom = incident.initial_symptom
            whys = []
            current_why = initial_symptom

            FOR i IN 0..5 DO
                next_why = self.ask_why(current_why, timeline, metrics, logs, traces).await?
                whys.push(next_why)

                IF next_why.is_fundamental_cause THEN
                    BREAK
                END IF

                current_why = next_why.effect
            END FOR

            // Verify with fault tree analysis
            fault_tree = build_fault_tree(incident, timeline)
            verified_cause = verify_root_cause_with_fault_tree(whys.last(), fault_tree)

            // Cross-reference with similar incidents
            similar = find_similar_resolved_incidents(incident)
            correlation = correlate_root_causes(verified_cause, similar)

            // Generate comprehensive root cause
            root_cause = RootCause {
                primary_cause: verified_cause,
                causal_chain: whys,
                verification_confidence: correlation.confidence,
                similar_incidents: similar,
                technical_details: extract_technical_details(verified_cause, logs, traces),
                fix_verification: verify_fix_effectiveness(incident)
            }

            RETURN root_cause
        END
    }

    // Helper: Extract lessons learned using LLM
    async fn extract_lessons_learned(
        &self,
        incident: Incident,
        timeline: Vec<TimelineEvent>,
        root_cause: RootCause
    ) -> Result<Vec<Lesson>> {
        BEGIN
            prompt = format!("""
                Analyze this incident and extract key lessons learned:

                Incident: {incident.title}
                Severity: {incident.severity}
                Duration: {incident.duration}

                Root Cause: {root_cause.primary_cause}

                Timeline Summary:
                {timeline_summary}

                What were the key lessons? Focus on:
                1. What went wrong and why
                2. What went right (what helped resolve it)
                3. What should be improved
                4. What should be prevented

                Format as structured lessons with categories.
            """)

            llm_response = call_llm(prompt, temperature: 0.3)
            parsed_lessons = parse_lessons_from_llm(llm_response)

            // Categorize lessons
            categorized = []
            FOR EACH lesson IN parsed_lessons DO
                category = categorize_lesson(lesson)

                categorized.push(Lesson {
                    category: category,
                    description: lesson.description,
                    importance: assess_lesson_importance(lesson),
                    related_incidents: find_related_learning_incidents(lesson),
                    team_responsible: determine_responsible_team(lesson, category)
                })
            END FOR

            RETURN categorized
        END
    }

    // Helper: Generate actionable items
    async fn generate_action_items(
        &self,
        root_cause: RootCause,
        lessons: Vec<Lesson>,
        incident: Incident
    ) -> Result<Vec<ActionItem>> {
        BEGIN
            action_items = []

            // Generate preventive actions from root cause
            preventive = generate_preventive_actions(root_cause)
            action_items.extend(preventive)

            // Generate improvement actions from lessons
            FOR EACH lesson IN lessons DO
                improvements = generate_improvement_actions(lesson)
                action_items.extend(improvements)
            END FOR

            // Generate monitoring improvements
            monitoring = generate_monitoring_improvements(incident, root_cause)
            action_items.extend(monitoring)

            // Generate process improvements
            process = generate_process_improvements(incident.resolution_metadata)
            action_items.extend(process)

            // Prioritize and assign
            FOR EACH item IN action_items DO
                item.priority = calculate_action_priority(item, incident.severity)
                item.assigned_team = determine_owner_team(item)
                item.estimated_effort = estimate_effort(item)
                item.due_date = calculate_due_date(item.priority, item.estimated_effort)
            END FOR

            // Sort by priority
            action_items.sort_by(|a, b| b.priority.cmp(a.priority))

            RETURN action_items
        END
    }
}
```

---

## 4. Orchestrator Integration

### 4.1 Core Trait Definition

```rust
// Core trait for LLM-Orchestrator integration
trait OrchestratorIntegration {
    async fn define_workflow(&self, workflow: WorkflowDef) -> Result<WorkflowId>;
    async fn execute_workflow(&self, id: WorkflowId, params: Params) -> Result<ExecutionId>;
    async fn get_execution_status(&self, id: ExecutionId) -> Result<ExecutionStatus>;
    async fn cancel_execution(&self, id: ExecutionId) -> Result<()>;
}

// Data structures
struct WorkflowDef {
    name: String,
    description: String,
    steps: Vec<WorkflowStep>,
    error_handlers: Vec<ErrorHandler>,
    retry_policies: HashMap<StepId, RetryPolicy>,
    rollback_strategies: HashMap<StepId, RollbackStrategy>,
}

struct WorkflowStep {
    id: StepId,
    name: String,
    step_type: StepType,
    action: Action,
    inputs: Vec<Input>,
    outputs: Vec<Output>,
    dependencies: Vec<StepId>,
    conditions: Vec<Condition>,
    timeout: Duration,
}

struct ExecutionStatus {
    execution_id: ExecutionId,
    workflow_id: WorkflowId,
    state: ExecutionState,
    current_step: Option<StepId>,
    completed_steps: Vec<StepId>,
    failed_steps: Vec<(StepId, Error)>,
    step_outputs: HashMap<StepId, Value>,
    started_at: Timestamp,
    completed_at: Option<Timestamp>,
}

enum StepType {
    Sequential,
    Parallel,
    Conditional,
    Loop,
    Checkpoint,
    HumanApproval,
}
```

### 4.2 Implementation with State Machine

```rust
struct OrchestratorClient {
    rest_client: RestClient,
    workflow_engine_client: WorkflowEngineClient,
    circuit_breaker: CircuitBreaker,
    retry_policy: RetryPolicy,
    state_store: StateStore,
    event_bus: EventBusClient,
}

impl OrchestratorClient {
    async fn new(config: OrchestratorConfig) -> Result<Self> {
        BEGIN
            // REST client for workflow management
            rest_client = RestClient::builder()
                .base_url(config.api_endpoint)
                .timeout(Duration::seconds(60))
                .build()

            // Workflow engine client (gRPC)
            workflow_engine_client = WorkflowEngineClient::connect(
                config.engine_endpoint
            ).await?

            // Circuit breaker
            circuit_breaker = CircuitBreaker::new(
                failure_threshold: 5,
                timeout: Duration::seconds(45),
                reset_timeout: Duration::minutes(2)
            )

            // Retry policy
            retry_policy = RetryPolicy::exponential(
                initial_delay: Duration::milliseconds(200),
                max_delay: Duration::seconds(15),
                max_attempts: 4
            )

            // Distributed state store for workflow state
            state_store = StateStore::connect(
                config.state_store_url,
                consistency: "strong"
            ).await?

            // Event bus for workflow events
            event_bus = EventBusClient::connect(
                config.event_bus_url,
                topics: ["workflows", "executions"]
            )

            RETURN OrchestratorClient {
                rest_client,
                workflow_engine_client,
                circuit_breaker,
                retry_policy,
                state_store,
                event_bus
            }
        END
    }
}
```

### 4.3 Workflow Definition from Natural Language

```rust
impl OrchestratorClient {
    async fn define_workflow(&self, workflow: WorkflowDef) -> Result<WorkflowId> {
        BEGIN
            // Validate workflow definition
            validation = self.validate_workflow(workflow)
            IF NOT validation.is_valid THEN
                RETURN Error("Invalid workflow: " + validation.errors)
            END IF

            // Optimize workflow (reorder steps, parallelize where possible)
            optimized = self.optimize_workflow(workflow)

            // Register workflow
            workflow_id = WITH_CIRCUIT_BREAKER(circuit_breaker) DO
                WITH_RETRY(retry_policy) DO
                    request = DefineWorkflowRequest {
                        workflow: optimized,
                        validation_mode: "strict",
                        dry_run: false
                    }

                    response = rest_client.post("/api/v1/workflows", request)
                    RETURN response.json().workflow_id
                END WITH_RETRY
            END WITH_CIRCUIT_BREAKER

            // Store workflow in state store for quick access
            state_store.set(
                key: format!("workflow:{}", workflow_id),
                value: optimized,
                ttl: None // Persist indefinitely
            )

            // Publish workflow created event
            event_bus.publish("workflows.created", {
                workflow_id: workflow_id,
                name: workflow.name
            })

            circuit_breaker.record_success()

            RETURN workflow_id
        END
    }

    // Helper: Parse natural language to workflow definition
    async fn parse_natural_language_workflow(&self, description: String) -> Result<WorkflowDef> {
        BEGIN
            prompt = format!("""
                Convert this natural language workflow description into a structured workflow:

                Description: {description}

                Extract:
                1. Workflow steps (in order)
                2. Step dependencies
                3. Conditional logic
                4. Error handling requirements
                5. Parallel execution opportunities
                6. Required inputs and outputs

                Return as structured JSON matching WorkflowDef schema.
            """)

            llm_response = call_llm(prompt, temperature: 0.2)
            parsed = json_parse(llm_response)

            // Validate and build workflow
            workflow = WorkflowDef::from_json(parsed)?

            // Infer missing details
            workflow = self.infer_workflow_details(workflow)

            RETURN workflow
        END
    }

    // Helper: Optimize workflow execution plan
    fn optimize_workflow(&self, workflow: WorkflowDef) -> WorkflowDef {
        BEGIN
            optimized_steps = []

            // Build dependency graph
            dep_graph = build_step_dependency_graph(workflow.steps)

            // Identify parallel execution opportunities
            parallel_groups = identify_parallel_groups(dep_graph)

            FOR EACH group IN parallel_groups DO
                IF group.steps.len() > 1 THEN
                    // Create parallel step
                    parallel_step = WorkflowStep {
                        id: generate_step_id(),
                        name: format!("Parallel: {}", group.name),
                        step_type: StepType::Parallel,
                        action: Action::Parallel(group.steps),
                        inputs: merge_inputs(group.steps),
                        outputs: merge_outputs(group.steps),
                        dependencies: group.dependencies,
                        conditions: [],
                        timeout: max(group.steps.map(|s| s.timeout))
                    }

                    optimized_steps.push(parallel_step)
                ELSE
                    optimized_steps.push(group.steps[0])
                END IF
            END FOR

            // Add checkpoints for long-running workflows
            IF estimated_duration(optimized_steps) > Duration::minutes(10) THEN
                optimized_steps = insert_checkpoints(optimized_steps)
            END IF

            RETURN WorkflowDef {
                steps: optimized_steps,
                ...workflow
            }
        END
    }
}
```

### 4.4 Multi-step Execution Coordination

```rust
impl OrchestratorClient {
    async fn execute_workflow(
        &self,
        id: WorkflowId,
        params: Params
    ) -> Result<ExecutionId> {
        BEGIN
            // Fetch workflow definition
            workflow = state_store.get(format!("workflow:{}", id))
                .or_else(|| self.fetch_workflow(id).await)
                .ok_or(Error("Workflow not found"))?

            // Validate execution parameters
            param_validation = validate_workflow_params(workflow, params)
            IF NOT param_validation.is_valid THEN
                RETURN Error("Invalid parameters: " + param_validation.errors)
            END IF

            // Create execution
            execution_id = WITH_CIRCUIT_BREAKER(circuit_breaker) DO
                WITH_RETRY(retry_policy) DO
                    request = ExecuteWorkflowRequest {
                        workflow_id: id,
                        params: params,
                        execution_mode: "async",
                        enable_checkpoints: true,
                        enable_rollback: true
                    }

                    response = workflow_engine_client.execute_workflow(request).await?
                    RETURN response.execution_id
                END WITH_RETRY
            END WITH_CIRCUIT_BREAKER

            // Initialize execution state
            initial_state = ExecutionStatus {
                execution_id: execution_id,
                workflow_id: id,
                state: ExecutionState::Running,
                current_step: Some(workflow.steps.first().id),
                completed_steps: [],
                failed_steps: [],
                step_outputs: HashMap::new(),
                started_at: now(),
                completed_at: None
            }

            state_store.set(
                key: format!("execution:{}", execution_id),
                value: initial_state,
                ttl: Some(Duration::days(7))
            )

            // Subscribe to execution events
            event_bus.subscribe(
                format!("executions.{}", execution_id),
                |event| self.handle_execution_event(execution_id, event)
            )

            // Start monitoring
            spawn(self.monitor_execution(execution_id))

            circuit_breaker.record_success()

            RETURN execution_id
        END
    }

    // Helper: Monitor workflow execution
    async fn monitor_execution(&self, execution_id: ExecutionId) {
        BEGIN
            LOOP
                status = self.get_execution_status(execution_id).await
                    .unwrap_or_else(|_| ExecutionStatus::unknown())

                MATCH status.state {
                    Running => {
                        // Check for timeouts
                        IF now() - status.started_at > calculate_max_duration(status) THEN
                            self.handle_execution_timeout(execution_id).await
                            BREAK
                        END IF

                        // Update progress
                        progress = calculate_progress(status)
                        emit_progress_event(execution_id, progress)

                        sleep(Duration::seconds(2))
                    }
                    Completed => {
                        emit_completion_event(execution_id, status)
                        BREAK
                    }
                    Failed => {
                        self.handle_execution_failure(execution_id, status).await
                        BREAK
                    }
                    Cancelled => {
                        emit_cancellation_event(execution_id)
                        BREAK
                    }
                    Paused => {
                        emit_pause_event(execution_id, status.current_step)
                        sleep(Duration::seconds(5))
                    }
                }
            END LOOP
        END
    }

    // Helper: Handle execution events from workflow engine
    async fn handle_execution_event(&self, execution_id: ExecutionId, event: Event) {
        BEGIN
            MATCH event.type {
                StepStarted(step_id) => {
                    state_store.update(
                        format!("execution:{}", execution_id),
                        |state| {
                            state.current_step = Some(step_id)
                        }
                    )

                    emit_event("step_started", {execution_id, step_id})
                }

                StepCompleted(step_id, output) => {
                    state_store.update(
                        format!("execution:{}", execution_id),
                        |state| {
                            state.completed_steps.push(step_id)
                            state.step_outputs.insert(step_id, output)
                        }
                    )

                    emit_event("step_completed", {execution_id, step_id, output})
                }

                StepFailed(step_id, error) => {
                    // Attempt error recovery
                    recovery_result = self.attempt_step_recovery(
                        execution_id,
                        step_id,
                        error
                    ).await

                    IF recovery_result.is_err() THEN
                        state_store.update(
                            format!("execution:{}", execution_id),
                            |state| {
                                state.failed_steps.push((step_id, error))
                                state.state = ExecutionState::Failed
                            }
                        )
                    END IF

                    emit_event("step_failed", {execution_id, step_id, error})
                }

                CheckpointReached(checkpoint_id) => {
                    self.save_checkpoint(execution_id, checkpoint_id).await
                    emit_event("checkpoint_reached", {execution_id, checkpoint_id})
                }

                ApprovalRequired(step_id, approval_request) => {
                    self.request_approval(execution_id, step_id, approval_request).await
                    emit_event("approval_required", {execution_id, step_id})
                }
            }
        END
    }
}
```

### 4.5 State Machine Management

```rust
impl OrchestratorClient {
    async fn get_execution_status(&self, id: ExecutionId) -> Result<ExecutionStatus> {
        BEGIN
            // Try state store first (fast path)
            IF state_store.exists(format!("execution:{}", id)) THEN
                cached_status = state_store.get(format!("execution:{}", id))

                // Verify freshness
                IF cached_status.last_updated > now() - Duration::seconds(5) THEN
                    RETURN cached_status
                END IF
            END IF

            // Fetch from workflow engine (slow path)
            status = WITH_CIRCUIT_BREAKER(circuit_breaker) DO
                WITH_RETRY(retry_policy) DO
                    response = workflow_engine_client.get_execution_status(id).await?
                    RETURN parse_execution_status(response)
                END WITH_RETRY
            END WITH_CIRCUIT_BREAKER

            // Update state store
            state_store.set(
                key: format!("execution:{}", id),
                value: status,
                ttl: Some(Duration::days(7))
            )

            circuit_breaker.record_success()

            RETURN status
        END
    }

    // Helper: Transition execution state
    async fn transition_state(
        &self,
        execution_id: ExecutionId,
        from_state: ExecutionState,
        to_state: ExecutionState
    ) -> Result<()> {
        BEGIN
            // Validate state transition
            IF NOT is_valid_transition(from_state, to_state) THEN
                RETURN Error(format!(
                    "Invalid state transition: {} -> {}",
                    from_state,
                    to_state
                ))
            END IF

            // Atomic state update
            updated = state_store.compare_and_swap(
                key: format!("execution:{}", execution_id),
                expected_state: from_state,
                new_state: to_state
            )

            IF NOT updated THEN
                RETURN Error("State transition failed - concurrent modification")
            END IF

            // Execute state transition hooks
            self.execute_transition_hooks(execution_id, from_state, to_state).await?

            // Publish state change event
            event_bus.publish("executions.state_changed", {
                execution_id: execution_id,
                from_state: from_state,
                to_state: to_state,
                timestamp: now()
            })

            RETURN Ok(())
        END
    }

    // Helper: Check if state transition is valid
    fn is_valid_transition(&self, from: ExecutionState, to: ExecutionState) -> bool {
        BEGIN
            // Define valid state machine transitions
            valid_transitions = [
                (Pending, Running),
                (Running, Paused),
                (Running, Completed),
                (Running, Failed),
                (Running, Cancelled),
                (Paused, Running),
                (Paused, Cancelled),
                (Failed, Running), // Retry
            ]

            RETURN valid_transitions.contains((from, to))
        END
    }
}
```

### 4.6 Error Recovery and Rollback

```rust
impl OrchestratorClient {
    async fn attempt_step_recovery(
        &self,
        execution_id: ExecutionId,
        step_id: StepId,
        error: Error
    ) -> Result<()> {
        BEGIN
            // Fetch workflow definition
            status = self.get_execution_status(execution_id).await?
            workflow = self.fetch_workflow(status.workflow_id).await?
            step = workflow.steps.find(|s| s.id == step_id)
                .ok_or(Error("Step not found"))?

            // Check if retry policy exists for this step
            IF workflow.retry_policies.contains_key(step_id) THEN
                retry_policy = workflow.retry_policies.get(step_id)

                // Get retry count
                retry_count = state_store.get(
                    format!("execution:{}:step:{}:retries", execution_id, step_id)
                ).unwrap_or(0)

                IF retry_count < retry_policy.max_attempts THEN
                    // Wait with exponential backoff
                    delay = retry_policy.calculate_delay(retry_count)
                    sleep(delay)

                    // Increment retry count
                    state_store.set(
                        format!("execution:{}:step:{}:retries", execution_id, step_id),
                        retry_count + 1
                    )

                    // Retry step execution
                    self.retry_step_execution(execution_id, step_id).await?

                    RETURN Ok(())
                END IF
            END IF

            // Check if error handler exists
            IF workflow.error_handlers.iter().any(|h| h.handles(error)) THEN
                handler = workflow.error_handlers.find(|h| h.handles(error))

                // Execute error handler
                self.execute_error_handler(execution_id, step_id, handler).await?

                RETURN Ok(())
            END IF

            // No recovery possible - attempt rollback
            self.rollback_execution(execution_id).await?

            RETURN Error("Recovery failed - execution rolled back")
        END
    }

    // Helper: Rollback execution to last checkpoint
    async fn rollback_execution(&self, execution_id: ExecutionId) -> Result<()> {
        BEGIN
            // Get execution status
            status = self.get_execution_status(execution_id).await?
            workflow = self.fetch_workflow(status.workflow_id).await?

            // Find last checkpoint
            last_checkpoint = status.completed_steps
                .iter()
                .rev()
                .find(|&step_id| {
                    workflow.steps.find(|s| s.id == *step_id)
                        .map(|s| s.step_type == StepType::Checkpoint)
                        .unwrap_or(false)
                })

            // Determine rollback target
            rollback_target = last_checkpoint
                .or_else(|| Some(workflow.steps.first().id))
                .ok_or(Error("Cannot determine rollback target"))?

            // Get steps to rollback (in reverse order)
            steps_to_rollback = status.completed_steps
                .iter()
                .rev()
                .take_while(|&step_id| step_id != rollback_target)
                .collect()

            // Execute rollback for each step
            FOR EACH step_id IN steps_to_rollback DO
                step = workflow.steps.find(|s| s.id == *step_id)
                    .ok_or(Error("Step not found"))?

                IF workflow.rollback_strategies.contains_key(*step_id) THEN
                    strategy = workflow.rollback_strategies.get(step_id)

                    // Execute rollback
                    WITH_RETRY(retry_policy) DO
                        self.execute_rollback_strategy(
                            execution_id,
                            *step_id,
                            strategy,
                            status.step_outputs.get(step_id)
                        ).await?
                    END WITH_RETRY

                    emit_event("step_rolled_back", {execution_id, step_id})
                END IF
            END FOR

            // Update execution state
            state_store.update(
                format!("execution:{}", execution_id),
                |state| {
                    state.state = ExecutionState::RolledBack
                    state.current_step = Some(*rollback_target)
                    state.completed_steps.retain(|id| {
                        !steps_to_rollback.contains(id)
                    })
                }
            )

            emit_event("execution_rolled_back", {
                execution_id,
                rollback_target,
                steps_rolled_back: steps_to_rollback.len()
            })

            RETURN Ok(())
        END
    }

    // Helper: Cancel workflow execution
    async fn cancel_execution(&self, id: ExecutionId) -> Result<()> {
        BEGIN
            // Get current status
            status = self.get_execution_status(id).await?

            // Only running or paused executions can be cancelled
            IF status.state != ExecutionState::Running AND
               status.state != ExecutionState::Paused THEN
                RETURN Error("Cannot cancel execution in state: " + status.state)
            END IF

            // Request cancellation from workflow engine
            WITH_CIRCUIT_BREAKER(circuit_breaker) DO
                WITH_RETRY(retry_policy) DO
                    workflow_engine_client.cancel_execution(id).await?
                END WITH_RETRY
            END WITH_CIRCUIT_BREAKER

            // Update state
            self.transition_state(
                id,
                status.state,
                ExecutionState::Cancelled
            ).await?

            // Cleanup resources
            self.cleanup_execution_resources(id).await?

            RETURN Ok(())
        END
    }
}
```

---

## 5. Module Registry and Discovery

### 5.1 Module Registry Architecture

```rust
struct ModuleRegistry {
    modules: HashMap<ModuleId, ModuleInfo>,
    health_checker: HealthChecker,
    capability_detector: CapabilityDetector,
    version_manager: VersionManager,
    fallback_manager: FallbackManager,
}

struct ModuleInfo {
    id: ModuleId,
    name: String,
    module_type: ModuleType,
    version: Version,
    endpoint: Endpoint,
    capabilities: Vec<Capability>,
    health_status: HealthStatus,
    last_health_check: Timestamp,
    metadata: ModuleMetadata,
}

enum ModuleType {
    TestBench,
    Observatory,
    IncidentManager,
    Orchestrator,
}

struct HealthStatus {
    is_healthy: bool,
    status_code: StatusCode,
    latency: Duration,
    error_rate: f64,
    last_error: Option<Error>,
}

struct Capability {
    name: String,
    version: Version,
    enabled: bool,
    feature_flags: HashMap<String, bool>,
}
```

### 5.2 Module Health Checking

```rust
impl ModuleRegistry {
    async fn new(config: RegistryConfig) -> Result<Self> {
        BEGIN
            registry = ModuleRegistry {
                modules: HashMap::new(),
                health_checker: HealthChecker::new(
                    check_interval: Duration::seconds(30),
                    timeout: Duration::seconds(5)
                ),
                capability_detector: CapabilityDetector::new(),
                version_manager: VersionManager::new(),
                fallback_manager: FallbackManager::new()
            }

            // Discover and register modules
            registry.discover_modules(config.module_configs).await?

            // Start background health checking
            spawn(registry.health_check_loop())

            RETURN registry
        END
    }

    // Continuous health checking
    async fn health_check_loop(&self) {
        BEGIN
            LOOP
                FOR EACH (module_id, module) IN self.modules DO
                    health = self.check_module_health(module).await

                    // Update health status
                    self.modules.get_mut(module_id).health_status = health
                    self.modules.get_mut(module_id).last_health_check = now()

                    // Handle health changes
                    IF health.is_healthy AND NOT module.health_status.is_healthy THEN
                        emit_event("module_recovered", module_id)
                        self.fallback_manager.restore_module(module_id)
                    ELSE IF NOT health.is_healthy AND module.health_status.is_healthy THEN
                        emit_event("module_unhealthy", module_id)
                        self.fallback_manager.enable_fallback(module_id)
                    END IF
                END FOR

                sleep(self.health_checker.check_interval)
            END LOOP
        END
    }

    // Check health of individual module
    async fn check_module_health(&self, module: ModuleInfo) -> HealthStatus {
        BEGIN
            start = now()

            // Perform health check with timeout
            result = timeout(
                self.health_checker.timeout,
                self.perform_health_check(module)
            ).await

            latency = now() - start

            MATCH result {
                Ok(response) => {
                    RETURN HealthStatus {
                        is_healthy: response.status == "healthy",
                        status_code: response.status_code,
                        latency: latency,
                        error_rate: response.error_rate.unwrap_or(0.0),
                        last_error: None
                    }
                }
                Err(timeout_error) => {
                    RETURN HealthStatus {
                        is_healthy: false,
                        status_code: 504, // Gateway timeout
                        latency: self.health_checker.timeout,
                        error_rate: 1.0,
                        last_error: Some(timeout_error)
                    }
                }
            }
        END
    }

    // Perform actual health check
    async fn perform_health_check(&self, module: ModuleInfo) -> Result<HealthResponse> {
        BEGIN
            // Module-specific health check
            MATCH module.module_type {
                ModuleType::TestBench => {
                    client = TestBenchClient::new_for_health_check(module.endpoint)
                    response = client.health_check().await?
                    RETURN response
                }
                ModuleType::Observatory => {
                    client = ObservatoryClient::new_for_health_check(module.endpoint)
                    response = client.health_check().await?
                    RETURN response
                }
                ModuleType::IncidentManager => {
                    client = IncidentManagerClient::new_for_health_check(module.endpoint)
                    response = client.health_check().await?
                    RETURN response
                }
                ModuleType::Orchestrator => {
                    client = OrchestratorClient::new_for_health_check(module.endpoint)
                    response = client.health_check().await?
                    RETURN response
                }
            }
        END
    }
}
```

### 5.3 Capability Detection

```rust
impl ModuleRegistry {
    // Detect module capabilities
    async fn detect_capabilities(&self, module: ModuleInfo) -> Vec<Capability> {
        BEGIN
            capabilities = []

            // Fetch capability manifest
            manifest = self.fetch_capability_manifest(module).await
                .unwrap_or_default()

            // Probe for standard capabilities
            standard_caps = self.probe_standard_capabilities(module).await
            capabilities.extend(standard_caps)

            // Probe for optional capabilities
            optional_caps = self.probe_optional_capabilities(module, manifest).await
            capabilities.extend(optional_caps)

            // Detect feature flags
            FOR EACH capability IN capabilities DO
                feature_flags = self.detect_feature_flags(module, capability).await
                capability.feature_flags = feature_flags
            END FOR

            RETURN capabilities
        END
    }

    // Probe standard capabilities
    async fn probe_standard_capabilities(&self, module: ModuleInfo) -> Vec<Capability> {
        BEGIN
            capabilities = []

            MATCH module.module_type {
                ModuleType::TestBench => {
                    // Test for core test-bench capabilities
                    tests = [
                        ("generate_tests", test_generate_tests_capability),
                        ("execute_suite", test_execute_suite_capability),
                        ("get_coverage", test_get_coverage_capability)
                    ]

                    FOR EACH (name, test_fn) IN tests DO
                        IF test_fn(module).await THEN
                            capabilities.push(Capability {
                                name: name,
                                version: detect_capability_version(module, name),
                                enabled: true,
                                feature_flags: HashMap::new()
                            })
                        END IF
                    END FOR
                }

                ModuleType::Observatory => {
                    // Test for observatory capabilities
                    tests = [
                        ("query_metrics", test_query_metrics_capability),
                        ("search_logs", test_search_logs_capability),
                        ("query_traces", test_query_traces_capability),
                        ("detect_anomalies", test_detect_anomalies_capability)
                    ]

                    FOR EACH (name, test_fn) IN tests DO
                        IF test_fn(module).await THEN
                            capabilities.push(Capability {
                                name: name,
                                version: detect_capability_version(module, name),
                                enabled: true,
                                feature_flags: HashMap::new()
                            })
                        END IF
                    END FOR
                }

                // Similar for other module types...
            }

            RETURN capabilities
        END
    }

    // Detect feature flags for capability
    async fn detect_feature_flags(
        &self,
        module: ModuleInfo,
        capability: Capability
    ) -> HashMap<String, bool> {
        BEGIN
            feature_flags = HashMap::new()

            // Query module's feature flag endpoint
            flags_response = http_get(
                format!("{}/api/v1/capabilities/{}/flags", module.endpoint, capability.name)
            ).await

            IF flags_response.is_ok() THEN
                flags = parse_feature_flags(flags_response)
                feature_flags = flags
            ELSE
                // Fallback: probe common features
                common_features = get_common_features(capability.name)

                FOR EACH feature IN common_features DO
                    is_enabled = test_feature(module, capability.name, feature).await
                    feature_flags.insert(feature, is_enabled)
                END FOR
            END IF

            RETURN feature_flags
        END
    }
}
```

### 5.4 Version Compatibility Checking

```rust
impl ModuleRegistry {
    // Check version compatibility
    fn check_version_compatibility(
        &self,
        required_version: VersionRequirement,
        module_version: Version
    ) -> CompatibilityResult {
        BEGIN
            // Semantic versioning compatibility check
            MATCH required_version {
                Exact(version) => {
                    IF module_version == version THEN
                        RETURN CompatibilityResult::Compatible
                    ELSE
                        RETURN CompatibilityResult::Incompatible(
                            "Exact version mismatch"
                        )
                    END IF
                }

                MinVersion(min) => {
                    IF module_version >= min THEN
                        RETURN CompatibilityResult::Compatible
                    ELSE
                        RETURN CompatibilityResult::Incompatible(
                            format!("Version {} < required {}", module_version, min)
                        )
                    END IF
                }

                Range(min, max) => {
                    IF module_version >= min AND module_version <= max THEN
                        RETURN CompatibilityResult::Compatible
                    ELSE
                        RETURN CompatibilityResult::Incompatible(
                            format!("Version {} outside range [{}, {}]",
                                module_version, min, max)
                        )
                    END IF
                }

                Semver(constraint) => {
                    // Check semver compatibility
                    // e.g., "^1.2.3" means >= 1.2.3 and < 2.0.0
                    IF semver_matches(module_version, constraint) THEN
                        RETURN CompatibilityResult::Compatible
                    ELSE
                        RETURN CompatibilityResult::Incompatible(
                            format!("Version {} doesn't match constraint {}",
                                module_version, constraint)
                        )
                    END IF
                }
            }
        END
    }

    // Negotiate best compatible version
    async fn negotiate_version(
        &self,
        module_id: ModuleId,
        required: VersionRequirement
    ) -> Result<Version> {
        BEGIN
            module = self.modules.get(module_id)
                .ok_or(Error("Module not found"))?

            // Check current version
            current_compat = self.check_version_compatibility(
                required,
                module.version
            )

            IF current_compat.is_compatible() THEN
                RETURN module.version
            END IF

            // Query available versions
            available_versions = self.query_available_versions(module_id).await?

            // Find best compatible version
            FOR EACH version IN available_versions.sort_desc() DO
                compat = self.check_version_compatibility(required, version)

                IF compat.is_compatible() THEN
                    // Suggest upgrade
                    emit_warning(format!(
                        "Module {} version {} is incompatible. Version {} is available.",
                        module_id, module.version, version
                    ))

                    RETURN version
                END IF
            END FOR

            RETURN Error(format!(
                "No compatible version found for requirement: {}",
                required
            ))
        END
    }
}
```

### 5.5 Graceful Degradation

```rust
impl ModuleRegistry {
    // Get module client with fallback
    async fn get_module_client<T>(
        &self,
        module_type: ModuleType,
        version_requirement: VersionRequirement
    ) -> Result<T> where T: ModuleClient {
        BEGIN
            // Find module by type
            module = self.modules.values()
                .find(|m| m.module_type == module_type)
                .ok_or(Error("Module not found"))?

            // Check health
            IF NOT module.health_status.is_healthy THEN
                // Check if fallback is available
                IF self.fallback_manager.has_fallback(module.id) THEN
                    fallback = self.fallback_manager.get_fallback(module.id)

                    emit_warning(format!(
                        "Module {} unhealthy, using fallback: {}",
                        module.name, fallback.name
                    ))

                    RETURN create_client_from_fallback::<T>(fallback)
                ELSE
                    // Use degraded mode
                    emit_warning(format!(
                        "Module {} unhealthy, using degraded mode",
                        module.name
                    ))

                    RETURN create_degraded_client::<T>(module_type)
                END IF
            END IF

            // Check version compatibility
            compat = self.check_version_compatibility(
                version_requirement,
                module.version
            )

            IF NOT compat.is_compatible() THEN
                // Try version negotiation
                compatible_version = self.negotiate_version(
                    module.id,
                    version_requirement
                ).await?

                IF compatible_version != module.version THEN
                    emit_warning(format!(
                        "Version mismatch. Consider upgrading to {}",
                        compatible_version
                    ))
                END IF
            END IF

            // Create and return client
            client = T::new(module.endpoint, module.version).await?

            RETURN client
        END
    }

    // Create degraded client with limited functionality
    fn create_degraded_client<T>(module_type: ModuleType) -> T where T: ModuleClient {
        BEGIN
            MATCH module_type {
                ModuleType::TestBench => {
                    // Degraded test-bench: no test generation, only execution
                    RETURN DegradedTestBenchClient {
                        capabilities: ["execute_suite"],
                        limitations: [
                            "Test generation unavailable",
                            "Coverage reporting limited"
                        ]
                    }
                }

                ModuleType::Observatory => {
                    // Degraded observatory: cached data only
                    RETURN DegradedObservatoryClient {
                        capabilities: ["query_cached_metrics"],
                        limitations: [
                            "Real-time queries unavailable",
                            "Using cached data (may be stale)"
                        ]
                    }
                }

                ModuleType::IncidentManager => {
                    // Degraded incident manager: read-only
                    RETURN DegradedIncidentManagerClient {
                        capabilities: ["get_incident", "list_incidents"],
                        limitations: [
                            "Cannot create new incidents",
                            "Runbook execution unavailable"
                        ]
                    }
                }

                ModuleType::Orchestrator => {
                    // Degraded orchestrator: simple workflows only
                    RETURN DegradedOrchestratorClient {
                        capabilities: ["execute_simple_workflows"],
                        limitations: [
                            "Complex workflows unavailable",
                            "No parallel execution",
                            "Limited error recovery"
                        ]
                    }
                }
            }
        END
    }
}
```

---

## 6. Common Infrastructure

### 6.1 Circuit Breaker Implementation

```rust
struct CircuitBreaker {
    state: Arc<Mutex<CircuitState>>,
    failure_threshold: usize,
    timeout: Duration,
    reset_timeout: Duration,
    failure_count: Arc<AtomicUsize>,
    last_failure_time: Arc<Mutex<Option<Instant>>>,
}

enum CircuitState {
    Closed,      // Normal operation
    Open,        // Failing, reject requests
    HalfOpen,    // Testing if service recovered
}

impl CircuitBreaker {
    fn new(
        failure_threshold: usize,
        timeout: Duration,
        reset_timeout: Duration
    ) -> Self {
        BEGIN
            RETURN CircuitBreaker {
                state: Arc::new(Mutex::new(CircuitState::Closed)),
                failure_threshold: failure_threshold,
                timeout: timeout,
                reset_timeout: reset_timeout,
                failure_count: Arc::new(AtomicUsize::new(0)),
                last_failure_time: Arc::new(Mutex::new(None))
            }
        END
    }

    fn is_open(&self) -> bool {
        BEGIN
            state = self.state.lock()

            MATCH *state {
                CircuitState::Open => {
                    // Check if reset timeout has elapsed
                    last_failure = self.last_failure_time.lock()

                    IF last_failure.is_some() THEN
                        elapsed = now() - last_failure.unwrap()

                        IF elapsed > self.reset_timeout THEN
                            // Transition to half-open
                            *state = CircuitState::HalfOpen
                            RETURN false
                        END IF
                    END IF

                    RETURN true
                }
                _ => RETURN false
            }
        END
    }

    fn record_success(&self) {
        BEGIN
            state = self.state.lock()

            MATCH *state {
                CircuitState::HalfOpen => {
                    // Service recovered, close circuit
                    *state = CircuitState::Closed
                    self.failure_count.store(0, Ordering::SeqCst)
                    *self.last_failure_time.lock() = None
                    emit_event("circuit_closed")
                }
                CircuitState::Closed => {
                    // Reset failure count on success
                    self.failure_count.store(0, Ordering::SeqCst)
                }
                _ => {}
            }
        END
    }

    fn record_failure(&self) {
        BEGIN
            count = self.failure_count.fetch_add(1, Ordering::SeqCst) + 1
            *self.last_failure_time.lock() = Some(now())

            IF count >= self.failure_threshold THEN
                state = self.state.lock()

                IF *state == CircuitState::Closed THEN
                    *state = CircuitState::Open
                    emit_event("circuit_opened", {
                        failure_count: count,
                        threshold: self.failure_threshold
                    })
                END IF
            END IF
        END
    }
}
```

### 6.2 Retry Policy Implementation

```rust
struct RetryPolicy {
    initial_delay: Duration,
    max_delay: Duration,
    max_attempts: usize,
    backoff_multiplier: f64,
    jitter: f64,
}

impl RetryPolicy {
    fn exponential(
        initial_delay: Duration,
        max_delay: Duration,
        max_attempts: usize
    ) -> Self {
        BEGIN
            RETURN RetryPolicy {
                initial_delay: initial_delay,
                max_delay: max_delay,
                max_attempts: max_attempts,
                backoff_multiplier: 2.0,
                jitter: 0.0
            }
        END
    }

    fn exponential_with_jitter(
        initial_delay: Duration,
        max_delay: Duration,
        max_attempts: usize,
        jitter: f64
    ) -> Self {
        BEGIN
            RETURN RetryPolicy {
                initial_delay: initial_delay,
                max_delay: max_delay,
                max_attempts: max_attempts,
                backoff_multiplier: 2.0,
                jitter: jitter
            }
        END
    }

    fn calculate_delay(&self, attempt: usize) -> Duration {
        BEGIN
            // Exponential backoff
            delay = self.initial_delay * (self.backoff_multiplier ^ attempt)

            // Cap at max delay
            delay = min(delay, self.max_delay)

            // Add jitter if configured
            IF self.jitter > 0.0 THEN
                jitter_amount = delay * self.jitter * random(-1.0, 1.0)
                delay = delay + jitter_amount
            END IF

            RETURN max(delay, Duration::zero())
        END
    }
}

// Retry macro
macro WITH_RETRY(policy: RetryPolicy) DO
    BEGIN
        last_error = None

        FOR attempt IN 0..policy.max_attempts DO
            result = EXECUTE_BLOCK()

            MATCH result {
                Ok(value) => RETURN value
                Err(error) => {
                    last_error = Some(error)

                    // Don't retry on last attempt
                    IF attempt < policy.max_attempts - 1 THEN
                        delay = policy.calculate_delay(attempt)
                        emit_event("retry_attempt", {
                            attempt: attempt + 1,
                            max_attempts: policy.max_attempts,
                            delay: delay,
                            error: error
                        })
                        sleep(delay)
                    END IF
                }
            }
        END FOR

        RETURN Err(last_error.unwrap())
    END
END

// Circuit breaker macro
macro WITH_CIRCUIT_BREAKER(breaker: CircuitBreaker) DO
    BEGIN
        IF breaker.is_open() THEN
            RETURN Error("Circuit breaker is open")
        END IF

        result = EXECUTE_BLOCK()

        MATCH result {
            Ok(value) => {
                breaker.record_success()
                RETURN value
            }
            Err(error) => {
                breaker.record_failure()
                RETURN Error(error)
            }
        }
    END
END
```

---

## Summary

This design provides comprehensive pseudocode for all four module integrations with:

1. **Test-Bench Integration**: Natural language test generation, streaming execution, coverage analysis
2. **Observatory Integration**: Multi-signal querying (metrics/logs/traces), anomaly detection with AI explanations
3. **Incident-Manager Integration**: Automated severity classification, triage workflows, runbook execution, AI-powered postmortems
4. **Orchestrator Integration**: Workflow DSL from natural language, state machine management, comprehensive error recovery

All integrations include:
- Circuit breaker patterns for resilience
- Retry policies with exponential backoff and jitter
- Health checking and graceful degradation
- Event-driven architecture
- Caching strategies
- Comprehensive error handling

The module registry provides:
- Automatic service discovery
- Continuous health monitoring
- Capability detection
- Version compatibility checking
- Fallback mechanisms

This architecture ensures the LLM-CoPilot-Agent can operate reliably even when integrated modules experience issues.
