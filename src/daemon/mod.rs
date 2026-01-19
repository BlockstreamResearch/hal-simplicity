pub mod handler;
pub mod types;

pub mod jsonrpc;

use std::net::SocketAddr;
use std::sync::Arc;

use http_body_util::{BodyExt, Full};
use hyper::body::Bytes;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{body::Incoming, Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;
use tokio::sync::broadcast;

use thiserror::Error;

use handler::DefaultRpcHandler;
use jsonrpc::JsonRpcService;

/// Errors that can occur in the daemon, usually on startup.
#[derive(Error, Debug)]
pub enum DaemonError {
	#[error("IO error: {0}")]
	Io(#[from] std::io::Error),
	#[error("Address parse error: {0}")]
	AddrParse(#[from] std::net::AddrParseError),
}

/// The HAL Simplicity Daemon
///
/// It listens for JSON-RPC requests over HTTP and handles them.
/// Does not block the current thread when started. Instead, it spawns a new thread.
pub struct HalSimplicityDaemon {
	address: SocketAddr,
	shutdown_tx: broadcast::Sender<()>,
	rpc_service: Arc<JsonRpcService<DefaultRpcHandler>>,
}

impl HalSimplicityDaemon {
	pub fn new(address: &str) -> Result<Self, DaemonError> {
		let address: SocketAddr = address.parse()?;
		let (shutdown_tx, _) = broadcast::channel(1);
		let rpc_service = Arc::new(handler::create_service());

		Ok(Self {
			address,
			shutdown_tx,
			rpc_service,
		})
	}

	/// Core event loop that accepts connections and handles them
	async fn run_event_loop(
		listener: TcpListener,
		rpc_service: Arc<JsonRpcService<DefaultRpcHandler>>,
		mut shutdown_rx: broadcast::Receiver<()>,
	) -> Result<(), DaemonError> {
		loop {
			tokio::select! {
				Ok((stream, _)) = listener.accept() => {
					let io = TokioIo::new(stream);
					let rpc_service_clone = rpc_service.clone();
					tokio::task::spawn(async move {
						http1::Builder::new()
							.serve_connection(io, service_fn(move |req| {
								handle_request(req, rpc_service_clone.clone())
							}))
							.await
					});
				}
				_ = shutdown_rx.recv() => {
					break;
				}
			}
		}

		Ok(())
	}

	/// Start the daemon on a new thread.
	/// Useful when you need just to spawn the daemon and continue doing other things in the main thread.
	pub fn start(&mut self) -> Result<(), DaemonError> {
		let address = self.address;
		let shutdown_tx = self.shutdown_tx.clone();
		let rpc_service = self.rpc_service.clone();

		let runtime = tokio::runtime::Runtime::new()?;
		let listener = runtime.block_on(async { TcpListener::bind(&address).await })?;

		std::thread::spawn(move || {
			runtime.block_on(async move {
				let shutdown_rx = shutdown_tx.subscribe();
				let _ = Self::run_event_loop(listener, rpc_service, shutdown_rx).await;
			});
		});

		Ok(())
	}

	/// Start the daemon and block the current thread,
	/// Useful for CLI applications.
	pub fn listen_blocking(self) -> Result<(), DaemonError> {
		let runtime = tokio::runtime::Runtime::new()?;

		runtime.block_on(async move {
			let listener = TcpListener::bind(&self.address).await?;
			let shutdown_rx = self.shutdown_tx.subscribe();
			Self::run_event_loop(listener, self.rpc_service, shutdown_rx).await
		})
	}

	/// Shutdown the daemon
	pub fn shutdown(&self) {
		let _ = self.shutdown_tx.send(());
	}
}

/// Handles an incoming HTTP request and produces a response.
async fn handle_request(
	req: Request<Incoming>,
	rpc_service: Arc<JsonRpcService<DefaultRpcHandler>>,
) -> Result<Response<Full<Bytes>>, DaemonError> {
	let path = req.uri().path();
	let method = req.method();

	if method != Method::POST {
		return Ok(create_status_response(StatusCode::METHOD_NOT_ALLOWED));
	}

	if path != "/rpc" && path != "/" {
		return Ok(create_status_response(StatusCode::NOT_FOUND));
	}

	let body_str = match read_body_as_string(req).await {
		Ok(body) => body,
		Err(status) => return Ok(create_status_response(status)),
	};

	let response_str = rpc_service.handle_raw(&body_str);

	if response_str.is_empty() {
		return Ok(create_status_response(StatusCode::NO_CONTENT));
	}

	Ok(create_json_response(response_str))
}

/// Creates an HTTP response with the given status code
fn create_status_response(status: StatusCode) -> Response<Full<Bytes>> {
	let body = if status == StatusCode::NO_CONTENT {
		Bytes::new()
	} else {
		Bytes::from(status.canonical_reason().unwrap_or("Unknown Error"))
	};
	let mut response = Response::new(Full::new(body));
	*response.status_mut() = status;
	response
}

/// Reads and validates the request body as a UTF-8 string
async fn read_body_as_string(req: Request<Incoming>) -> Result<String, StatusCode> {
	let body_bytes = req.collect().await.map_err(|_| StatusCode::BAD_REQUEST)?.to_bytes();

	String::from_utf8(body_bytes.to_vec()).map_err(|_| StatusCode::BAD_REQUEST)
}

/// Creates a successful JSON-RPC response
fn create_json_response(body: String) -> Response<Full<Bytes>> {
	let mut response = Response::new(Full::new(Bytes::from(body)));
	response.headers_mut().insert(
		hyper::header::CONTENT_TYPE,
		hyper::header::HeaderValue::from_static("application/json"),
	);
	response
}
