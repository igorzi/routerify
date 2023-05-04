use bytes::Bytes;
use http_body_util::Full;
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::{Request, Response, StatusCode};
use routerify::{prelude::*, Middleware, RequestInfo, RequestServiceBuilder, Router};
use std::net::SocketAddr;
use tokio::net::TcpListener;

pub struct State(pub i32);

pub async fn pre_middleware(req: Request<Incoming>) -> Result<Request<Incoming>, routerify::Error> {
    let data = req.data::<State>().map(|s| s.0).unwrap_or(0);
    println!("Pre Data: {}", data);
    println!("Pre Data2: {:?}", req.data::<u32>());

    Ok(req)
}

pub async fn post_middleware(
    res: Response<Full<Bytes>>,
    req_info: RequestInfo,
) -> Result<Response<Full<Bytes>>, routerify::Error> {
    let data = req_info.data::<State>().map(|s| s.0).unwrap_or(0);
    println!("Post Data: {}", data);

    Ok(res)
}

pub async fn home_handler(req: Request<Incoming>) -> Result<Response<Full<Bytes>>, routerify::Error> {
    let data = req.data::<State>().map(|s| s.0).unwrap_or(0);
    println!("Route Data: {}", data);
    println!("Route Data2: {:?}", req.data::<u32>());

    Err(routerify::Error::new("Error"))
}

async fn error_handler(err: routerify::RouteError, req_info: RequestInfo) -> Response<Full<Bytes>> {
    let data = req_info.data::<State>().map(|s| s.0).unwrap_or(0);
    println!("Error Data: {}", data);
    println!("Error Data2: {:?}", req_info.data::<u32>());

    eprintln!("{}", err);
    Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .body(Full::from(format!("Something went wrong: {}", err)))
        .unwrap()
}

fn router2() -> Router<Incoming, Full<Bytes>, routerify::Error> {
    Router::builder()
        .data(111_u32)
        .get("/a", |req| async move {
            println!("Router2 Data: {:?}", req.data::<&str>());
            println!("Router2 Data: {:?}", req.data::<State>().map(|s| s.0));
            println!("Router2 Data: {:?}", req.data::<u32>());
            Ok(Response::new(Full::from("Hello world!")))
        })
        .build()
        .unwrap()
}

fn router3() -> Router<Incoming, Full<Bytes>, routerify::Error> {
    Router::builder()
        .data(555_u32)
        .get("/h/g/j", |req| async move {
            println!("Router3 Data: {:?}", req.data::<&str>());
            println!("Router3 Data: {:?}", req.data::<State>().map(|s| s.0));
            println!("Router3 Data: {:?}", req.data::<u32>());
            Ok(Response::new(Full::from("Hello world!")))
        })
        .build()
        .unwrap()
}

#[tokio::main]
async fn main() {
    let router: Router<Incoming, Full<Bytes>, routerify::Error> = Router::builder()
        .data(State(100))
        .scope("/r", router2())
        .scope("/bcd", router3())
        .data("abcd")
        .middleware(Middleware::pre(pre_middleware))
        .middleware(Middleware::post_with_info(post_middleware))
        .get("/", home_handler)
        .err_handler_with_info(error_handler)
        .build()
        .unwrap();

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
