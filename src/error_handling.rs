use std::collections::HashMap;
use std::io;
use std::env;
use std::time::Duration;
use actix_extensible_rate_limit as rl;
use actix_extensible_rate_limit::backend::memory::InMemoryBackend;
use actix_extensible_rate_limit::backend::{SimpleInputFunctionBuilder, SimpleInputFuture};
use actix_extensible_rate_limit::{RateLimiter, RateLimiterBuilder};
use actix_web::{middleware, body::BoxBody, dev::ServiceResponse, get,
                http::{header::ContentType, StatusCode},
                middleware::{ErrorHandlerResponse, ErrorHandlers},
                web, App, HttpResponse, HttpServer, Result, post};
use handlebars::Handlebars;
use serde_json::json;
use serde::Deserialize;

// Custom error handlers, to return HTML responses when an error occurs.
pub(crate) fn error_handlers() -> ErrorHandlers<BoxBody> {
  ErrorHandlers::new().handler(StatusCode::NOT_FOUND, not_found)
}

// Error handler for a 404 Page not found error.
fn not_found<B>(res: ServiceResponse<B>) -> Result<ErrorHandlerResponse<BoxBody>> {
  let response = get_error_response(&res, "Not Found");
  Ok(ErrorHandlerResponse::Response(ServiceResponse::new(
    res.into_parts().0,
    response.map_into_left_body(),
  )))
}

// Generic error handler.
fn get_error_response<B>(res: &ServiceResponse<B>, error: &str) -> HttpResponse<BoxBody> {
  let request = res.request();

  // Provide a fallback to a simple plain text response in case an error occurs during the
  // rendering of the error page.
  let fallback = |err: &str| {
    HttpResponse::build(res.status())
      .content_type(ContentType::plaintext())
      .body(err.to_string())
  };

  let hb = request
    .app_data::<web::Data<Handlebars>>()
    .map(|t| t.get_ref());
  match hb {
    Some(hb) => {
      let data = json!({
            "request_method": format!("{}", request.method()),
            "request_uri": format!("{}", request.uri()),
            "error": error,
            "status_code": res.status().as_str(),
            "error_info": ""
            });
      let body = hb.render("error", &data);

      match body {
        Ok(body) => HttpResponse::build(res.status())
          .content_type(ContentType::html())
          .body(body),
        Err(_) => fallback(error),
      }
    }
    None => fallback(error),
  }
}
