use std::net::{IpAddr, Ipv4Addr};
use std::{net::SocketAddr, str::FromStr};

use aws_lambda_events::{
    encodings::Body,
    event::apigw::{
        ApiGatewayProxyRequestContext, ApiGatewayV2httpRequestContext, ApiGatewayV2httpRequestContextHttpDescription,
    },
};
use bytes::{Buf, Bytes};
use http_body_util::{BodyExt, Full};
use hyper::service::Service;
use lambda_http::{
    handler,
    lambda::{self, Context},
    request::RequestContext,
    IntoResponse, Request, RequestExt, Response,
};
use slog::debug;
use sloggers::terminal::{Destination, TerminalLoggerBuilder};
use sloggers::types::Severity;
use sloggers::Build;

use routerify::{RequestServiceBuilder, Router};

type HandlerError = Box<dyn std::error::Error + Send + Sync + 'static>;

const SERVER_ADDR: &str = "127.0.0.1:8080";

#[tokio::main]
async fn main() -> Result<(), HandlerError> {
    std::env::set_var("RUST_BACKTRACE", "1");
    lambda::run(handler(entrypoint)).await?;
    Ok(())
}

async fn entrypoint(req: Request, _ctx: Context) -> Result<impl IntoResponse, HandlerError> {
    let level = Severity::Info;
    // let level = Severity::Trace;
    let logger = TerminalLoggerBuilder::new()
        .level(level)
        .destination(Destination::Stderr)
        .build()
        .unwrap();

    // Cache the query parameters.
    let query_params = req.query_string_parameters();

    // Convert the lambda_http::Request into a hyper::Request, replacing the URI
    // with the local address of the routerify server.
    let (mut parts, body) = req.into_parts();
    let body = match body {
        Body::Empty => Full::default(),
        Body::Text(t) => Full::from(t.into_bytes()),
        Body::Binary(b) => Full::from(b),
    };
    let mut uri = format!("http://{}{}", SERVER_ADDR, parts.uri.path());

    // AWS Lambda Rust Runtime will automatically parse the query params *and*
    // remove those query parameters from the original URI. This is fine if
    // you're writing your logic directly in the handler function, but for
    // passing-through to a separate router library, we need to
    // re-url-encode the query parameters and place them back into the URI.
    if !query_params.is_empty() {
        append_querystring_from_map(&mut uri, query_params.iter());
    }

    parts.uri = match hyper::Uri::from_str(uri.as_str()) {
        Ok(uri) => uri,
        Err(e) => panic!("failed to build uri: {:?}", e),
    };
    let req = hyper::Request::from_parts(parts, body);
    debug!(logger, "lambda request: {:#?}", req);
    debug!(logger, "request uri path: {}", req.uri().path());

    let router: Router<Full<Bytes>, Full<Bytes>, routerify::Error> = Router::builder()
        .get("/", |_| async move { Ok(Response::new("Hello, world!".into())) })
        .build()
        .unwrap();

    let builder = RequestServiceBuilder::new(router)?;
    let mut service = builder.build();
    let resp = service.call(req).await?;

    // Parse the hyper::Request from Routerify into a lambda_http::Request
    let (parts, body) = resp.into_parts();
    let body = std::io::read_to_string(body.collect().await?.aggregate().reader())?;
    Ok(Response::from_parts(parts, Body::from(body)))
}

#[allow(dead_code)]
fn get_remote_addr(req: &Request) -> SocketAddr {
    const PORT: u16 = 8080;
    let source_ip: String = match req.request_context() {
        RequestContext::ApiGatewayV1(ApiGatewayProxyRequestContext { identity, .. }) => {
            identity.source_ip.unwrap_or_else(|| Ipv4Addr::UNSPECIFIED.to_string())
        }
        RequestContext::ApiGatewayV2(ApiGatewayV2httpRequestContext {
            http: ApiGatewayV2httpRequestContextHttpDescription { source_ip, .. },
            ..
        }) => source_ip.unwrap_or_else(|| Ipv4Addr::UNSPECIFIED.to_string()),
        _ => Ipv4Addr::UNSPECIFIED.to_string(),
    };
    SocketAddr::new(IpAddr::from_str(source_ip.as_str()).unwrap(), PORT)
}

fn append_querystring_from_map<'a, I>(uri: &mut String, from_query_params: I)
where
    I: Iterator<Item = (&'a str, &'a str)>,
{
    uri.push('?');
    let mut serializer = url::form_urlencoded::Serializer::new(String::new());
    for (key, value) in from_query_params.into_iter() {
        serializer.append_pair(key, value);
    }
    uri.push_str(serializer.finish().as_str())
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use hyper::{Method, StatusCode};

    use super::*;

    #[test]
    fn should_parse_query_params_as_expected() {
        // let expected = "?FOO=BAR&BOO=BAZ&key=value";
        let mut buffer = String::new();
        let query_params = {
            let mut m = HashMap::new();
            m.insert("FOO", "BAR");
            m.insert("BOO", "BAZ");
            m.insert("key", "value");
            m
        };
        append_querystring_from_map(&mut buffer, query_params.clone().into_iter());
        for (key, value) in query_params {
            assert!(buffer.contains(&(key.to_string() + "=" + value)));
        }
    }

    #[tokio::test]
    async fn should_return_200_and_response() {
        let mut request = lambda_http::Request::new(aws_lambda_events::encodings::Body::Empty);
        *request.uri_mut() = "/".parse().unwrap();
        request
            .extensions_mut()
            .insert::<RequestContext>(RequestContext::ApiGatewayV2(ApiGatewayV2httpRequestContext {
                route_key: None,
                account_id: None,
                stage: None,
                request_id: None,
                authorizer: None,
                apiid: None,
                domain_name: None,
                domain_prefix: None,
                time: None,
                time_epoch: 0,
                http: ApiGatewayV2httpRequestContextHttpDescription {
                    method: Method::GET,
                    path: None,
                    protocol: None,
                    source_ip: Some("127.0.0.1".to_string()),
                    user_agent: None,
                },
            }));
        let response = entrypoint(request, Context::default()).await.unwrap().into_response();
        assert_eq!(StatusCode::OK, response.status());
        assert!(!response.body().is_empty())
    }
}
