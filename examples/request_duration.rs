use bytes::Bytes;
use http_body_util::Full;
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::{Request, Response};
// Import the routerify prelude traits.
use routerify::{prelude::*, RequestServiceBuilder};
use routerify::{Middleware, RequestInfo, Router};
use std::convert::Infallible;
use std::net::SocketAddr;
use tokio::net::TcpListener;

async fn before(req: Request<Incoming>) -> Result<Request<Incoming>, Infallible> {
    req.set_context(tokio::time::Instant::now());
    Ok(req)
}

async fn hello(_: Request<Incoming>) -> Result<Response<Full<Bytes>>, Infallible> {
    Ok(Response::new(Full::from("Home page")))
}

async fn after(res: Response<Full<Bytes>>, req_info: RequestInfo) -> Result<Response<Full<Bytes>>, Infallible> {
    let started = req_info.context::<tokio::time::Instant>().unwrap();
    let duration = started.elapsed();
    println!("duration {:?}", duration);
    Ok(res)
}

fn router() -> Router<Incoming, Full<Bytes>, Infallible> {
    Router::builder()
        .get("/", hello)
        .middleware(Middleware::pre(before))
        .middleware(Middleware::post_with_info(after))
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
