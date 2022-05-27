//! Treblle.com connector for Rust Actix web framework.
//!
//! ## Stay in tune with your APIs
//!
//! Treblle makes it super easy to understand what's going on with your APIs and the apps that use them.
//!
//! ## With Treblle
//!
//! - Auto-generated and updated docs
//! - Self service integration support
//! - Get in-depth API insights
//! - 90% less meetings
//! - Complete API analytics
//! - Complete picture of your API
//! - 1 single awesome service
//! - Know exactly what's ok and what not
//! - Quality score of your API
//! - 1 click testing
//! - Device detection
//! - Endpoint grouping
//!
//! # Installation
//!
//! Go to [Treblle.com](https://treblle.com/) register and create a project, copy your `project_id` and go and get your `api_key` from settings.
//!
//! Add this crate to your Rust Actix v4 powered application as a regular middleware, give it `project_id` and `api_key`, turn on the [features you might need](https://docs.rs/actix-treblle/latest/actix_treblle/)
//! and thats it! Watch your requests get logged in Treblle project.
//!
//! Example:
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
