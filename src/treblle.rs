pub struct Treblle {
    pub(crate) project_id: String,
    pub(crate) api_key: String,
    pub(crate) debug: bool,
    pub(crate) masking_fields: Vec<String>,
    pub(crate) ignored_routes: Vec<String>,
}

impl Treblle {
    /// Create the middleware and wrap your application with it.
    ///
    /// ```rust,ignore
    /// HttpServer::new(|| {
    ///     App::new()
    ///         .wrap(actix_treblle::Treblle::new("project_id".to_string(), "api_key".to_string()))
    ///         .route("/hello", web::get().to(|| async { "Hello World!" }))
    /// })
    /// .bind(("127.0.0.1", 8080))?
    /// .run()
    /// .await
    /// ```
    pub fn new(project_id: String, api_key: String) -> Treblle {
        Treblle {
            project_id,
            api_key,
            debug: false,
            masking_fields: vec![
                "password".to_string(),
                "pwd".to_string(),
                "secret".to_string(),
                "password_confirmation".to_string(),
                "passwordConfirmation".to_string(),
                "cc".to_string(),
                "card_number".to_string(),
                "cardNumber".to_string(),
                "ccv".to_string(),
                "ssn".to_string(),
                "credit_score".to_string(),
                "creditScore".to_string(),
            ],
            ignored_routes: vec![],
        }
    }

    /// Turn on the debug mode
    ///
    /// ```rust,ignore
    /// HttpServer::new(|| {
    ///     App::new()
    ///         .wrap(
    ///             actix_treblle::Treblle::new("project_id".to_string(), "api_key".to_string())
    ///                .debug()
    ///         )
    ///         .route("/hello", web::get().to(|| async { "Hello World!" }))
    /// })
    /// .bind(("127.0.0.1", 8080))?
    /// .run()
    /// .await
    /// ```
    pub fn debug(mut self) -> Treblle {
        self.debug = true;
        self
    }

    /// If you don't wish to have default masking fields, or simply want to remove the
    /// default ones use this method when wrapping your application with this middleware.
    ///
    /// ```rust,ignore
    /// HttpServer::new(|| {
    ///     App::new()
    ///         .wrap(
    ///             actix_treblle::Treblle::new("project_id".to_string(), "api_key".to_string())
    ///                .clear_masking_fields()
    ///         )
    ///         .route("/hello", web::get().to(|| async { "Hello World!" }))
    /// })
    /// .bind(("127.0.0.1", 8080))?
    /// .run()
    /// .await
    /// ```
    pub fn clear_masking_fields(mut self) -> Treblle {
        self.masking_fields.clear();
        self
    }

    /// Set masking fields that will be masked before the request
    /// leaves your application
    ///
    /// Default masking fields:
    /// - "password"
    /// - "pwd"
    /// - "secret"
    /// - "password_confirmation"
    /// - "passwordConfirmation"
    /// - "cc"
    /// - "card_number"
    /// - "cardNumber"
    /// - "ccv"
    /// - "ssn"
    /// - "credit_score"
    /// - "creditScore"
    ///
    /// Add a vector of route matching patterns, same as you would define them in your application.
    ///
    /// ```rust,ignore
    /// HttpServer::new(|| {
    ///     App::new()
    ///         .wrap(
    ///             actix_treblle::Treblle::new("project_id".to_string(), "api_key".to_string())
    ///                .add_masking_fields(vec![
    ///                    "password".to_string(),
    ///                    "ssl_key".to_string(),
    ///                ])
    ///         )
    ///         .route("/hello", web::get().to(|| async { "Hello World!" }))
    /// })
    /// .bind(("127.0.0.1", 8080))?
    /// .run()
    /// .await
    /// ```
    pub fn add_masking_fields(mut self, mut fields: Vec<String>) -> Treblle {
        self.masking_fields.append(&mut fields);
        self
    }

    /// Add routes that will be ignored for logging
    ///
    /// Add a vector of route matching patterns, same as you would define them in your application.
    ///
    /// ```rust,ignore
    /// HttpServer::new(|| {
    ///     App::new()
    ///         .wrap(
    ///             actix_treblle::Treblle::new("project_id".to_string(), "api_key".to_string())
    ///                .add_ignored_routes(vec![
    ///                    "/users/{user_id}".to_string(),
    ///                    "/users/{user_id}".to_string(),
    ///                ])
    ///         )
    ///         .route("/hello", web::get().to(|| async { "Hello World!" }))
    /// })
    /// .bind(("127.0.0.1", 8080))?
    /// .run()
    /// .await
    /// ```
    pub fn add_ignored_routes(mut self, mut routes: Vec<String>) -> Treblle {
        self.ignored_routes.append(&mut routes);
        self
    }
}
