//!
//! A module that provides abstraction and management of responses
//!

use hyper::header::{
    ACCESS_CONTROL_ALLOW_HEADERS, ACCESS_CONTROL_ALLOW_METHODS, ACCESS_CONTROL_ALLOW_ORIGIN,
};
use hyper::{http, Response};
use lazy_static::lazy_static;
use tokio::sync::RwLock;

lazy_static! {
    static ref CONFIG: RwLock<ResponseConfig> = RwLock::new(ResponseConfig::new());
}

///
/// A configuration struct for creating responses
///
#[derive(Clone)]
pub struct ResponseConfig {
    /// [Access-Control-Allow-Origin](https://fetch.spec.whatwg.org/#http-access-control-allow-origin)
    pub access_control_allow_origin: Option<String>,

    /// [Access-Control-Allow-Methods](https://fetch.spec.whatwg.org/#http-access-control-allow-methods)
    pub access_control_allow_methods: Option<String>,

    /// [Access-Control-Allow-Headers](https://fetch.spec.whatwg.org/#http-access-control-allow-headers)
    pub access_control_allow_headers: Option<String>,
}

impl ResponseConfig {
    ///
    /// Create new configuration for responses
    ///
    pub fn new() -> Self {
        Self {
            access_control_allow_origin: None,
            access_control_allow_methods: None,
            access_control_allow_headers: None,
        }
    }

    ///
    /// Override configuration by given argument
    ///
    pub async fn set(this: Self) {
        *CONFIG.write().await = this;
    }
}

///
/// An abstraction over response-builder in hyper to apply options consistently
///
pub struct ResponseBuilder;

impl ResponseBuilder {
    ///
    /// Create new builder for response with stored options.
    /// For options, See [ResponseConfig]
    ///
    pub fn new() -> http::response::Builder {
        let mut builder = Response::builder();

        let config = CONFIG.blocking_read().clone();

        if let Some(cors_origin) = config.access_control_allow_origin.as_ref() {
            builder = builder.header(ACCESS_CONTROL_ALLOW_ORIGIN, cors_origin);
        }
        if let Some(cors_methods) = config.access_control_allow_methods.as_ref() {
            builder = builder.header(ACCESS_CONTROL_ALLOW_METHODS, cors_methods);
        }
        if let Some(cors_headers) = config.access_control_allow_headers.as_ref() {
            builder = builder.header(ACCESS_CONTROL_ALLOW_HEADERS, cors_headers);
        }

        builder
    }
}
