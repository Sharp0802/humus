mod terminal;
mod route;

use http_body_util::Full;
use hyper::body::{Bytes, Incoming};
use hyper::server::conn::http2;
use hyper::service::service_fn;
use hyper::{Request, Response};
use hyper_util::rt::TokioIo;
use std::convert::Infallible;
use std::error::Error;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::TcpListener;
use terminal::log;


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


pub struct App {
    port: u16,
    shutdown_duration: Duration
}

impl App {

    pub fn new(port: u16, shutdown_duration: Duration) -> Self {
        Self {
            port,
            shutdown_duration
        }
    }

    async fn configure() -> Result<(), Box<dyn Error + Send + Sync>> {
        todo!()
    }

    async fn map(request: Request<Incoming>) -> Result<Response<Full<Bytes>>, Infallible> {
        todo!()
    }

    async fn shutdown() -> Result<(), Box<dyn Error + Send + Sync>> {
        todo!()
    }

    pub async fn main(&self) -> Result<(), Box<dyn Error + Send + Sync>> {

        Self::configure().await?;

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

                    tokio::task::spawn(async move {
                        if let Err(err) = http2::Builder::new(TokioExecutor)
                            .serve_connection(io, service_fn(Self::map))
                            .await {
                            log!(fail "HTTP2 error: {}", err);
                        }
                    });
                },

                _ = &mut signal => {
                    log!(info "Shutting down...");
                    Self::shutdown().await?;
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
