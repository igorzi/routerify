use http_body_util::BodyExt;
use hyper::body::Buf;
use hyper::body::Incoming;
use hyper::{body::Body, server::conn::http1};
use routerify::Router;
use std::io;
use std::net::SocketAddr;
use tokio::sync::oneshot::{self, Sender};

pub struct Serve {
    addr: SocketAddr,
    tx: Sender<()>,
}

impl Serve {
    pub fn addr(&self) -> &SocketAddr {
        &self.addr
    }

    pub fn new_request(&self, method: &str, route: &str) -> http::request::Builder {
        http::request::Request::builder()
            .method(method.to_ascii_uppercase().as_str())
            .uri(format!("http://{}{}", self.addr(), route))
    }

    pub fn shutdown(self) {
        self.tx.send(()).unwrap();
    }
}

pub async fn serve<ResponseBody, E>(router: Router<Incoming, ResponseBody, E>) -> Serve
where
    ResponseBody: Body + Send + Sync + 'static,
    E: Into<Box<dyn std::error::Error + Send + Sync>> + 'static,
    <ResponseBody as Body>::Data: Send + Sync + 'static,
    <ResponseBody as Body>::Error: Into<Box<dyn std::error::Error + Send + Sync>> + 'static,
{
    let addr: SocketAddr = ([127, 0, 0, 1], 0).into();
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    let addr = listener.local_addr().unwrap();
    let builder = routerify::RequestServiceBuilder::new(router).unwrap();
    let (tx, mut rx) = oneshot::channel::<()>();

    tokio::task::spawn(async move {
        loop {
            tokio::select! {
                res = listener.accept() => {
                    let (stream, _) = res.unwrap();
                    let service = builder.build();
                    tokio::task::spawn(async move {
                        http1::Builder::new().serve_connection(stream, service).await.unwrap();
                    });
                }
                _ = &mut rx => {
                    break;
                }
            }
        }
    });
    Serve { addr, tx }
}

pub async fn into_text(body: Incoming) -> String {
    let body = body.collect().await.unwrap().aggregate();
    io::read_to_string(body.reader()).unwrap()
}
