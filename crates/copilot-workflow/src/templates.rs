//! Workflow templates library
//!
//! Provides reusable workflow templates with parameterization.

use crate::{
    engine::WorkflowDefinition,
    step::{StepType, StepAction, WorkflowStep},
    Result, WorkflowError,
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

/// Template parameter definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateParameter {
    /// Parameter name
    pub name: String,
    /// Display label
    pub label: String,
    /// Description
    pub description: Option<String>,
    /// Parameter type
    pub param_type: ParameterType,
    /// Whether parameter is required
    pub required: bool,
    /// Default value
    pub default_value: Option<serde_json::Value>,
    /// Validation rules
    pub validation: Option<ParameterValidation>,
}

/// Parameter types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParameterType {
    String,
    Number,
    Boolean,
    Array,
    Object,
    Select { options: Vec<SelectOption> },
    Secret,
}

/// Select option
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectOption {
    pub value: String,
    pub label: String,
}

/// Parameter validation rules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterValidation {
    /// Minimum value (for numbers)
    pub min: Option<f64>,
    /// Maximum value (for numbers)
    pub max: Option<f64>,
    /// Minimum length (for strings)
    pub min_length: Option<usize>,
    /// Maximum length (for strings)
    pub max_length: Option<usize>,
    /// Regex pattern (for strings)
    pub pattern: Option<String>,
}

/// Workflow template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowTemplate {
    /// Template ID
    pub id: String,
    /// Template name
    pub name: String,
    /// Description
    pub description: String,
    /// Template category
    pub category: String,
    /// Template tags
    pub tags: Vec<String>,
    /// Icon (emoji or icon name)
    pub icon: Option<String>,
    /// Template parameters
    pub parameters: Vec<TemplateParameter>,
    /// Template definition (with placeholders)
    pub definition: WorkflowDefinition,
    /// Version
    pub version: String,
    /// Author
    pub author: Option<String>,
    /// Whether template is public
    pub is_public: bool,
    /// Tenant ID (for private templates)
    pub tenant_id: Option<String>,
    /// Usage count
    pub usage_count: u64,
    /// Created at
    pub created_at: DateTime<Utc>,
    /// Updated at
    pub updated_at: DateTime<Utc>,
}

impl WorkflowTemplate {
    /// Create a new template
    pub fn new(name: &str, description: &str, definition: WorkflowDefinition) -> Self {
        let now = Utc::now();
        Self {
            id: format!(
                "tmpl_{}",
                uuid::Uuid::new_v4().to_string().replace('-', "")
            ),
            name: name.to_string(),
            description: description.to_string(),
            category: "General".to_string(),
            tags: Vec::new(),
            icon: None,
            parameters: Vec::new(),
            definition,
            version: "1.0.0".to_string(),
            author: None,
            is_public: false,
            tenant_id: None,
            usage_count: 0,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn with_category(mut self, category: &str) -> Self {
        self.category = category.to_string();
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    pub fn with_icon(mut self, icon: &str) -> Self {
        self.icon = Some(icon.to_string());
        self
    }

    pub fn with_parameter(mut self, param: TemplateParameter) -> Self {
        self.parameters.push(param);
        self
    }

    pub fn with_author(mut self, author: &str) -> Self {
        self.author = Some(author.to_string());
        self
    }

    pub fn make_public(mut self) -> Self {
        self.is_public = true;
        self
    }

    pub fn with_tenant(mut self, tenant_id: &str) -> Self {
        self.tenant_id = Some(tenant_id.to_string());
        self
    }

    /// Validate parameters
    pub fn validate_params(&self, params: &serde_json::Value) -> Result<()> {
        for param_def in &self.parameters {
            let value = params.get(&param_def.name);

            // Check required
            if param_def.required && value.is_none() {
                return Err(WorkflowError::InvalidDefinition(format!(
                    "Missing required parameter: {}",
                    param_def.name
                )));
            }

            // Validate if value present
            if let Some(value) = value {
                self.validate_param_value(param_def, value)?;
            }
        }

        Ok(())
    }

    /// Validate a single parameter value
    fn validate_param_value(
        &self,
        param_def: &TemplateParameter,
        value: &serde_json::Value,
    ) -> Result<()> {
        // Type validation
        match &param_def.param_type {
            ParameterType::String | ParameterType::Secret => {
                if !value.is_string() {
                    return Err(WorkflowError::InvalidDefinition(format!(
                        "Parameter {} must be a string",
                        param_def.name
                    )));
                }

                if let Some(ref validation) = param_def.validation {
                    let s = value.as_str().unwrap();

                    if let Some(min) = validation.min_length {
                        if s.len() < min {
                            return Err(WorkflowError::InvalidDefinition(format!(
                                "Parameter {} must be at least {} characters",
                                param_def.name, min
                            )));
                        }
                    }

                    if let Some(max) = validation.max_length {
                        if s.len() > max {
                            return Err(WorkflowError::InvalidDefinition(format!(
                                "Parameter {} must be at most {} characters",
                                param_def.name, max
                            )));
                        }
                    }

                    if let Some(ref pattern) = validation.pattern {
                        let re = regex::Regex::new(pattern).map_err(|_| {
                            WorkflowError::InvalidDefinition("Invalid regex pattern".to_string())
                        })?;
                        if !re.is_match(s) {
                            return Err(WorkflowError::InvalidDefinition(format!(
                                "Parameter {} does not match required pattern",
                                param_def.name
                            )));
                        }
                    }
                }
            }
            ParameterType::Number => {
                if !value.is_number() {
                    return Err(WorkflowError::InvalidDefinition(format!(
                        "Parameter {} must be a number",
                        param_def.name
                    )));
                }

                if let Some(ref validation) = param_def.validation {
                    let n = value.as_f64().unwrap();

                    if let Some(min) = validation.min {
                        if n < min {
                            return Err(WorkflowError::InvalidDefinition(format!(
                                "Parameter {} must be at least {}",
                                param_def.name, min
                            )));
                        }
                    }

                    if let Some(max) = validation.max {
                        if n > max {
                            return Err(WorkflowError::InvalidDefinition(format!(
                                "Parameter {} must be at most {}",
                                param_def.name, max
                            )));
                        }
                    }
                }
            }
            ParameterType::Boolean => {
                if !value.is_boolean() {
                    return Err(WorkflowError::InvalidDefinition(format!(
                        "Parameter {} must be a boolean",
                        param_def.name
                    )));
                }
            }
            ParameterType::Array => {
                if !value.is_array() {
                    return Err(WorkflowError::InvalidDefinition(format!(
                        "Parameter {} must be an array",
                        param_def.name
                    )));
                }
            }
            ParameterType::Object => {
                if !value.is_object() {
                    return Err(WorkflowError::InvalidDefinition(format!(
                        "Parameter {} must be an object",
                        param_def.name
                    )));
                }
            }
            ParameterType::Select { options } => {
                if let Some(s) = value.as_str() {
                    if !options.iter().any(|o| o.value == s) {
                        return Err(WorkflowError::InvalidDefinition(format!(
                            "Parameter {} must be one of the allowed options",
                            param_def.name
                        )));
                    }
                } else {
                    return Err(WorkflowError::InvalidDefinition(format!(
                        "Parameter {} must be a string",
                        param_def.name
                    )));
                }
            }
        }

        Ok(())
    }

    /// Instantiate the template with parameters
    pub fn instantiate(&self, params: &serde_json::Value) -> Result<WorkflowDefinition> {
        // Validate parameters
        self.validate_params(params)?;

        // Create a copy of the definition
        let mut definition = self.definition.clone();

        // Generate new workflow ID
        definition.id = format!(
            "wf_{}",
            uuid::Uuid::new_v4().to_string().replace('-', "")
        );

        // Replace placeholders in the definition
        let definition_json = serde_json::to_string(&definition)
            .map_err(|e| WorkflowError::Serialization(e))?;

        let mut result = definition_json;

        // Replace parameter placeholders
        for param_def in &self.parameters {
            let placeholder = format!("{{{{ {} }}}}", param_def.name);
            let alt_placeholder = format!("{{{{{}}}}} ", param_def.name);

            let value = params
                .get(&param_def.name)
                .or(param_def.default_value.as_ref())
                .map(|v| {
                    if v.is_string() {
                        v.as_str().unwrap().to_string()
                    } else {
                        v.to_string()
                    }
                })
                .unwrap_or_default();

            result = result.replace(&placeholder, &value);
            result = result.replace(&alt_placeholder, &value);
        }

        // Parse back to definition
        let instantiated: WorkflowDefinition = serde_json::from_str(&result)
            .map_err(|e| WorkflowError::Serialization(e))?;

        debug!(
            template_id = %self.id,
            workflow_id = %instantiated.id,
            "Instantiated workflow from template"
        );

        Ok(instantiated)
    }
}

/// Template repository trait
#[async_trait]
pub trait TemplateRepository: Send + Sync {
    async fn save(&self, template: &WorkflowTemplate) -> Result<()>;
    async fn get(&self, id: &str) -> Result<Option<WorkflowTemplate>>;
    async fn list(&self) -> Result<Vec<WorkflowTemplate>>;
    async fn list_public(&self) -> Result<Vec<WorkflowTemplate>>;
    async fn list_by_category(&self, category: &str) -> Result<Vec<WorkflowTemplate>>;
    async fn list_by_tenant(&self, tenant_id: &str) -> Result<Vec<WorkflowTemplate>>;
    async fn search(&self, query: &str) -> Result<Vec<WorkflowTemplate>>;
    async fn delete(&self, id: &str) -> Result<()>;
    async fn update(&self, template: &WorkflowTemplate) -> Result<()>;
    async fn increment_usage(&self, id: &str) -> Result<()>;
}

/// In-memory template repository
pub struct InMemoryTemplateRepository {
    templates: RwLock<HashMap<String, WorkflowTemplate>>,
}

impl InMemoryTemplateRepository {
    pub fn new() -> Self {
        Self {
            templates: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for InMemoryTemplateRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TemplateRepository for InMemoryTemplateRepository {
    async fn save(&self, template: &WorkflowTemplate) -> Result<()> {
        let mut templates = self.templates.write().await;
        templates.insert(template.id.clone(), template.clone());
        Ok(())
    }

    async fn get(&self, id: &str) -> Result<Option<WorkflowTemplate>> {
        let templates = self.templates.read().await;
        Ok(templates.get(id).cloned())
    }

    async fn list(&self) -> Result<Vec<WorkflowTemplate>> {
        let templates = self.templates.read().await;
        let mut list: Vec<_> = templates.values().cloned().collect();
        list.sort_by(|a, b| b.usage_count.cmp(&a.usage_count));
        Ok(list)
    }

    async fn list_public(&self) -> Result<Vec<WorkflowTemplate>> {
        let templates = self.templates.read().await;
        let mut list: Vec<_> = templates.values().filter(|t| t.is_public).cloned().collect();
        list.sort_by(|a, b| b.usage_count.cmp(&a.usage_count));
        Ok(list)
    }

    async fn list_by_category(&self, category: &str) -> Result<Vec<WorkflowTemplate>> {
        let templates = self.templates.read().await;
        Ok(templates
            .values()
            .filter(|t| t.category == category)
            .cloned()
            .collect())
    }

    async fn list_by_tenant(&self, tenant_id: &str) -> Result<Vec<WorkflowTemplate>> {
        let templates = self.templates.read().await;
        Ok(templates
            .values()
            .filter(|t| t.tenant_id.as_deref() == Some(tenant_id) || t.is_public)
            .cloned()
            .collect())
    }

    async fn search(&self, query: &str) -> Result<Vec<WorkflowTemplate>> {
        let templates = self.templates.read().await;
        let query_lower = query.to_lowercase();

        Ok(templates
            .values()
            .filter(|t| {
                t.name.to_lowercase().contains(&query_lower)
                    || t.description.to_lowercase().contains(&query_lower)
                    || t.tags.iter().any(|tag| tag.to_lowercase().contains(&query_lower))
            })
            .cloned()
            .collect())
    }

    async fn delete(&self, id: &str) -> Result<()> {
        let mut templates = self.templates.write().await;
        templates.remove(id);
        Ok(())
    }

    async fn update(&self, template: &WorkflowTemplate) -> Result<()> {
        let mut templates = self.templates.write().await;
        if templates.contains_key(&template.id) {
            templates.insert(template.id.clone(), template.clone());
            Ok(())
        } else {
            Err(WorkflowError::NotFound(template.id.clone()))
        }
    }

    async fn increment_usage(&self, id: &str) -> Result<()> {
        let mut templates = self.templates.write().await;
        if let Some(template) = templates.get_mut(id) {
            template.usage_count += 1;
            Ok(())
        } else {
            Err(WorkflowError::NotFound(id.to_string()))
        }
    }
}

/// Template library service
pub struct TemplateLibrary {
    repository: Arc<dyn TemplateRepository>,
}

impl TemplateLibrary {
    pub fn new(repository: Arc<dyn TemplateRepository>) -> Self {
        Self { repository }
    }

    /// Create a new template
    pub async fn create(&self, template: WorkflowTemplate) -> Result<WorkflowTemplate> {
        self.repository.save(&template).await?;

        info!(
            template_id = %template.id,
            name = %template.name,
            "Created workflow template"
        );

        Ok(template)
    }

    /// Get a template by ID
    pub async fn get(&self, id: &str) -> Result<Option<WorkflowTemplate>> {
        self.repository.get(id).await
    }

    /// List all templates
    pub async fn list(&self) -> Result<Vec<WorkflowTemplate>> {
        self.repository.list().await
    }

    /// List public templates
    pub async fn list_public(&self) -> Result<Vec<WorkflowTemplate>> {
        self.repository.list_public().await
    }

    /// List templates by category
    pub async fn list_by_category(&self, category: &str) -> Result<Vec<WorkflowTemplate>> {
        self.repository.list_by_category(category).await
    }

    /// List templates available to a tenant
    pub async fn list_for_tenant(&self, tenant_id: &str) -> Result<Vec<WorkflowTemplate>> {
        self.repository.list_by_tenant(tenant_id).await
    }

    /// Search templates
    pub async fn search(&self, query: &str) -> Result<Vec<WorkflowTemplate>> {
        self.repository.search(query).await
    }

    /// Get template categories
    pub async fn get_categories(&self) -> Result<Vec<String>> {
        let templates = self.repository.list().await?;
        let mut categories: Vec<String> = templates.iter().map(|t| t.category.clone()).collect();
        categories.sort();
        categories.dedup();
        Ok(categories)
    }

    /// Instantiate a template
    pub async fn instantiate(
        &self,
        template_id: &str,
        params: serde_json::Value,
    ) -> Result<WorkflowDefinition> {
        let template = self
            .repository
            .get(template_id)
            .await?
            .ok_or_else(|| WorkflowError::NotFound(template_id.to_string()))?;

        let definition = template.instantiate(&params)?;

        // Increment usage count
        let _ = self.repository.increment_usage(template_id).await;

        info!(
            template_id = %template_id,
            workflow_id = %definition.id,
            "Instantiated workflow from template"
        );

        Ok(definition)
    }

    /// Delete a template
    pub async fn delete(&self, id: &str) -> Result<()> {
        self.repository.delete(id).await?;
        info!(template_id = %id, "Deleted workflow template");
        Ok(())
    }
}

/// Pre-built template builders
pub struct TemplateBuilders;

impl TemplateBuilders {
    /// Create a simple sequential workflow template
    pub fn sequential_workflow(name: &str, description: &str, steps: Vec<&str>) -> WorkflowTemplate {
        let workflow_steps: Vec<WorkflowStep> = steps
            .iter()
            .enumerate()
            .map(|(i, step_name)| {
                let mut step = WorkflowStep::new(
                    format!("{{{{ step_{}_name }}}}", i + 1),
                    StepType::Action,
                    StepAction::Custom {
                        handler: step_name.to_string(),
                        parameters: HashMap::new(),
                    },
                )
                .with_id(format!("step-{}", i + 1));

                if i > 0 {
                    step = step.with_dependency(format!("step-{}", i));
                }

                step
            })
            .collect();

        let definition = WorkflowDefinition {
            id: "{{ workflow_id }}".to_string(),
            name: "{{ workflow_name }}".to_string(),
            description: description.to_string(),
            steps: workflow_steps,
            metadata: HashMap::new(),
            timeout_secs: None,
        };

        let mut template = WorkflowTemplate::new(name, description, definition)
            .with_category("Sequential")
            .with_tags(vec!["sequential".to_string(), "simple".to_string()]);

        // Add name parameters for each step
        for i in 0..steps.len() {
            template = template.with_parameter(TemplateParameter {
                name: format!("step_{}_name", i + 1),
                label: format!("Step {} Name", i + 1),
                description: Some(format!("Name for step {}", i + 1)),
                param_type: ParameterType::String,
                required: false,
                default_value: Some(serde_json::Value::String(steps[i].to_string())),
                validation: None,
            });
        }

        template
            .with_parameter(TemplateParameter {
                name: "workflow_name".to_string(),
                label: "Workflow Name".to_string(),
                description: Some("Name for the workflow".to_string()),
                param_type: ParameterType::String,
                required: true,
                default_value: None,
                validation: Some(ParameterValidation {
                    min_length: Some(3),
                    max_length: Some(100),
                    ..Default::default()
                }),
            })
            .with_parameter(TemplateParameter {
                name: "workflow_id".to_string(),
                label: "Workflow ID".to_string(),
                description: Some("Unique identifier".to_string()),
                param_type: ParameterType::String,
                required: false,
                default_value: Some(serde_json::Value::String(
                    uuid::Uuid::new_v4().to_string(),
                )),
                validation: None,
            })
    }

    /// Create an approval workflow template
    pub fn approval_workflow(name: &str, description: &str) -> WorkflowTemplate {
        let steps = vec![
            WorkflowStep::new(
                "Submit Request",
                StepType::Action,
                StepAction::Custom {
                    handler: "submit".to_string(),
                    parameters: HashMap::new(),
                },
            )
            .with_id("submit"),
            WorkflowStep::new(
                "Review",
                StepType::Approval,
                StepAction::Wait { duration_secs: 0 },
            )
            .with_id("review")
            .with_dependency("submit")
            .with_timeout(86400), // 24 hours
            WorkflowStep::new(
                "Approve",
                StepType::Approval,
                StepAction::Wait { duration_secs: 0 },
            )
            .with_id("approve")
            .with_dependency("review")
            .with_timeout(86400),
            WorkflowStep::new(
                "Execute",
                StepType::Action,
                StepAction::Custom {
                    handler: "execute".to_string(),
                    parameters: HashMap::new(),
                },
            )
            .with_id("execute")
            .with_dependency("approve"),
        ];

        let definition = WorkflowDefinition {
            id: "approval-workflow".to_string(),
            name: name.to_string(),
            description: description.to_string(),
            steps,
            metadata: HashMap::new(),
            timeout_secs: None,
        };

        WorkflowTemplate::new(name, description, definition)
            .with_category("Approval")
            .with_tags(vec!["approval".to_string(), "review".to_string()])
            .with_icon("✅")
            .with_parameter(TemplateParameter {
                name: "approval_timeout_hours".to_string(),
                label: "Approval Timeout (hours)".to_string(),
                description: Some("How long to wait for approval".to_string()),
                param_type: ParameterType::Number,
                required: false,
                default_value: Some(serde_json::json!(24)),
                validation: Some(ParameterValidation {
                    min: Some(1.0),
                    max: Some(168.0),
                    ..Default::default()
                }),
            })
    }
}

impl Default for ParameterValidation {
    fn default() -> Self {
        Self {
            min: None,
            max: None,
            min_length: None,
            max_length: None,
            pattern: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_template() -> WorkflowTemplate {
        let definition = WorkflowDefinition {
            id: "{{ workflow_id }}".to_string(),
            name: "{{ workflow_name }}".to_string(),
            description: "Test workflow".to_string(),
            steps: vec![WorkflowStep::new(
                "{{ step_name }}",
                StepType::Action,
                StepAction::Wait { duration_secs: 1 },
            )
            .with_id("step-1")],
            metadata: HashMap::new(),
            timeout_secs: None,
        };

        WorkflowTemplate::new("Test Template", "A test template", definition)
            .with_parameter(TemplateParameter {
                name: "workflow_id".to_string(),
                label: "Workflow ID".to_string(),
                description: None,
                param_type: ParameterType::String,
                required: true,
                default_value: None,
                validation: None,
            })
            .with_parameter(TemplateParameter {
                name: "workflow_name".to_string(),
                label: "Workflow Name".to_string(),
                description: None,
                param_type: ParameterType::String,
                required: true,
                default_value: None,
                validation: Some(ParameterValidation {
                    min_length: Some(3),
                    ..Default::default()
                }),
            })
            .with_parameter(TemplateParameter {
                name: "step_name".to_string(),
                label: "Step Name".to_string(),
                description: None,
                param_type: ParameterType::String,
                required: false,
                default_value: Some(serde_json::Value::String("Default Step".to_string())),
                validation: None,
            })
    }

    #[test]
    fn test_parameter_validation() {
        let template = create_test_template();

        // Missing required parameter
        let params = serde_json::json!({
            "workflow_id": "test-123"
        });
        assert!(template.validate_params(&params).is_err());

        // Valid parameters
        let params = serde_json::json!({
            "workflow_id": "test-123",
            "workflow_name": "Test Workflow"
        });
        assert!(template.validate_params(&params).is_ok());

        // Invalid min_length
        let params = serde_json::json!({
            "workflow_id": "test-123",
            "workflow_name": "ab"  // Too short
        });
        assert!(template.validate_params(&params).is_err());
    }

    #[test]
    fn test_template_instantiation() {
        let template = create_test_template();

        let params = serde_json::json!({
            "workflow_id": "my-workflow",
            "workflow_name": "My Custom Workflow",
            "step_name": "Custom Step"
        });

        let definition = template.instantiate(&params).unwrap();

        assert_eq!(definition.name, "My Custom Workflow");
        assert_eq!(definition.steps[0].name, "Custom Step");
    }

    #[tokio::test]
    async fn test_template_repository() {
        let repo = InMemoryTemplateRepository::new();
        let template = create_test_template().make_public();

        repo.save(&template).await.unwrap();

        let retrieved = repo.get(&template.id).await.unwrap();
        assert!(retrieved.is_some());

        let public = repo.list_public().await.unwrap();
        assert_eq!(public.len(), 1);

        let search_results = repo.search("test").await.unwrap();
        assert_eq!(search_results.len(), 1);

        repo.increment_usage(&template.id).await.unwrap();
        let updated = repo.get(&template.id).await.unwrap().unwrap();
        assert_eq!(updated.usage_count, 1);
    }

    #[test]
    fn test_builder_sequential() {
        let template =
            TemplateBuilders::sequential_workflow("Data Pipeline", "Process data", vec!["fetch", "transform", "load"]);

        assert_eq!(template.category, "Sequential");
        assert_eq!(template.definition.steps.len(), 3);
    }

    #[test]
    fn test_builder_approval() {
        let template = TemplateBuilders::approval_workflow("Request Approval", "Approve requests");

        assert_eq!(template.category, "Approval");
        assert!(template.definition.steps.iter().any(|s| s.step_type == StepType::Approval));
    }
}
