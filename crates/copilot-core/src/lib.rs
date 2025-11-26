pub mod cache;
pub mod config;
pub mod error;
pub mod events;
pub mod traits;
pub mod types;

// Re-export specific items to avoid ambiguity
pub use config::*;
pub use error::*;
pub use types::*;

// Re-export cache module items (simpler API)
pub use cache::Cache as SimpleCache;

// Re-export events module items
pub use events::{Event, EventPublisher as EventPublisherSimple, EventSubscriber};

// Re-export traits module items (more comprehensive interfaces)
pub use traits::{Cache, EventPublisher, HealthCheck, HealthStatus, Repository, Transaction};
