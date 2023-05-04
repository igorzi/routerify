use crate::helpers;
use crate::regex_generator::generate_exact_match_regex;
use crate::types::{RequestMeta, RouteParams};
use crate::Error;
use hyper::{body::Body, Method, Request, Response};
use regex::Regex;
use std::fmt::{self, Debug, Formatter};
use std::future::Future;
use std::pin::Pin;

type Handler<RequestBody, ResponseBody, E> =
    Box<dyn Fn(Request<RequestBody>) -> HandlerReturn<ResponseBody, E> + Send + Sync + 'static>;
type HandlerReturn<ResponseBody, E> = Box<dyn Future<Output = Result<Response<ResponseBody>, E>> + Send + 'static>;

/// Represents a single route.
///
/// A route consists of a path, http method type(s) and a handler. It shouldn't be created directly, use [RouterBuilder](./struct.RouterBuilder.html) methods
/// to create a route.
///
/// This `Route<RequestBody, ResponseBody, E>` type accepts two type parameters: `B` and `E`.
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
/// ```
/// use routerify::Router;
/// use hyper::{Response, Request, Body};
///
/// async fn home_handler(req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
///     Ok(Response::new(Body::from("home")))
/// }
///
/// # fn run() -> Router<Body, hyper::Error> {
/// let router = Router::builder()
///     // Create a route on "/" path for `GET` method.
///     .get("/", home_handler)
///     .build()
///     .unwrap();
/// # router
/// # }
/// # run();
/// ```
pub struct Route<RequestBody, ResponseBody, E> {
    pub(crate) path: String,
    pub(crate) regex: Regex,
    route_params: Vec<String>,
    // Make it an option so that when a router is used to scope in another router,
    // It can be extracted out by 'opt.take()' without taking the whole router's ownership.
    pub(crate) handler: Option<Handler<RequestBody, ResponseBody, E>>,
    pub(crate) methods: Vec<Method>,
    // Scope depth with regards to the top level router.
    pub(crate) scope_depth: u32,
}

impl<
        RequestBody: Body + Send + Sync + 'static,
        ResponseBody: Body + Send + Sync + 'static,
        E: Into<Box<dyn std::error::Error + Send + Sync>> + 'static,
    > Route<RequestBody, ResponseBody, E>
{
    pub(crate) fn new_with_boxed_handler<P: Into<String>>(
        path: P,
        methods: Vec<Method>,
        handler: Handler<RequestBody, ResponseBody, E>,
        scope_depth: u32,
    ) -> crate::Result<Route<RequestBody, ResponseBody, E>> {
        let path = path.into();
        let (re, params) = generate_exact_match_regex(path.as_str()).map_err(|e| {
            Error::new(format!(
                "Could not create an exact match regex for the route path: {}",
                e
            ))
        })?;

        Ok(Route {
            path,
            regex: re,
            route_params: params,
            handler: Some(handler),
            methods,
            scope_depth,
        })
    }

    pub(crate) fn new<P, H, R>(
        path: P,
        methods: Vec<Method>,
        handler: H,
    ) -> crate::Result<Route<RequestBody, ResponseBody, E>>
    where
        P: Into<String>,
        H: Fn(Request<RequestBody>) -> R + Send + Sync + 'static,
        R: Future<Output = Result<Response<ResponseBody>, E>> + Send + 'static,
    {
        let handler: Handler<RequestBody, ResponseBody, E> =
            Box::new(move |req: Request<RequestBody>| Box::new(handler(req)));
        Route::new_with_boxed_handler(path, methods, handler, 1)
    }

    pub(crate) fn is_match_method(&self, method: &Method) -> bool {
        self.methods.contains(method)
    }

    pub(crate) async fn process(
        &self,
        target_path: &str,
        mut req: Request<RequestBody>,
    ) -> crate::Result<Response<ResponseBody>> {
        self.push_req_meta(target_path, &mut req);

        let handler = self
            .handler
            .as_ref()
            .expect("A router can not be used after mounting into another router");

        Pin::from(handler(req)).await.map_err(Into::into)
    }

    fn push_req_meta(&self, target_path: &str, req: &mut Request<RequestBody>) {
        self.update_req_meta(req, self.generate_req_meta(target_path));
    }

    fn update_req_meta(&self, req: &mut Request<RequestBody>, req_meta: RequestMeta) {
        helpers::update_req_meta_in_extensions(req.extensions_mut(), req_meta);
    }

    fn generate_req_meta(&self, target_path: &str) -> RequestMeta {
        let route_params_list = &self.route_params;
        let ln = route_params_list.len();

        let mut route_params = RouteParams::with_capacity(ln);

        if ln > 0 {
            if let Some(caps) = self.regex.captures(target_path) {
                let mut iter = caps.iter();
                // Skip the first match because it's the whole path.
                iter.next();
                for param in route_params_list {
                    if let Some(Some(g)) = iter.next() {
                        route_params.set(param.clone(), g.as_str());
                    }
                }
            }
        }

        RequestMeta::with_route_params(route_params)
    }
}

impl<RequestBody, ResponseBody, E> Debug for Route<RequestBody, ResponseBody, E> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{{ path: {:?}, regex: {:?}, route_params: {:?}, methods: {:?} }}",
            self.path, self.regex, self.route_params, self.methods
        )
    }
}
