use std::net::SocketAddr;

use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{body::Incoming, Request, Response};
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;
use tokio::sync::broadcast;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum DaemonError {
	#[error("IO error: {0}")]
	Io(#[from] std::io::Error),
	#[error("Address parse error: {0}")]
	AddrParse(#[from] std::net::AddrParseError),
}

pub struct HalSimplicityDaemon {
	address: SocketAddr,
	shutdown_tx: broadcast::Sender<()>,
}

impl HalSimplicityDaemon {
	pub fn new(address: &str) -> Result<Self, DaemonError> {
		let address: SocketAddr = address.parse()?;
		let (shutdown_tx, _) = broadcast::channel(1);

		Ok(Self {
			address,
			shutdown_tx,
		})
	}

	pub fn start(&mut self) -> Result<(), DaemonError> {
		let address = self.address.clone();
		let shutdown_tx = self.shutdown_tx.clone();

		let runtime = tokio::runtime::Runtime::new()?;

		let listener = runtime.block_on(async { TcpListener::bind(address).await })?;

		std::thread::spawn(move || {
			runtime.block_on(async move {
				println!("Listening on http://{}", address);

				let mut shutdown_rx = shutdown_tx.subscribe();

				loop {
					tokio::select! {
						Ok((stream, _)) = listener.accept() => {
							let io = TokioIo::new(stream);
							tokio::task::spawn(async move {
								if let Err(err) = http1::Builder::new()
									.serve_connection(io, service_fn(handle_request))
									.await
								{
									eprintln!("Connection error: {:?}", err);
								}
							});
						}
						_ = shutdown_rx.recv() => {
							println!("Server shutting down...");
							break;
						}
					}
				}
			});
		});

		Ok(())
	}

	pub fn shutdown(&self) {
		let _ = self.shutdown_tx.send(());
	}
}
async fn handle_request(req: Request<Incoming>) -> Result<Response<String>, DaemonError> {
	let path = req.uri().path();
	let method = req.method();

	println!("Received {} request for {}", method, path);

	Ok(Response::new("Acknowledged".to_string()))
}
