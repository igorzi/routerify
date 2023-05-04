use bytes::Bytes;
use http_body_util::Full;
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::{Request, Response, StatusCode};
// Import the routerify prelude traits.
use routerify::{prelude::*, Middleware, RequestInfo, RequestServiceBuilder, Router};
use std::{convert::Infallible, net::SocketAddr};
use tokio::net::TcpListener;

// Define an app state to share it across the route handlers, middlewares
// and the error handler.
struct State(u64);

// A handler for "/" page.
async fn home_handler(req: Request<Incoming>) -> Result<Response<Full<Bytes>>, Infallible> {
    // Access the app state.
    let state = req.data::<State>().unwrap();
    println!("State value: {}", state.0);

    Ok(Response::new(Full::from("Home page")))
}

// A middleware which logs an http request.
async fn logger(req: Request<Incoming>) -> Result<Request<Incoming>, Infallible> {
    // You can also access the same state from middleware.
    let state = req.data::<State>().unwrap();
    println!("State value: {}", state.0);

    println!("{} {} {}", req.remote_addr(), req.method(), req.uri().path());
    Ok(req)
}

// Define an error handler function which will accept the `routerify::Error`
// and the request information and generates an appropriate response.
async fn error_handler(err: routerify::RouteError, req_info: RequestInfo) -> Response<Full<Bytes>> {
    // You can also access the same state from error handler.
    let state = req_info.data::<State>().unwrap();
    println!("State value: {}", state.0);

    eprintln!("{}", err);
    Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .body(Full::from(format!("Something went wrong: {}", err)))
        .unwrap()
}

// Create a `Router<Body, Infallible>` for response body type `hyper::Body`
// and for handler error type `Infallible`.
fn router() -> Router<Incoming, Full<Bytes>, Infallible> {
    Router::builder()
        // Specify the state data which will be available to every route handlers,
        // error handler and middlewares.
        .data(State(100))
        .middleware(Middleware::pre(logger))
        .get("/", home_handler)
        .err_handler_with_info(error_handler)
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
