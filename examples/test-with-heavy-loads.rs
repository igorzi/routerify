use bytes::Bytes;
use http_body_util::Full;
use hyper::{body::Incoming, server::conn::http1, Response};
use routerify::{Middleware, RequestServiceBuilder, Router};
use std::net::SocketAddr;
use tokio::net::TcpListener;

fn router() -> Router<Incoming, Full<Bytes>, routerify::Error> {
    let mut builder = Router::builder();

    for i in 0..3000_usize {
        builder = builder.middleware(
            Middleware::pre_with_path(format!("/abc-{}", i), move |req| async move {
                // println!("PreMiddleware: {}", format!("/abc-{}", i));
                Ok(req)
            })
            .unwrap(),
        );

        builder = builder.get(format!("/abc-{}", i), move |_req| async move {
            // println!("Route: {}, params: {:?}", format!("/abc-{}", i), req.params());
            Ok(Response::new(Full::from(format!("/abc-{}", i))))
        });

        builder = builder.middleware(
            Middleware::post_with_path(format!("/abc-{}", i), move |res| async move {
                // println!("PostMiddleware: {}", format!("/abc-{}", i));
                Ok(res)
            })
            .unwrap(),
        );
    }

    builder.build().unwrap()
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
