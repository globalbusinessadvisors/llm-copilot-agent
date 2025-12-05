//! LLM-Marketplace Adapter
//!
//! Thin adapter for consuming LLM-Marketplace service.
//! Provides service marketplace for LLM tools and plugins.

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info};

use crate::{
    AdapterError, AdapterResult, HealthStatus, ModuleCapabilities, ModuleAdapter,
    circuit_breaker::CircuitBreaker,
    retry::with_retry,
};

/// Marketplace listing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplaceListing {
    pub id: String,
    pub name: String,
    pub description: String,
    pub listing_type: ListingType,
    pub publisher: Publisher,
    pub pricing: ListingPricing,
    pub versions: Vec<ListingVersion>,
    pub ratings: ListingRatings,
    pub tags: Vec<String>,
    pub published_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ListingType {
    Plugin,
    Model,
    PromptLibrary,
    Integration,
    Tool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Publisher {
    pub id: String,
    pub name: String,
    pub verified: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListingPricing {
    pub pricing_model: PricingModel,
    pub base_price: Option<f64>,
    pub currency: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PricingModel {
    Free,
    OneTime,
    Subscription,
    Usage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListingVersion {
    pub version: String,
    pub release_notes: String,
    pub released_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListingRatings {
    pub average: f32,
    pub count: u64,
}

/// Purchase/installation record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Installation {
    pub id: String,
    pub listing_id: String,
    pub version: String,
    pub status: InstallationStatus,
    pub installed_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InstallationStatus {
    Active,
    Suspended,
    Expired,
}

/// Trait defining Marketplace adapter operations
#[async_trait]
pub trait MarketplaceAdapter: Send + Sync {
    /// Browse marketplace listings
    async fn browse(&self, filters: BrowseFilters) -> AdapterResult<Vec<MarketplaceListing>>;

    /// Get listing details
    async fn get_listing(&self, listing_id: &str) -> AdapterResult<MarketplaceListing>;

    /// Install a listing
    async fn install(&self, listing_id: &str, version: Option<String>) -> AdapterResult<Installation>;

    /// List installations
    async fn list_installations(&self) -> AdapterResult<Vec<Installation>>;

    /// Uninstall a listing
    async fn uninstall(&self, installation_id: &str) -> AdapterResult<()>;
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BrowseFilters {
    pub listing_type: Option<ListingType>,
    pub tags: Option<Vec<String>>,
    pub pricing_model: Option<PricingModel>,
    pub min_rating: Option<f32>,
    pub query: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

/// HTTP client for LLM-Marketplace service
pub struct MarketplaceClient {
    base_url: String,
    client: Client,
    circuit_breaker: CircuitBreaker,
}

impl MarketplaceClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            client: Client::new(),
            circuit_breaker: CircuitBreaker::default(),
        }
    }

    async fn send_request<T: Serialize, R: for<'de> Deserialize<'de>>(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<&T>,
    ) -> AdapterResult<R> {
        let url = format!("{}{}", self.base_url, path);
        debug!("Sending {} request to {}", method, url);

        let response = with_retry(3, || async {
            self.circuit_breaker.call(|| async {
                let mut request = self.client.request(method.clone(), &url);
                if let Some(body) = body {
                    request = request.json(body);
                }
                request
                    .send()
                    .await
                    .map_err(|e| AdapterError::RequestFailed(e.to_string()))
            }).await
        }).await?;

        if !response.status().is_success() {
            return Err(AdapterError::RequestFailed(format!(
                "Request failed with status: {}",
                response.status()
            )));
        }

        response
            .json::<R>()
            .await
            .map_err(|e| AdapterError::SerializationError(e.to_string()))
    }
}

#[async_trait]
impl ModuleAdapter for MarketplaceClient {
    async fn health_check(&self) -> AdapterResult<HealthStatus> {
        let url = format!("{}/health", self.base_url);
        match self.client.get(&url).send().await {
            Ok(response) if response.status().is_success() => {
                Ok(HealthStatus::healthy("LLM-Marketplace is operational"))
            }
            Ok(response) => {
                Ok(HealthStatus::unhealthy(format!("Status: {}", response.status())))
            }
            Err(e) => Ok(HealthStatus::unhealthy(format!("Unreachable: {}", e))),
        }
    }

    async fn execute(&self, request: serde_json::Value) -> AdapterResult<serde_json::Value> {
        let filters: BrowseFilters = serde_json::from_value(request)
            .map_err(|e| AdapterError::SerializationError(e.to_string()))?;
        let response = self.browse(filters).await?;
        serde_json::to_value(response)
            .map_err(|e| AdapterError::SerializationError(e.to_string()))
    }

    async fn get_capabilities(&self) -> AdapterResult<ModuleCapabilities> {
        Ok(ModuleCapabilities {
            name: "LLM-Marketplace".to_string(),
            version: "0.1.0".to_string(),
            features: vec![
                "browse_listings".to_string(),
                "install_plugins".to_string(),
                "manage_subscriptions".to_string(),
                "ratings_reviews".to_string(),
            ],
            endpoints: vec![
                "/listings".to_string(),
                "/listings/{id}".to_string(),
                "/installations".to_string(),
            ],
        })
    }
}

#[async_trait]
impl MarketplaceAdapter for MarketplaceClient {
    async fn browse(&self, filters: BrowseFilters) -> AdapterResult<Vec<MarketplaceListing>> {
        debug!("Browsing marketplace");
        self.send_request(reqwest::Method::POST, "/listings/search", Some(&filters)).await
    }

    async fn get_listing(&self, listing_id: &str) -> AdapterResult<MarketplaceListing> {
        debug!("Getting listing: {}", listing_id);
        let path = format!("/listings/{}", listing_id);
        self.send_request::<(), MarketplaceListing>(reqwest::Method::GET, &path, None).await
    }

    async fn install(&self, listing_id: &str, version: Option<String>) -> AdapterResult<Installation> {
        info!("Installing listing: {}", listing_id);
        let request = serde_json::json!({
            "listing_id": listing_id,
            "version": version
        });
        self.send_request(reqwest::Method::POST, "/installations", Some(&request)).await
    }

    async fn list_installations(&self) -> AdapterResult<Vec<Installation>> {
        debug!("Listing installations");
        self.send_request::<(), Vec<Installation>>(reqwest::Method::GET, "/installations", None).await
    }

    async fn uninstall(&self, installation_id: &str) -> AdapterResult<()> {
        info!("Uninstalling: {}", installation_id);
        let path = format!("/installations/{}", installation_id);
        self.send_request::<(), ()>(reqwest::Method::DELETE, &path, None).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_client_creation() {
        let client = MarketplaceClient::new("http://localhost:8105");
        assert_eq!(client.base_url, "http://localhost:8105");
    }

    #[tokio::test]
    async fn test_capabilities() {
        let client = MarketplaceClient::new("http://localhost:8105");
        let caps = client.get_capabilities().await.unwrap();
        assert_eq!(caps.name, "LLM-Marketplace");
    }
}
