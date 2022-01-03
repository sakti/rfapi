use dropshot::endpoint;
use dropshot::ApiDescription;
use dropshot::ConfigDropshot;
use dropshot::ConfigLogging;
use dropshot::ConfigLoggingLevel;
use dropshot::HttpError;
use dropshot::HttpResponseOk;
use dropshot::HttpResponseUpdatedNoContent;
use dropshot::HttpServerStarter;
use dropshot::RequestContext;
use dropshot::TypedBody;
use http::{Response, StatusCode};
use hyper::Body;
use lazy_static::lazy_static;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use std::fs::File;
use std::io::Cursor;
use std::str::from_utf8;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::sync::Arc;

lazy_static! {
    static ref OPENAPI_DOC: String = {
        let mut output = Cursor::new(Vec::new());
        let api = build_api_description();
        let _ = api
            .openapi("rfapi", "v0.1.0")
            .description("forever in progress")
            .contact_name("sakti")
            .write(&mut output);
        let result = from_utf8(&output.get_ref()).unwrap();
        result.to_owned()
    };
}

#[endpoint {
    method = GET,
    path = "/",
}]
async fn index(_rqctx: Arc<RequestContext<ExampleContext>>) -> Result<Response<Body>, HttpError> {
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(http::header::CONTENT_TYPE, "text/html")
        .body(format!("testing {}", "abc").into())?)
}

#[endpoint {
    method = GET,
    path = "/openapi.json",
}]
async fn docs(_rqctx: Arc<RequestContext<ExampleContext>>) -> Result<Response<Body>, HttpError> {
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(http::header::CONTENT_TYPE, "application/json")
        .body(format!("{}", *OPENAPI_DOC).into())?)
}

fn build_api_description() -> ApiDescription<ExampleContext> {
    let mut api = ApiDescription::new();
    // Register API functions -- see detailed example or ApiDescription docs.
    api.register(index).unwrap();
    api.register(docs).unwrap();
    api.register(example_api_get_counter).unwrap();
    api.register(example_api_put_counter).unwrap();
    return api;
}

#[tokio::main]
async fn main() -> Result<(), String> {
    // Set up a logger.
    let log = ConfigLogging::StderrTerminal {
        level: ConfigLoggingLevel::Info,
    }
    .to_logger("minimal-example")
    .map_err(|e| e.to_string())?;

    // Describe the API.
    let api = build_api_description();

    // swagger/openapi
    let mut file = File::create("docs.json").unwrap();
    api.openapi("rfastapi test", "v1.0.0")
        .write(&mut file)
        .map_err(|e| e.to_string())?;
    drop(file);

    let api_context = ExampleContext::new();

    // Start the server.
    let server = HttpServerStarter::new(
        &ConfigDropshot {
            bind_address: "0.0.0.0:8000".parse().unwrap(),
            request_body_max_bytes: 1024,
        },
        api,
        api_context,
        &log,
    )
    .map_err(|error| format!("failed to start server: {}", error))?
    .start();

    server.await
}

struct ExampleContext {
    /** counter that can be manipulated by requests to the HTTP API */
    counter: AtomicU64,
}

impl ExampleContext {
    /**
     * Return a new ExampleContext.
     */
    pub fn new() -> ExampleContext {
        ExampleContext {
            counter: AtomicU64::new(0),
        }
    }
}

#[derive(Deserialize, Serialize, JsonSchema)]
struct CounterValue {
    counter: u64,
}

/**
 * Fetch the current value of the counter.
 */
#[endpoint {
    method = GET,
    path = "/counter",
}]
async fn example_api_get_counter(
    rqctx: Arc<RequestContext<ExampleContext>>,
) -> Result<HttpResponseOk<CounterValue>, HttpError> {
    let api_context = rqctx.context();

    Ok(HttpResponseOk(CounterValue {
        counter: api_context.counter.load(Ordering::SeqCst),
    }))
}

/**
 * Update the current value of the counter.  Note that the special value of 10
 * is not allowed (just to demonstrate how to generate an error).
 */
#[endpoint {
    method = PUT,
    path = "/counter",
}]
async fn example_api_put_counter(
    rqctx: Arc<RequestContext<ExampleContext>>,
    update: TypedBody<CounterValue>,
) -> Result<HttpResponseUpdatedNoContent, HttpError> {
    let api_context = rqctx.context();
    let updated_value = update.into_inner();

    if updated_value.counter == 10 {
        Err(HttpError::for_bad_request(
            Some(String::from("BadInput")),
            format!("do not like the number {}", updated_value.counter),
        ))
    } else {
        api_context
            .counter
            .store(updated_value.counter, Ordering::SeqCst);
        Ok(HttpResponseUpdatedNoContent())
    }
}
