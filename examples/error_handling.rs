use bytes::Bytes;
use http_body_util::Full;
use hyper::{body::Incoming, server::conn::http1, Request, Response, StatusCode};
use routerify::{RequestServiceBuilder, Router};
use std::net::SocketAddr;
use tokio::net::TcpListener;

// A handler for "/" page.
async fn home_handler(_: Request<Incoming>) -> Result<Response<Full<Bytes>>, routerify::Error> {
    Err(routerify::Error::new("Some errors"))
}

// A handler for "/about" page.
async fn about_handler(_: Request<Incoming>) -> Result<Response<Full<Bytes>>, routerify::Error> {
    Ok(Response::new(Full::from("About page")))
}

// Define an error handler function which will accept the `routerify::RouteError`
// and generates an appropriate response.
async fn error_handler(err: routerify::RouteError) -> Response<Full<Bytes>> {
    Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .body(Full::from(err.to_string()))
        .unwrap()
}

fn router() -> Router<Incoming, Full<Bytes>, routerify::Error> {
    // Create a router and specify the the handlers.
    Router::builder()
        .get("/", home_handler)
        .get("/about", about_handler)
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
