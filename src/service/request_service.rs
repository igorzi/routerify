use crate::helpers;
use crate::router::Router;
use crate::types::{RequestContext, RequestInfo};
use crate::Error;
use hyper::{body::Body, service::Service, Request, Response};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

pub struct RequestService<RequestBody, ResponseBody, E> {
    pub(crate) router: Arc<Router<RequestBody, ResponseBody, E>>,
}

impl<
        RequestBody: Body + Send + Sync + 'static,
        ResponseBody: Body + Send + Sync + 'static,
        E: Into<Box<dyn std::error::Error + Send + Sync>> + 'static,
    > Service<Request<RequestBody>> for RequestService<RequestBody, ResponseBody, E>
{
    type Response = Response<ResponseBody>;
    type Error = crate::RouteError;
    #[allow(clippy::type_complexity)]
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn call(&mut self, mut req: Request<RequestBody>) -> Self::Future {
        let router = self.router.clone();

        let fut = async move {
            let mut target_path = helpers::percent_decode_request_path(req.uri().path())
                .map_err(|e| Error::new(format!("Couldn't percent decode request path: {}", e)))?;

            if target_path.is_empty() || target_path.as_bytes()[target_path.len() - 1] != b'/' {
                target_path.push('/');
            }

            let mut req_info = None;
            let should_gen_req_info = router
                .should_gen_req_info
                .expect("The `should_gen_req_info` flag in Router is not initialized");

            let context = RequestContext::new();

            if should_gen_req_info {
                req_info = Some(RequestInfo::new_from_req(&req, context.clone()));
            }

            req.extensions_mut().insert(context);

            router.process(target_path.as_str(), req, req_info.clone()).await
        };

        Box::pin(fut)
    }
}

#[derive(Debug)]
pub struct RequestServiceBuilder<RequestBody, ResponseBody, E> {
    pub(crate) router: Arc<Router<RequestBody, ResponseBody, E>>,
}

impl<
        RequestBody: Body + Send + Sync + 'static,
        ResponseBody: Body + Send + Sync + 'static,
        E: Into<Box<dyn std::error::Error + Send + Sync>> + 'static,
    > RequestServiceBuilder<RequestBody, ResponseBody, E>
{
    pub fn new(mut router: Router<RequestBody, ResponseBody, E>) -> crate::Result<Self> {
        // router.init_keep_alive_middleware();

        router.init_global_options_route();
        router.init_default_404_route();

        router.init_err_handler();

        router.init_regex_set()?;
        router.init_req_info_gen();
        Ok(Self {
            router: Arc::from(router),
        })
    }

    pub fn build(&self) -> RequestService<RequestBody, ResponseBody, E> {
        RequestService {
            router: self.router.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{Error, RequestServiceBuilder, Router};
    use bytes::{Buf, Bytes};
    use http::Method;
    use http_body_util::{BodyExt, Empty, Full};
    use hyper::service::Service;
    use hyper::{Request, Response};

    #[tokio::test]
    async fn should_route_request() {
        const RESPONSE_TEXT: &str = "Hello world!";
        let router: Router<Empty<Bytes>, Full<Bytes>, Error> = Router::builder()
            .get("/", |_| async move { Ok(Response::new(Full::from(RESPONSE_TEXT))) })
            .build()
            .unwrap();
        let req = Request::builder()
            .method(Method::GET)
            .uri("/")
            .body(Empty::<Bytes>::new())
            .unwrap();
        let builder = RequestServiceBuilder::new(router).unwrap();
        let mut service = builder.build();
        let resp: Response<Full<Bytes>> = service.call(req).await.unwrap();
        let body = resp.into_body().collect().await.unwrap().aggregate();
        let body = std::io::read_to_string(body.reader()).unwrap();
        assert_eq!(RESPONSE_TEXT, body)
    }
}
