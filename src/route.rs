//!
//! A module that provides abstraction for routes and its helpers
//!

use std::sync::Arc;
use async_trait::async_trait;
use http_body_util::Full;
use hyper::body::{Bytes, Incoming};
use hyper::{Request, Response};

type Error = dyn std::error::Error + Send + Sync;

/// An abstraction for routes
#[async_trait]
pub trait Route {
    /// Get name of route
    fn name(&self) -> &str;

    /// Get children of route
    fn children(&self) -> Vec<Arc<dyn Route + Send + Sync>> { vec![] }

    /// Configure route and Initialise its resources
    async fn configure(&self) -> Result<(), Box<Error>> { Ok(()) }

    /// Shutdown route and Finalise its resources
    async fn shutdown(&self) -> Result<(), Box<Error>> { Ok(()) }

    /// Handle request asynchronously
    async fn handle(&self, request: Request<Incoming>) -> Result<Response<Full<Bytes>>, Box<Error>>;
}

pub(crate) async fn configure_all(root: Arc<dyn Route + Send + Sync>) -> Result<(), Box<Error>> {

    root.configure().await?;
    for route in root.children() {
        route.configure().await?;
    }

    Ok(())
}

pub(crate) async fn shutdown_all(root: Arc<dyn Route + Send + Sync>) -> Result<(), Box<Error>> {

    root.shutdown().await?;
    for route in root.children() {
        route.shutdown().await?;
    }

    Ok(())
}

pub(crate) fn match_route(path: &str, root: Arc<dyn Route + Send + Sync>) -> Option<Arc<dyn Route + Send + Sync>> {

    let parts = path.split("/").skip(1).collect::<Vec<&str>>();

    let mut current = root;
    for part in parts {
        if part.len() == 0 {
            continue;
        }

        let mut found = false;
        for route in current.children() {
            if route.name() == part {
                current = route;
                found = true;
                break;
            }
        }

        if !found {
            return None
        }
    }

    Some(current)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct RootRoute {
        route_a: Arc<ARoute>
    }

    struct ARoute {
        route_b: Arc<BRoute>
    }

    struct BRoute;

    impl RootRoute {
        fn new() -> Self {
            Self { route_a: Arc::new(ARoute::new()) }
        }
    }

    impl ARoute {
        fn new() -> Self {
            Self { route_b: Arc::new(BRoute::new()) }
        }
    }

    impl BRoute {
        fn new() -> Self {
            Self {}
        }
    }

    #[async_trait]
    impl Route for RootRoute {
        fn name(&self) -> &str { "" }

        fn children(&self) -> Vec<Arc<dyn Route + Send + Sync>> {
            vec![ self.route_a.clone() ]
        }

        async fn handle(&self, _request: Request<Incoming>) -> Result<Response<Full<Bytes>>, Box<Error>> {
            panic!()
        }
    }

    #[async_trait]
    impl Route for ARoute {
        fn name(&self) -> &str { "a" }

        fn children(&self) -> Vec<Arc<dyn Route + Send + Sync>> {
            vec![ self.route_b.clone() ]
        }

        async fn handle(&self, _request: Request<Incoming>) -> Result<Response<Full<Bytes>>, Box<Error>> {
            panic!()
        }
    }

    #[async_trait]
    impl Route for BRoute {
        fn name(&self) -> &str { "b" }

        async fn handle(&self, _request: Request<Incoming>) -> Result<Response<Full<Bytes>>, Box<Error>> {
            panic!()
        }
    }

    #[test]
    fn route_root() {
        let root = Arc::new(RootRoute::new());
        match match_route("/", root) {
            None => panic!("Couldn't find route for '/'"),
            Some(route) => assert_eq!(route.name(), "")
        };
    }

    #[test]
    fn route_a() {
        let root = Arc::new(RootRoute::new());
        match match_route("/a/", root) {
            None => panic!("Couldn't find route for '/a/'"),
            Some(route) => assert_eq!(route.name(), "a")
        };
    }

    #[test]
    fn route_b() {
        let root = Arc::new(RootRoute::new());
        match match_route("/a/b", root) {
            None => panic!("Couldn't find route for '/b'"),
            Some(route) => assert_eq!(route.name(), "b")
        };
    }
}
