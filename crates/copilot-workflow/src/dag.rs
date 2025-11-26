//! Directed Acyclic Graph (DAG) for workflow execution order

use crate::step::WorkflowStep;
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::DfsPostOrder;
use petgraph::Direction;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DagValidationError {
    #[error("Cycle detected in workflow: {0}")]
    CycleDetected(String),

    #[error("Missing dependency: step {step} depends on {dependency} which does not exist")]
    MissingDependency {
        step: String,
        dependency: String,
    },

    #[error("Empty workflow: no steps defined")]
    EmptyWorkflow,

    #[error("Duplicate step ID: {0}")]
    DuplicateStepId(String),

    #[error("Invalid step configuration: {0}")]
    InvalidStep(String),
}

/// Directed Acyclic Graph representation of a workflow
#[derive(Debug, Clone)]
pub struct WorkflowDag {
    /// Petgraph DiGraph for DAG operations
    graph: DiGraph<String, ()>,
    /// Map from step ID to graph node index
    step_to_node: HashMap<String, NodeIndex>,
    /// Map from node index to step ID
    node_to_step: HashMap<NodeIndex, String>,
    /// Map from step ID to step definition
    steps: HashMap<String, WorkflowStep>,
}

impl WorkflowDag {
    /// Create a new workflow DAG from steps
    pub fn new(steps: Vec<WorkflowStep>) -> Result<Self, DagValidationError> {
        if steps.is_empty() {
            return Err(DagValidationError::EmptyWorkflow);
        }

        let mut graph = DiGraph::new();
        let mut step_to_node = HashMap::new();
        let mut node_to_step = HashMap::new();
        let mut step_map = HashMap::new();

        // Check for duplicate IDs
        let mut seen_ids = HashSet::new();
        for step in &steps {
            if !seen_ids.insert(step.id.clone()) {
                return Err(DagValidationError::DuplicateStepId(step.id.clone()));
            }
        }

        // Create nodes for all steps
        for step in &steps {
            let node = graph.add_node(step.id.clone());
            step_to_node.insert(step.id.clone(), node);
            node_to_step.insert(node, step.id.clone());
            step_map.insert(step.id.clone(), step.clone());
        }

        // Create edges based on dependencies
        for step in &steps {
            let to_node = step_to_node[&step.id];

            for dep_id in &step.dependencies {
                let from_node = step_to_node.get(dep_id).ok_or_else(|| {
                    DagValidationError::MissingDependency {
                        step: step.id.clone(),
                        dependency: dep_id.clone(),
                    }
                })?;

                graph.add_edge(*from_node, to_node, ());
            }
        }

        let mut dag = Self {
            graph,
            step_to_node,
            node_to_step,
            steps: step_map,
        };

        // Validate the DAG
        dag.validate()?;

        Ok(dag)
    }

    /// Validate the DAG for cycles and other issues
    pub fn validate(&self) -> Result<(), DagValidationError> {
        // Check for cycles using DFS
        if petgraph::algo::is_cyclic_directed(&self.graph) {
            let cycle = self.find_cycle();
            return Err(DagValidationError::CycleDetected(cycle));
        }

        Ok(())
    }

    /// Find a cycle in the graph (for error reporting)
    fn find_cycle(&self) -> String {
        // Simple cycle detection for error message
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();

        for node in self.graph.node_indices() {
            if let Some(cycle) = self.find_cycle_util(node, &mut visited, &mut rec_stack) {
                return cycle
                    .iter()
                    .map(|n| self.node_to_step[n].clone())
                    .collect::<Vec<_>>()
                    .join(" -> ");
            }
        }

        "unknown".to_string()
    }

    fn find_cycle_util(
        &self,
        node: NodeIndex,
        visited: &mut HashSet<NodeIndex>,
        rec_stack: &mut HashSet<NodeIndex>,
    ) -> Option<Vec<NodeIndex>> {
        if rec_stack.contains(&node) {
            return Some(vec![node]);
        }

        if visited.contains(&node) {
            return None;
        }

        visited.insert(node);
        rec_stack.insert(node);

        for neighbor in self.graph.neighbors_directed(node, Direction::Outgoing) {
            if let Some(mut cycle) = self.find_cycle_util(neighbor, visited, rec_stack) {
                cycle.push(node);
                return Some(cycle);
            }
        }

        rec_stack.remove(&node);
        None
    }

    /// Get steps in topological order (execution order: dependencies before dependents)
    pub fn topological_sort(&self) -> Vec<String> {
        let topo = petgraph::algo::toposort(&self.graph, None)
            .expect("DAG should be validated");

        // petgraph's toposort returns nodes in topological order
        // (dependencies before dependents for directed graphs with edges pointing forward)
        topo.into_iter()
            .map(|node| self.node_to_step[&node].clone())
            .collect()
    }

    /// Get steps that are ready to execute (no pending dependencies)
    pub fn get_ready_steps(&self, completed: &HashSet<String>) -> Vec<String> {
        let mut ready = Vec::new();

        for (step_id, step) in &self.steps {
            // Skip if already completed
            if completed.contains(step_id) {
                continue;
            }

            // Check if all dependencies are completed
            let all_deps_completed = step.dependencies.iter().all(|dep| completed.contains(dep));

            if all_deps_completed {
                ready.push(step_id.clone());
            }
        }

        ready
    }

    /// Get steps that can be executed in parallel at the current state
    pub fn get_parallel_ready_steps(
        &self,
        completed: &HashSet<String>,
        running: &HashSet<String>,
    ) -> Vec<Vec<String>> {
        let ready = self.get_ready_steps(completed);
        let available: Vec<_> = ready
            .into_iter()
            .filter(|id| !running.contains(id))
            .collect();

        if available.is_empty() {
            return Vec::new();
        }

        // Group steps that can run in parallel (no dependencies between them)
        let mut groups = Vec::new();
        let mut grouped = HashSet::new();

        for step_id in &available {
            if grouped.contains(step_id) {
                continue;
            }

            let mut group = vec![step_id.clone()];
            grouped.insert(step_id.clone());

            // Find other steps that can run in parallel with this one
            for other_id in &available {
                if grouped.contains(other_id) {
                    continue;
                }

                // Check if there's no path between these steps
                if !self.has_path(step_id, other_id) && !self.has_path(other_id, step_id) {
                    group.push(other_id.clone());
                    grouped.insert(other_id.clone());
                }
            }

            groups.push(group);
        }

        groups
    }

    /// Check if there's a path from one step to another
    fn has_path(&self, from: &str, to: &str) -> bool {
        let from_node = match self.step_to_node.get(from) {
            Some(n) => *n,
            None => return false,
        };

        let to_node = match self.step_to_node.get(to) {
            Some(n) => *n,
            None => return false,
        };

        petgraph::algo::has_path_connecting(&self.graph, from_node, to_node, None)
    }

    /// Get a step by ID
    pub fn get_step(&self, step_id: &str) -> Option<&WorkflowStep> {
        self.steps.get(step_id)
    }

    /// Get all steps
    pub fn get_all_steps(&self) -> &HashMap<String, WorkflowStep> {
        &self.steps
    }

    /// Get root steps (steps with no dependencies)
    pub fn get_root_steps(&self) -> Vec<String> {
        self.steps
            .values()
            .filter(|step| step.dependencies.is_empty())
            .map(|step| step.id.clone())
            .collect()
    }

    /// Get leaf steps (steps with no dependents)
    pub fn get_leaf_steps(&self) -> Vec<String> {
        self.graph
            .node_indices()
            .filter(|&node| {
                self.graph
                    .neighbors_directed(node, Direction::Outgoing)
                    .count()
                    == 0
            })
            .map(|node| self.node_to_step[&node].clone())
            .collect()
    }

    /// Get dependencies of a step
    pub fn get_dependencies(&self, step_id: &str) -> Vec<String> {
        self.steps
            .get(step_id)
            .map(|s| s.dependencies.clone())
            .unwrap_or_default()
    }

    /// Get dependents of a step (steps that depend on this step)
    pub fn get_dependents(&self, step_id: &str) -> Vec<String> {
        let node = match self.step_to_node.get(step_id) {
            Some(n) => *n,
            None => return Vec::new(),
        };

        self.graph
            .neighbors_directed(node, Direction::Outgoing)
            .map(|n| self.node_to_step[&n].clone())
            .collect()
    }

    /// Get number of steps
    pub fn len(&self) -> usize {
        self.steps.len()
    }

    /// Check if DAG is empty
    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::step::{StepAction, StepType};

    fn create_test_step(id: &str, name: &str, deps: Vec<String>) -> WorkflowStep {
        WorkflowStep::new(
            name,
            StepType::Action,
            StepAction::Wait { duration_secs: 1 },
        )
        .with_id(id)
        .with_dependencies(deps)
    }

    #[test]
    fn test_dag_creation() {
        let steps = vec![
            create_test_step("step1", "Step 1", vec![]),
            create_test_step("step2", "Step 2", vec!["step1".to_string()]),
            create_test_step("step3", "Step 3", vec!["step1".to_string()]),
        ];

        let dag = WorkflowDag::new(steps).unwrap();
        assert_eq!(dag.len(), 3);
    }

    #[test]
    fn test_cycle_detection() {
        let steps = vec![
            create_test_step("step1", "Step 1", vec!["step2".to_string()]),
            create_test_step("step2", "Step 2", vec!["step1".to_string()]),
        ];

        let result = WorkflowDag::new(steps);
        assert!(matches!(result, Err(DagValidationError::CycleDetected(_))));
    }

    #[test]
    fn test_missing_dependency() {
        let steps = vec![
            create_test_step("step1", "Step 1", vec![]),
            create_test_step("step2", "Step 2", vec!["step3".to_string()]),
        ];

        let result = WorkflowDag::new(steps);
        assert!(matches!(result, Err(DagValidationError::MissingDependency { .. })));
    }

    #[test]
    fn test_topological_sort() {
        let steps = vec![
            create_test_step("step1", "Step 1", vec![]),
            create_test_step("step2", "Step 2", vec!["step1".to_string()]),
            create_test_step("step3", "Step 3", vec!["step2".to_string()]),
        ];

        let dag = WorkflowDag::new(steps).unwrap();
        let sorted = dag.topological_sort();

        // Verify the result contains all steps
        assert_eq!(sorted.len(), 3);
        assert!(sorted.contains(&"step1".to_string()));
        assert!(sorted.contains(&"step2".to_string()));
        assert!(sorted.contains(&"step3".to_string()));

        // Verify that dependencies come before dependents
        let step1_pos = sorted.iter().position(|s| s == "step1").unwrap();
        let step2_pos = sorted.iter().position(|s| s == "step2").unwrap();
        let step3_pos = sorted.iter().position(|s| s == "step3").unwrap();

        assert!(step1_pos < step2_pos, "step1 should come before step2");
        assert!(step2_pos < step3_pos, "step2 should come before step3");
    }

    #[test]
    fn test_get_ready_steps() {
        let steps = vec![
            create_test_step("step1", "Step 1", vec![]),
            create_test_step("step2", "Step 2", vec!["step1".to_string()]),
            create_test_step("step3", "Step 3", vec!["step1".to_string()]),
        ];

        let dag = WorkflowDag::new(steps).unwrap();

        let ready = dag.get_ready_steps(&HashSet::new());
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0], "step1");

        let mut completed = HashSet::new();
        completed.insert("step1".to_string());
        let ready = dag.get_ready_steps(&completed);
        assert_eq!(ready.len(), 2);
        assert!(ready.contains(&"step2".to_string()));
        assert!(ready.contains(&"step3".to_string()));
    }
}
