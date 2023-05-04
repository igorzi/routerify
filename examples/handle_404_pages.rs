use bytes::Bytes;
use http_body_util::Full;
use hyper::{body::Incoming, server::conn::http1, Request, Response, StatusCode};
use routerify::{RequestServiceBuilder, Router};
use std::io;
use std::net::SocketAddr;
use tokio::net::TcpListener;

// A handler for "/" page.
async fn home_handler(_: Request<Incoming>) -> Result<Response<Full<Bytes>>, io::Error> {
    Ok(Response::new(Full::from("Home page")))
}

// A handler for "/about" page.
async fn about_handler(_: Request<Incoming>) -> Result<Response<Full<Bytes>>, io::Error> {
    Ok(Response::new(Full::from("About page")))
}

// Define a handler to handle any non-existent routes i.e. a 404 handler.
async fn handler_404(_: Request<Incoming>) -> Result<Response<Full<Bytes>>, io::Error> {
    Ok(Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(Full::from("Page Not Found"))
        .unwrap())
}

fn router() -> Router<Incoming, Full<Bytes>, io::Error> {
    // Create a router and specify the the handlers.
    Router::builder()
        .get("/", home_handler)
        .get("/about", about_handler)
        // Add a route to handle 404 routes.
        .any(handler_404)
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
