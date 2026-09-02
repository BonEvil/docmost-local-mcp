pub mod auth;
pub mod debug;
pub mod docmost_client;
pub mod network_policy;
pub mod position;
pub mod prosemirror;
pub mod server;
pub mod startup_config;
pub mod stdio_compat;
pub mod storage;
pub mod types;
pub mod version;

/// Stable product identity used by every user- and protocol-visible surface.
///
/// Keep these values owned by this crate. In particular, MCP metadata must not use
/// `rmcp::model::Implementation::default()`: that default describes the dependency
/// crate which constructs it, not this product.
pub const PRODUCT_NAME: &str = env!("CARGO_PKG_NAME");
pub const PRODUCT_TITLE: &str = "Docmost MCP";
pub const PRODUCT_VERSION: &str = env!("CARGO_PKG_VERSION");
