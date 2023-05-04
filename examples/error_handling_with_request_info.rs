use bytes::Bytes;
use http_body_util::Full;
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::{header, Request, Response, StatusCode};
use routerify::{RequestInfo, RequestServiceBuilder, Router};
use std::io;
use std::net::SocketAddr;
use tokio::net::TcpListener;

// A handler for "/" page.
async fn home_handler(_: Request<Incoming>) -> Result<Response<Full<Bytes>>, io::Error> {
    Err(io::Error::new(io::ErrorKind::Other, "Some errors"))
}

// A handler for "/about" page.
async fn about_handler(_: Request<Incoming>) -> Result<Response<Full<Bytes>>, io::Error> {
    Ok(Response::new(Full::from("About page")))
}

// Define an error handler function which will accept the `routerify::Error` and the `req_info`
// and generates an appropriate response.
async fn error_handler(err: routerify::RouteError, req_info: RequestInfo) -> Response<Full<Bytes>> {
    eprintln!("{}", err);

    // Access a cookie.
    let cookie = req_info.headers().get(header::COOKIE).unwrap().to_str().unwrap();

    Response::builder()
        .header(header::SET_COOKIE, cookie)
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .body(Full::from(format!("Something went wrong: {}", err)))
        .unwrap()
}

fn router() -> Router<Incoming, Full<Bytes>, io::Error> {
    // Create a router and specify the the handlers.
    Router::builder()
        .get("/", home_handler)
        .get("/about", about_handler)
        // Specify the error handler to handle any errors caused by
        // a route or any middleware.
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
