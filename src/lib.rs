//! Treblle.com connector for Rust Actix web framework.
//!
//! ```rust,ignore
//! use actix_web::{App, HttpServer};
//! use actix_treblle::Treblle;
//!
//! #[actix_web::main]
//! async fn main() -> std::io::Result<()> {
//!    HttpServer::new(|| {
//!        App::new()
//!            .wrap(Treblle::new("project_id".to_string(), "api_key".to_string()))
//!            .route("/hello", web::get().to(|| async { "Hello World!" }))
//!    })
//!    .bind(("127.0.0.1", 8080))?
//!    .run()
//!    .await
//! }
//! ```
mod extractors;
mod middleware;
mod payload;
mod treblle;

pub use treblle::Treblle;
