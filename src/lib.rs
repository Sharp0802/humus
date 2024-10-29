mod terminal;
mod route;
mod response;

use http_body_util::Full;
use hyper::body::{Bytes, Incoming};
use hyper::server::conn::http2;
use hyper::service::service_fn;
use hyper::{Request, Response, StatusCode};
use hyper_util::rt::TokioIo;

use std::convert::Infallible;
use std::error::Error;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use terminal::log;
use crate::response::ResponseBuilder;
use crate::route::{configure_all, match_route, shutdown_all, Route};

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


pub(crate) struct App {
    port: u16,
    shutdown_duration: Duration,
    root_route: Arc<dyn Route + Send + Sync>
}

impl App {

    pub(crate) fn new(port: u16, shutdown_duration: Duration, root_route: Arc<dyn Route + Send + Sync>) -> Self {
        Self {
            port,
            shutdown_duration,
            root_route
        }
    }

    async fn configure(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        configure_all(self.root_route.clone()).await
    }

    async fn map(&self, request: Request<Incoming>) -> Result<Response<Full<Bytes>>, Infallible> {
        let route = match match_route(request.uri().path(), self.root_route.clone()) {
            None => return Ok(ResponseBuilder::new()
                .status(StatusCode::NOT_FOUND)
                .body(Full::from(Bytes::new()))
                .unwrap()),
            Some(route) => route
        };

        match route.handle(request).await {
            Ok(response) => Ok(response),
            Err(error) => Ok(ResponseBuilder::new()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Full::from(error.to_string()))
                .unwrap())
        }
    }

    async fn shutdown(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        shutdown_all(self.root_route.clone()).await
    }

    pub(crate) async fn main(self: Arc<Self>) -> Result<(), Box<dyn Error + Send + Sync>> {

        self.configure().await?;

        let addr = SocketAddr::from(([127, 0, 0, 1], self.port));
        let listener = TcpListener::bind(addr).await?;

        let graceful = hyper_util::server::graceful::GracefulShutdown::new();
        let mut signal = std::pin::pin!(async {
            tokio::signal::ctrl_c().await.expect("failed to install CTRL+C signal handler");
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
        };
    }
}
