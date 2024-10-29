use hyper::{http, Response};
use hyper::header::{ACCESS_CONTROL_ALLOW_HEADERS, ACCESS_CONTROL_ALLOW_METHODS, ACCESS_CONTROL_ALLOW_ORIGIN};
use lazy_static::lazy_static;
use tokio::sync::RwLock;

lazy_static! {
    static ref CONFIG: RwLock<ResponseConfig> = RwLock::new(ResponseConfig::new());
}

#[derive(Clone)]
struct ResponseConfig {
    access_control_allow_origin: Option<String>,
    access_control_allow_methods: Option<String>,
    access_control_allow_headers: Option<String>
}

impl ResponseConfig {
    fn new() -> Self {
        Self {
            access_control_allow_origin: None,
            access_control_allow_methods: None,
            access_control_allow_headers: None
        }
    }

    pub async fn set(this: Self) {
        *CONFIG.write().await = this;
    }
}


pub struct ResponseBuilder;

impl ResponseBuilder {
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
