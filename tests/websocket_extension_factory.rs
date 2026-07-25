use std::sync::{
	atomic::{AtomicUsize, Ordering},
	Arc,
};

use jsonrpsee_server::{
	middleware::rpc::RpcServiceBuilder, stop_channel, ws, ConnectionGuard, ConnectionState, HttpBody, HttpRequest,
	Methods, Server, ServerConfig, WsExtensionFactory,
};
use soketto::{
	base::Header,
	extension::{Extension, Param},
	BoxedError, Storage,
};
use tower::Service;

#[derive(Debug)]
struct TestExtension {
	enabled: bool,
}

impl Extension for TestExtension {
	fn is_enabled(&self) -> bool {
		self.enabled
	}

	fn name(&self) -> &str {
		"test-extension"
	}

	fn params(&self) -> &[Param<'_>] {
		&[]
	}

	fn configure(&mut self, params: &[Param<'_>]) -> Result<(), BoxedError> {
		self.enabled = params.is_empty();
		Ok(())
	}

	fn encode(&mut self, _header: &mut Header, _data: &mut Storage<'_>) -> Result<(), BoxedError> {
		Ok(())
	}

	fn decode(&mut self, _header: &mut Header, _data: &mut Vec<u8>) -> Result<(), BoxedError> {
		Ok(())
	}
}

#[derive(Clone, Debug)]
struct CountingFactory(Arc<AtomicUsize>);

impl WsExtensionFactory for CountingFactory {
	fn create(&self) -> Vec<Box<dyn Extension + Send>> {
		self.0.fetch_add(1, Ordering::SeqCst);
		vec![Box::new(TestExtension { enabled: false })]
	}
}

fn upgrade_request() -> HttpRequest {
	HttpRequest::builder()
		.header("host", "localhost")
		.header("connection", "upgrade")
		.header("upgrade", "websocket")
		.header("sec-websocket-key", "dGhlIHNhbXBsZSBub25jZQ==")
		.header("sec-websocket-version", "13")
		.header("sec-websocket-extensions", "test-extension")
		.body(HttpBody::empty())
		.unwrap()
}

fn http_request() -> HttpRequest {
	HttpRequest::builder()
		.method("POST")
		.header("host", "localhost")
		.header("content-type", "application/json")
		.body(HttpBody::from(r#"{"jsonrpc":"2.0","id":1,"method":"missing","params":[]}"#))
		.unwrap()
}

#[tokio::test]
async fn tower_service_instantiates_extensions_per_connection() {
	let calls = Arc::new(AtomicUsize::new(0));
	let (stop_handle, _server_handle) = stop_channel();
	let mut service = Server::builder()
		.set_ws_extension_factory(CountingFactory(calls.clone()))
		.to_service_builder()
		.build(Methods::new(), stop_handle);

	for expected_calls in 1..=2 {
		let response = service.call(upgrade_request()).await.unwrap();
		assert_eq!(response.status(), http::StatusCode::SWITCHING_PROTOCOLS);
		assert_eq!(response.headers()["sec-websocket-extensions"], "test-extension");
		assert_eq!(calls.load(Ordering::SeqCst), expected_calls);
	}
}

#[tokio::test]
async fn tower_service_skips_extension_factory_for_http_requests() {
	let calls = Arc::new(AtomicUsize::new(0));
	let (stop_handle, _server_handle) = stop_channel();
	let mut service = Server::builder()
		.set_ws_extension_factory(CountingFactory(calls.clone()))
		.to_service_builder()
		.build(Methods::new(), stop_handle);

	let response = service.call(http_request()).await.unwrap();

	assert_eq!(response.status(), http::StatusCode::OK);
	assert_eq!(calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn low_level_connect_instantiates_extensions_per_connection() {
	let calls = Arc::new(AtomicUsize::new(0));
	let server_config = ServerConfig::default().set_ws_extension_factory(CountingFactory(calls.clone()));
	let guard = ConnectionGuard::new(1);
	let permit = guard.try_acquire().unwrap();
	let (stop_handle, _server_handle) = stop_channel();
	let connection = ConnectionState::new(stop_handle, 0, permit);

	let (response, _connection) =
		ws::connect(upgrade_request(), server_config, Methods::new(), connection, RpcServiceBuilder::new())
			.await
			.unwrap();

	assert_eq!(response.status(), http::StatusCode::SWITCHING_PROTOCOLS);
	assert_eq!(response.headers()["sec-websocket-extensions"], "test-extension");
	assert_eq!(calls.load(Ordering::SeqCst), 1);
}
