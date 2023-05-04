use hyper::server::conn::http1;
use hyper::{Request, Response};
// Import the routerify prelude traits.
use bytes::Bytes;
use http_body_util::Full;
use hyper::body::Incoming;
use routerify::{prelude::*, RequestServiceBuilder, Router};
use std::io;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tokio::net::TcpListener;

mod users {
    use super::*;

    struct State {
        count: Arc<Mutex<u8>>,
    }

    async fn list(req: Request<Incoming>) -> Result<Response<Full<Bytes>>, io::Error> {
        let count = req.data::<State>().unwrap().count.lock().unwrap();
        Ok(Response::new(Full::from(format!("Suppliers: {}", count))))
    }

    pub fn router() -> Router<Incoming, Full<Bytes>, io::Error> {
        let state = State {
            count: Arc::new(Mutex::new(20)),
        };
        Router::builder().data(state).get("/", list).build().unwrap()
    }
}

mod offers {
    use super::*;

    struct State {
        count: Arc<Mutex<u8>>,
    }

    async fn list(req: Request<Incoming>) -> Result<Response<Full<Bytes>>, io::Error> {
        let count = req.data::<State>().unwrap().count.lock().unwrap();

        println!("I can also access parent state: {:?}", req.data::<String>().unwrap());

        Ok(Response::new(Full::from(format!("Suppliers: {}", count))))
    }

    pub fn router() -> Router<Incoming, Full<Bytes>, io::Error> {
        let state = State {
            count: Arc::new(Mutex::new(100)),
        };
        Router::builder().data(state).get("/abc", list).build().unwrap()
    }
}

#[tokio::main]
async fn main() {
    let scopes = Router::builder()
        .data("Parent State data".to_owned())
        .scope("/offers", offers::router())
        .scope("/users", users::router())
        .build()
        .unwrap();

    let router = Router::builder().scope("/v1", scopes).build().unwrap();
    dbg!(&router);

    let builder = RequestServiceBuilder::new(router).unwrap();
    let addr = SocketAddr::from(([127, 0, 0, 1], 3001));
    let listener = TcpListener::bind(addr).await.unwrap();
    println!("App is running on: {}", listener.local_addr().unwrap());
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
