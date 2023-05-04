use crate::types::RequestInfo;
use hyper::{body::Body, Request, Response};
use std::future::Future;

pub use self::post::PostMiddleware;
pub use self::pre::PreMiddleware;

mod post;
mod pre;

/// Enum type for all the middleware types. Please refer to the [Middleware](./index.html#middleware) for more info.
///
/// This `Middleware<RequestBody, ResponseBody, E>` type accepts two type parameters: `B` and `E`.
///
/// * The `RequestBody` represents the response body type which will be used by route handlers and the middlewares and this body type must implement
///   the [hyper::body::Body](https://docs.rs/hyper/1.0.0-rc.3/hyper/body/trait.Body.html) trait. For an instance, `RequestBody` could be [hyper::body::Incoming](https://docs.rs/hyper/1.0.0-rc.3/hyper/body/struct.Incoming.html)
///   type.
/// * The `ResponseBody` represents the response body type which will be used by route handlers and the middlewares and this body type must implement
///   the [hyper::body::Body](https://docs.rs/hyper/1.0.0-rc.3/hyper/body/trait.Body.html) trait. For an instance, `ResponseBody` could be [http_body_util::Full](https://docs.rs/http-body-util/0.1.0-rc.2/http_body_util/struct.Full.html)
///   type.
/// * The `E` represents any error type which will be used by route handlers and the middlewares. This error type must implement the [std::error::Error](https://doc.rust-lang.org/std/error/trait.Error.html).
#[derive(Debug)]
pub enum Middleware<RequestBody, ResponseBody, E> {
    /// Variant for the pre middleware. Refer to [Pre Middleware](./index.html#pre-middleware) for more info.
    Pre(PreMiddleware<RequestBody, E>),

    /// Variant for the post middleware. Refer to [Post Middleware](./index.html#post-middleware) for more info.
    Post(PostMiddleware<ResponseBody, E>),
}

impl<
        RequestBody: Body + Send + Sync + 'static,
        ResponseBody: Body + Send + Sync + 'static,
        E: Into<Box<dyn std::error::Error + Send + Sync>> + 'static,
    > Middleware<RequestBody, ResponseBody, E>
{
    /// Creates a pre middleware with a handler at the `/*` path.
    ///
    /// # Examples
    ///
    /// ```
    /// use routerify::{Router, Middleware};
    /// use hyper::{Request, Body};
    /// use std::convert::Infallible;
    ///
    /// # fn run() -> Router<Body, Infallible> {
    /// let router = Router::builder()
    ///      .middleware(Middleware::pre(|req| async move { /* Do some operations */ Ok(req) }))
    ///      .build()
    ///      .unwrap();
    /// # router
    /// # }
    /// # run();
    /// ```
    pub fn pre<H, R>(handler: H) -> Middleware<RequestBody, ResponseBody, E>
    where
        H: Fn(Request<RequestBody>) -> R + Send + Sync + 'static,
        R: Future<Output = Result<Request<RequestBody>, E>> + Send + 'static,
    {
        Middleware::pre_with_path("/*", handler).unwrap()
    }

    /// Creates a post middleware with a handler at the `/*` path.
    ///
    /// # Examples
    ///
    /// ```
    /// use routerify::{Router, Middleware};
    /// use hyper::{Response, Body};
    /// use std::convert::Infallible;
    ///
    /// # fn run() -> Router<Body, Infallible> {
    /// let router = Router::builder()
    ///      .middleware(Middleware::post(|res| async move { /* Do some operations */ Ok(res) }))
    ///      .build()
    ///      .unwrap();
    /// # router
    /// # }
    /// # run();
    /// ```
    pub fn post<H, R>(handler: H) -> Middleware<RequestBody, ResponseBody, E>
    where
        H: Fn(Response<ResponseBody>) -> R + Send + Sync + 'static,
        R: Future<Output = Result<Response<ResponseBody>, E>> + Send + 'static,
    {
        Middleware::post_with_path("/*", handler).unwrap()
    }

    /// Creates a post middleware which can access [request info](./struct.RequestInfo.html) e.g. headers, method, uri etc. It should be used when the post middleware trandforms the response based on
    /// the request information.
    ///
    /// # Examples
    ///
    /// ```
    /// use routerify::{Router, Middleware, PostMiddleware, RequestInfo};
    /// use hyper::{Response, Body};
    /// use std::convert::Infallible;
    ///
    /// async fn post_middleware_with_info_handler(res: Response<Body>, req_info: RequestInfo) -> Result<Response<Body>, Infallible> {
    ///     let headers = req_info.headers();
    ///     
    ///     // Do some response transformation based on the request headers, method etc.
    ///     
    ///     Ok(res)
    /// }
    ///
    /// # fn run() -> Router<Body, Infallible> {
    /// let router = Router::builder()
    ///      .middleware(Middleware::post_with_info(post_middleware_with_info_handler))
    ///      .build()
    ///      .unwrap();
    /// # router
    /// # }
    /// # run();
    /// ```
    pub fn post_with_info<H, R>(handler: H) -> Middleware<RequestBody, ResponseBody, E>
    where
        H: Fn(Response<ResponseBody>, RequestInfo) -> R + Send + Sync + 'static,
        R: Future<Output = Result<Response<ResponseBody>, E>> + Send + 'static,
    {
        Middleware::post_with_info_with_path("/*", handler).unwrap()
    }

    /// Create a pre middleware with a handler at the specified path.
    ///
    /// # Examples
    ///
    /// ```
    /// use routerify::{Router, Middleware};
    /// use hyper::{Request, Body};
    /// use std::convert::Infallible;
    ///
    /// # fn run() -> Router<Body, Infallible> {
    /// let router = Router::builder()
    ///      .middleware(Middleware::pre_with_path("/my-path", |req| async move { /* Do some operations */ Ok(req) }).unwrap())
    ///      .build()
    ///      .unwrap();
    /// # router
    /// # }
    /// # run();
    /// ```
    pub fn pre_with_path<P, H, R>(path: P, handler: H) -> crate::Result<Middleware<RequestBody, ResponseBody, E>>
    where
        P: Into<String>,
        H: Fn(Request<RequestBody>) -> R + Send + Sync + 'static,
        R: Future<Output = Result<Request<RequestBody>, E>> + Send + 'static,
    {
        Ok(Middleware::Pre(PreMiddleware::new(path, handler)?))
    }

    /// Creates a post middleware with a handler at the specified path.
    ///
    /// # Examples
    ///
    /// ```
    /// use routerify::{Router, Middleware};
    /// use hyper::{Response, Body};
    /// use std::convert::Infallible;
    ///
    /// # fn run() -> Router<Body, Infallible> {
    /// let router = Router::builder()
    ///      .middleware(Middleware::post_with_path("/my-path", |res| async move { /* Do some operations */ Ok(res) }).unwrap())
    ///      .build()
    ///      .unwrap();
    /// # router
    /// # }
    /// # run();
    /// ```
    pub fn post_with_path<P, H, R>(path: P, handler: H) -> crate::Result<Middleware<RequestBody, ResponseBody, E>>
    where
        P: Into<String>,
        H: Fn(Response<ResponseBody>) -> R + Send + Sync + 'static,
        R: Future<Output = Result<Response<ResponseBody>, E>> + Send + 'static,
    {
        Ok(Middleware::Post(PostMiddleware::new(path, handler)?))
    }

    /// Creates a post middleware which can access [request info](./struct.RequestInfo.html) e.g. headers, method, uri etc. It should be used when the post middleware trandforms the response based on
    /// the request information.
    ///
    /// # Examples
    ///
    /// ```
    /// use routerify::{Router, Middleware, PostMiddleware, RequestInfo};
    /// use hyper::{Response, Body};
    /// use std::convert::Infallible;
    ///
    /// async fn post_middleware_with_info_handler(res: Response<Body>, req_info: RequestInfo) -> Result<Response<Body>, Infallible> {
    ///     let _headers = req_info.headers();
    ///     
    ///     // Do some response transformation based on the request headers, method etc.
    ///     
    ///     Ok(res)
    /// }
    ///
    /// # fn run() -> Router<Body, Infallible> {
    /// let router = Router::builder()
    ///      .middleware(Middleware::post_with_info_with_path("/abc", post_middleware_with_info_handler).unwrap())
    ///      .build()
    ///      .unwrap();
    /// # router
    /// # }
    /// # run();
    /// ```
    pub fn post_with_info_with_path<P, H, R>(
        path: P,
        handler: H,
    ) -> crate::Result<Middleware<RequestBody, ResponseBody, E>>
    where
        P: Into<String>,
        H: Fn(Response<ResponseBody>, RequestInfo) -> R + Send + Sync + 'static,
        R: Future<Output = Result<Response<ResponseBody>, E>> + Send + 'static,
    {
        Ok(Middleware::Post(PostMiddleware::new_with_info(path, handler)?))
    }
}
