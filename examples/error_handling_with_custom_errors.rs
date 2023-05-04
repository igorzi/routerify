use bytes::Bytes;
use http_body_util::Full;
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::{Request, Response, StatusCode};
use routerify::{RequestServiceBuilder, Router};
use std::fmt;
use std::net::SocketAddr;
use tokio::net::TcpListener;

// Define a custom error enum to model a possible API service error.
#[derive(Debug)]
enum ApiError {
    #[allow(dead_code)]
    Unauthorized,
    Generic(String),
}

impl std::error::Error for ApiError {}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ApiError::Unauthorized => write!(f, "Unauthorized"),
            ApiError::Generic(s) => write!(f, "Generic: {}", s),
        }
    }
}

// Router, handlers and middleware must use the same error type.
// In this case it's `ApiError`.

// A handler for "/" page.
async fn home_handler(_: Request<Incoming>) -> Result<Response<Full<Bytes>>, ApiError> {
    // Simulate failure by returning `ApiError::Generic` variant.
    Err(ApiError::Generic("Something went wrong!".into()))
}

// Define an error handler function which will accept the `routerify::RouteError`
// and generates an appropriate response.
async fn error_handler(err: routerify::RouteError) -> Response<Full<Bytes>> {
    // Because `routerify::RouteError` is a boxed error, it must be
    // downcasted first. Unwrap for simplicity.
    let api_err = err.downcast::<ApiError>().unwrap();

    // Now that we've got the actual error, we can handle it
    // appropriately.
    match api_err.as_ref() {
        ApiError::Unauthorized => Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body(Full::default())
            .unwrap(),
        ApiError::Generic(s) => Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Full::from(s.to_string()))
            .unwrap(),
    }
}

fn router() -> Router<Incoming, Full<Bytes>, ApiError> {
    // Create a router and specify the the handlers.
    Router::builder()
        .get("/", home_handler)
        // Specify the error handler to handle any errors caused by
        // a route or any middleware.
        .err_handler(error_handler)
        .build()
        .unwrap()
}

#[tokio::main]
async fn main() {
    let router = router();

    // Create a Service builder from the router above to handle incoming requests.
    let builder = RequestServiceBuilder::new(router).unwrap();

    // The address on which the server will be listening.
    let addr = SocketAddr::from(([127, 0, 0, 1], 3001));

    // Create a TcpListener and bind it to the address.
    let listener = TcpListener::bind(addr).await.unwrap();

    // Start a loop to continuously accept incoming connections.
    loop {
        let (stream, _) = listener.accept().await.unwrap();
        let service = builder.build();
        tokio::task::spawn(async move {
            if let Err(err) = http1::Builder::new().serve_connection(stream, service).await {
                println!("Failed to serve connection: {:?}", err);
            }
        });
    }
}
