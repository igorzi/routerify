use bytes::Bytes;
use http_body_util::Full;
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::{Request, Response};
// Import the routerify prelude traits.
use routerify::{prelude::*, RequestServiceBuilder, Router};
use std::io;
use std::net::SocketAddr;
use tokio::net::TcpListener;

// A handler for "/" page.
async fn home_handler(_: Request<Incoming>) -> Result<Response<Full<Bytes>>, io::Error> {
    Ok(Response::new(Full::from("Home page")))
}

// Define a different module which will have only API related handlers.
mod api {
    use super::*;

    // Define a handler for "/users/:userName/books/:bookName" API endpoint which will have two
    // route parameters: `userName` and `bookName`.
    async fn user_book_handler(req: Request<Incoming>) -> Result<Response<Full<Bytes>>, io::Error> {
        let user_name = req.param("userName").unwrap();
        let book_name = req.param("bookName").unwrap();

        Ok(Response::new(Full::from(format!(
            "User: {}, Book: {}",
            user_name, book_name
        ))))
    }

    pub fn router() -> Router<Incoming, Full<Bytes>, io::Error> {
        // Create a router for API and specify the the handlers.
        Router::builder()
            .get("/users/:userName/books/:bookName", user_book_handler)
            .build()
            .unwrap()
    }
}

fn router() -> Router<Incoming, Full<Bytes>, io::Error> {
    // Create a root router and specify the the handlers.
    Router::builder()
        .get("/", home_handler)
        // Mount the api router at `/api` path.
        // Now the app can handle: `/api/users/:userName/books/:bookName` path.
        .scope("/api", api::router())
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
