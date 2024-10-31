#![warn(missing_docs)]

//!
//! # humus-terra
//!
//! humus-terra is an **intuitive** and **robust** framework for writing web-servers based on HTTP2.
//!
//! # Features
//!
//! - HTTP/2
//! - Asynchronous Design
//!

mod encrypt;
mod error;
pub mod response;
pub mod route;
pub mod terminal;
pub mod tokens;

use http_body_util::Full;
use hyper::body::{Bytes, Incoming};
use hyper::server::conn::http2;
use hyper::service::service_fn;
use hyper::{Request, Response, StatusCode};
use hyper_util::rt::TokioIo;

use crate::response::ResponseBuilder;
use crate::route::{configure_all, match_route, shutdown_all, Route};
use std::convert::Infallible;
use std::error::Error;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;

#[derive(Clone)]
struct TokioExecutor;

impl<F> hyper::rt::Executor<F> for TokioExecutor
where
    F: std::future::Future + Send + 'static,
    F::Output: Send + 'static,
{
    fn execute(&self, fut: F) {
        tokio::task::spawn(fut);
    }
}

/// An abstraction for hosting and routing.
pub struct App {
    port: u16,
    shutdown_duration: Duration,
    root_route: Arc<dyn Route + Send + Sync>,
}

impl App {
    ///
    /// Create new application with specified settings
    ///
    /// - *port*: Port to be used for hosting application
    /// - *shutdown_duration*: Timeout from `SIGINT` for finalising resources and connections
    /// - *root_route*: Implementation of root route
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use std::time::Duration;
    /// use humus_terra::App;
    ///
    /// let app = App::new(8080, Duration::from_secs(10), ...);
    /// ```
    ///
    pub fn new(
        port: u16,
        shutdown_duration: Duration,
        root_route: Arc<dyn Route + Send + Sync>,
    ) -> Self {
        Self {
            port,
            shutdown_duration,
            root_route,
        }
    }

    async fn configure(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        configure_all(self.root_route.clone()).await
    }

    async fn map(&self, request: Request<Incoming>) -> Result<Response<Full<Bytes>>, Infallible> {
        let route = match match_route(request.uri().path(), self.root_route.clone()) {
            None => {
                return Ok(ResponseBuilder::new()
                    .status(StatusCode::NOT_FOUND)
                    .body(Full::from(Bytes::new()))
                    .unwrap())
            }
            Some(route) => route,
        };

        match route.handle(request).await {
            Ok(response) => Ok(response),
            Err(error) => Ok(ResponseBuilder::new()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Full::from(error.to_string()))
                .unwrap()),
        }
    }

    async fn shutdown(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        shutdown_all(self.root_route.clone()).await
    }

    /// Run the configured application.
    ///
    /// This function executes the main loop of the application. It will block
    /// until the application is shutdown. If the application is triggered
    /// with `SIGINT`, it will exit the main loop and finalise resources.
    /// Instead of terminating the entire programme, the invocation of this
    /// function will simply return after finalisation.
    ///
    /// If the application fails to close all connections within the specified
    /// time limit, it will log a message but will not panic or forcibly shut
    /// down the system.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use std::sync::Arc;
    /// use std::time::Duration;
    /// use humus_terra::App;
    ///
    /// let app = App::new(8080, Duration::from_secs(10), ...);
    ///
    /// async move {
    ///     App::main(Arc::new(app)).await?;
    /// }
    /// ```
    ///
    pub async fn main(self: Arc<Self>) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.configure().await?;

        let addr = SocketAddr::from(([127, 0, 0, 1], self.port));
        let listener = TcpListener::bind(addr).await?;

        let graceful = hyper_util::server::graceful::GracefulShutdown::new();
        let mut signal = std::pin::pin!(async {
            tokio::signal::ctrl_c()
                .await
                .expect("failed to install CTRL+C signal handler");
        });

        loop {
            tokio::select! {
                Ok((stream, _)) = listener.accept() => {
                    let io = TokioIo::new(stream);
                    let app = self.clone();

                    tokio::task::spawn(async move {
                        if let Err(err) = http2::Builder::new(TokioExecutor)
                            .serve_connection(io, service_fn(move |req| {
                                let scoped_app = app.clone();
                                async move { scoped_app.clone().map(req).await }
                            }))
                            .await {
                            log!(fail "HTTP2 error: {}", err);
                        }
                    });
                },

                _ = &mut signal => {
                    log!(info "Shutting down...");
                    self.shutdown().await?;
                    break;
                }
            }
        }

        tokio::select! {
            _ = graceful.shutdown() => {
                log!(info "All connections gracefully closed");
            },
            _ = tokio::time::sleep(self.shutdown_duration) => {
                log!(info "Timed out waiting for connections");
            }
        }

        Ok(())
    }
}
