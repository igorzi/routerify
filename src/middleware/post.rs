use crate::regex_generator::generate_exact_match_regex;
use crate::types::RequestInfo;
use crate::Error;
use hyper::{body::Body, Response};
use regex::Regex;
use std::fmt::{self, Debug, Formatter};
use std::future::Future;
use std::pin::Pin;

type HandlerWithoutInfo<ResponseBody, E> =
    Box<dyn Fn(Response<ResponseBody>) -> HandlerWithoutInfoReturn<ResponseBody, E> + Send + Sync + 'static>;
type HandlerWithoutInfoReturn<ResponseBody, E> =
    Box<dyn Future<Output = Result<Response<ResponseBody>, E>> + Send + 'static>;

type HandlerWithInfo<ResponseBody, E> =
    Box<dyn Fn(Response<ResponseBody>, RequestInfo) -> HandlerWithInfoReturn<ResponseBody, E> + Send + Sync + 'static>;
type HandlerWithInfoReturn<ResponseBody, E> =
    Box<dyn Future<Output = Result<Response<ResponseBody>, E>> + Send + 'static>;

/// The post middleware type. Refer to [Post Middleware](./index.html#post-middleware) for more info.
///
/// This `PostMiddleware<RequestBody, ResponseBody, E>` type accepts two type parameters: `B` and `E`.
///
/// * The `RequestBody` represents the response body type which will be used by route handlers and the middlewares and this body type must implement
///   the [hyper::body::Body](https://docs.rs/hyper/1.0.0-rc.3/hyper/body/trait.Body.html) trait. For an instance, `RequestBody` could be [hyper::body::Incoming](https://docs.rs/hyper/1.0.0-rc.3/hyper/body/struct.Incoming.html)
///   type.
/// * The `ResponseBody` represents the response body type which will be used by route handlers and the middlewares and this body type must implement
///   the [hyper::body::Body](https://docs.rs/hyper/1.0.0-rc.3/hyper/body/trait.Body.html) trait. For an instance, `ResponseBody` could be [http_body_util::Full](https://docs.rs/http-body-util/0.1.0-rc.2/http_body_util/struct.Full.html)
///   type.
/// * The `E` represents any error type which will be used by route handlers and the middlewares. This error type must implement the [std::error::Error](https://doc.rust-lang.org/std/error/trait.Error.html).
pub struct PostMiddleware<ResponseBody, E> {
    pub(crate) path: String,
    pub(crate) regex: Regex,
    // Make it an option so that when a router is used to scope in another router,
    // It can be extracted out by 'opt.take()' without taking the whole router's ownership.
    pub(crate) handler: Option<Handler<ResponseBody, E>>,
    // Scope depth with regards to the top level router.
    pub(crate) scope_depth: u32,
}

pub(crate) enum Handler<ResponseBody, E> {
    WithoutInfo(HandlerWithoutInfo<ResponseBody, E>),
    WithInfo(HandlerWithInfo<ResponseBody, E>),
}

impl<ResponseBody: Body + Send + Sync + 'static, E: Into<Box<dyn std::error::Error + Send + Sync>> + 'static>
    PostMiddleware<ResponseBody, E>
{
    pub(crate) fn new_with_boxed_handler<P: Into<String>>(
        path: P,
        handler: Handler<ResponseBody, E>,
        scope_depth: u32,
    ) -> crate::Result<PostMiddleware<ResponseBody, E>> {
        let path = path.into();
        let (re, _) = generate_exact_match_regex(path.as_str()).map_err(|e| {
            Error::new(format!(
                "Could not create an exact match regex for the post middleware path: {}",
                e
            ))
        })?;

        Ok(PostMiddleware {
            path,
            regex: re,
            handler: Some(handler),
            scope_depth,
        })
    }

    /// Creates a post middleware with a handler at the specified path.
    ///
    /// # Examples
    ///
    /// ```
    /// use routerify::{Router, Middleware, PostMiddleware};
    /// use hyper::{Response, Body};
    /// use std::convert::Infallible;
    ///
    /// # fn run() -> Router<Body, Infallible> {
    /// let router = Router::builder()
    ///      .middleware(Middleware::Post(PostMiddleware::new("/abc", |res| async move { /* Do some operations */ Ok(res) }).unwrap()))
    ///      .build()
    ///      .unwrap();
    /// # router
    /// # }
    /// # run();
    /// ```
    pub fn new<P, H, R>(path: P, handler: H) -> crate::Result<PostMiddleware<ResponseBody, E>>
    where
        P: Into<String>,
        H: Fn(Response<ResponseBody>) -> R + Send + Sync + 'static,
        R: Future<Output = Result<Response<ResponseBody>, E>> + Send + 'static,
    {
        let handler: HandlerWithoutInfo<ResponseBody, E> =
            Box::new(move |res: Response<ResponseBody>| Box::new(handler(res)));
        PostMiddleware::new_with_boxed_handler(path, Handler::WithoutInfo(handler), 1)
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
    ///      .middleware(Middleware::Post(PostMiddleware::new_with_info("/abc", post_middleware_with_info_handler).unwrap()))
    ///      .build()
    ///      .unwrap();
    /// # router
    /// # }
    /// # run();
    /// ```
    pub fn new_with_info<P, H, R>(path: P, handler: H) -> crate::Result<PostMiddleware<ResponseBody, E>>
    where
        P: Into<String>,
        H: Fn(Response<ResponseBody>, RequestInfo) -> R + Send + Sync + 'static,
        R: Future<Output = Result<Response<ResponseBody>, E>> + Send + 'static,
    {
        let handler: HandlerWithInfo<ResponseBody, E> =
            Box::new(move |res: Response<ResponseBody>, req_info: RequestInfo| Box::new(handler(res, req_info)));
        PostMiddleware::new_with_boxed_handler(path, Handler::WithInfo(handler), 1)
    }

    pub(crate) fn should_require_req_meta(&self) -> bool {
        if let Some(ref handler) = self.handler {
            match handler {
                Handler::WithInfo(_) => true,
                Handler::WithoutInfo(_) => false,
            }
        } else {
            false
        }
    }

    pub(crate) async fn process(
        &self,
        res: Response<ResponseBody>,
        req_info: Option<RequestInfo>,
    ) -> crate::Result<Response<ResponseBody>> {
        let handler = self
            .handler
            .as_ref()
            .expect("A router can not be used after mounting into another router");

        match handler {
            Handler::WithoutInfo(ref handler) => Pin::from(handler(res)).await.map_err(Into::into),
            Handler::WithInfo(ref handler) => Pin::from(handler(res, req_info.expect("No RequestInfo is provided")))
                .await
                .map_err(Into::into),
        }
    }
}

impl<ResponseBody, E> Debug for PostMiddleware<ResponseBody, E> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{{ path: {:?}, regex: {:?} }}", self.path, self.regex)
    }
}
