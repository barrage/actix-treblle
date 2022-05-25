[![Crates.io](https://img.shields.io/crates/v/actix-treblle.svg)](https://crates.io/crates/actix-treblle)

# actix-treblle

```toml
actix-treblle = "4.0.0"
```

Treblle.com connector for Rust Actix web framework.

```rust
use actix_web::{App, HttpServer};
use actix_treblle::Treblle;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
   HttpServer::new(|| {
       App::new()
           .wrap(Treblle::new("project_id".to_string(), "api_key".to_string()))
           .route("/hello", web::get().to(|| async { "Hello World!" }))
   })
   .bind(("127.0.0.1", 8080))?
   .run()
   .await
}
```

## License

Licensed under either of

* Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or https://www.apache.org/licenses/LICENSE-2.0)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or https://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you, as defined in the Apache-2.0
license, shall be dual licensed as above, without any additional terms or
conditions.
