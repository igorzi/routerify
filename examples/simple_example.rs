use bytes::Bytes;
use http_body_util::Full;
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::{Request, Response};
// Import the routerify prelude traits.
use routerify::{prelude::*, RequestServiceBuilder};
use routerify::{Middleware, Router};
use std::{convert::Infallible, net::SocketAddr};
use tokio::net::TcpListener;

// A handler for "/" page.
async fn home_handler(_: Request<Incoming>) -> Result<Response<Full<Bytes>>, Infallible> {
    Ok(Response::new(Full::from("Home page")))
}

// A handler for "/about" page.
async fn about_handler(_: Request<Incoming>) -> Result<Response<Full<Bytes>>, Infallible> {
    Ok(Response::new(Full::from("About page")))
}

// A middleware which logs an http request.
async fn logger(req: Request<Incoming>) -> Result<Request<Incoming>, Infallible> {
    println!("{} {} {}", req.remote_addr(), req.method(), req.uri().path());
    Ok(req)
}

fn router() -> Router<Incoming, Full<Bytes>, Infallible> {
    // Create a router and specify the logger middleware and the handlers.
    // Here, "Middleware::pre" means we're adding a pre middleware which will be executed
    // before any route handlers.
    Router::builder()
        .middleware(Middleware::pre(logger))
        .get("/", home_handler)
        .get("/about", about_handler)
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
