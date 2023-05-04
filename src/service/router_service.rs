use crate::router::Router;
use crate::service::request_service::RequestServiceBuilder;
use futures::Future;
use http::Request;
use hyper::{body::Body, service::Service};
use std::pin::Pin;
use std::sync::Arc;

/// A [`Service`](https://docs.rs/hyper/0.14.4/hyper/service/trait.Service.html) to process incoming requests.
///
/// This `RouterService<RequestBody, ResponseBody, E>` type accepts two type parameters: `B` and `E`.
///
/// * The `RequestBody` represents the response body type which will be used by route handlers and the middlewares and this body type must implement
///   the [hyper::body::Body](https://docs.rs/hyper/1.0.0-rc.3/hyper/body/trait.Body.html) trait. For an instance, `RequestBody` could be [hyper::body::Incoming](https://docs.rs/hyper/1.0.0-rc.3/hyper/body/struct.Incoming.html)
///   type.
/// * The `ResponseBody` represents the response body type which will be used by route handlers and the middlewares and this body type must implement
///   the [hyper::body::Body](https://docs.rs/hyper/1.0.0-rc.3/hyper/body/trait.Body.html) trait. For an instance, `ResponseBody` could be [http_body_util::Full](https://docs.rs/http-body-util/0.1.0-rc.2/http_body_util/struct.Full.html)
///   type.
/// * The `E` represents any error type which will be used by route handlers and the middlewares. This error type must implement the [std::error::Error](https://doc.rust-lang.org/std/error/trait.Error.html).
///
/// # Examples
///
/// ```no_run
/// use hyper::{Body, Request, Response, Server};
/// use routerify::{Router, RouterService};
/// use std::convert::Infallible;
/// use std::net::SocketAddr;
///
/// // A handler for "/" page.
/// async fn home(_: Request<Body>) -> Result<Response<Body>, Infallible> {
///     Ok(Response::new(Body::from("Home page")))
/// }
///
/// fn router() -> Router<Body, Infallible> {
///     Router::builder()
///         .get("/", home)
///         .build()
///         .unwrap()
/// }
///
/// #[tokio::main]
/// async fn main() {
///     let router = router();
///
///     // Create a Service from the router above to handle incoming requests.
///     let service = RouterService::new(router).unwrap();
///
///     let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
///
///     // Create a server by passing the created service to `.serve` method.
///     let server = Server::bind(&addr).serve(service);
///
///     println!("App is running on: {}", addr);
///     if let Err(err) = server.await {
///         eprintln!("Server error: {}", err);
///    }
/// }
/// ```
#[derive(Debug)]
pub struct RouterService<RequestBody, ResponseBody, E> {
    builder: RequestServiceBuilder<RequestBody, ResponseBody, E>,
}

impl<
        RequestBody: Body + Send + Sync + 'static,
        ResponseBody: Body + Send + Sync + 'static,
        E: Into<Box<dyn std::error::Error + Send + Sync>> + 'static,
    > RouterService<RequestBody, ResponseBody, E>
{
    /// Creates a new service with the provided router and it's ready to be used with the hyper [`serve`](https://docs.rs/hyper/0.14.4/hyper/server/struct.Builder.html#method.serve)
    /// method.
    pub fn new(
        router: Router<RequestBody, ResponseBody, E>,
    ) -> crate::Result<RouterService<RequestBody, ResponseBody, E>> {
        let builder = RequestServiceBuilder::new(router)?;
        Ok(RouterService { builder })
    }

    pub fn router(&self) -> Arc<Router<RequestBody, ResponseBody, E>> {
        self.builder.router.clone()
    }
}

impl<
        RequestBody: Body + Send + Sync + 'static,
        ResponseBody: Body + Send + Sync + 'static,
        E: Into<Box<dyn std::error::Error + Send + Sync>> + 'static,
    > Service<Request<RequestBody>> for RouterService<RequestBody, ResponseBody, E>
{
    type Response = http::Response<ResponseBody>;
    type Error = crate::RouteError;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn call(&mut self, req: Request<RequestBody>) -> Self::Future {
        let mut req_service = self.builder.build();
        req_service.call(req)
    }
}
