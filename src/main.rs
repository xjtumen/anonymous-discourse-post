use std::collections::HashMap;
use std::io;
use std::env;
use std::time::Duration;
use actix_extensible_rate_limit::backend::memory::InMemoryBackend;
use actix_extensible_rate_limit::backend::SimpleInputFunctionBuilder;
use actix_extensible_rate_limit::RateLimiter;
use actix_web::{middleware, body::BoxBody, dev::ServiceResponse, get,
                http::{header::ContentType, StatusCode},
                middleware::{ErrorHandlerResponse, ErrorHandlers},
                web, App, HttpResponse, HttpServer, Result, post};
use handlebars::Handlebars;
use serde_json::json;
use serde::Deserialize;

const XJTUMEN_URL_BASE: &str = "https://xjtu.live/posts";
const MAX_REQ_PER_60S: u64 = 2;

#[derive(Debug, Deserialize)]
pub struct WebForm {
  content: String,
  topic_id: String,
}

// Macro documentation can be found in the actix_web_codegen crate
#[post("/xjtumen-custom-api/discourse-post-to-topic/{hostname}")]
async fn do_discourse_post_to_topic(hb: web::Data<Handlebars<'_>>, form: web::Form<WebForm>, path: web::Path<String>) -> HttpResponse {
  let mut map = HashMap::from([
    ("category", ""),
    ("title", ""),
    ("raw", &*form.content),
    ("topic_id", &*form.topic_id),
  ]);
  map.insert("body", "json");

  let client = reqwest::Client::new();
  let api_key_anonymous = env::var("DISCOURSE_API_KEY_ANONYMOUS").unwrap();

  let res = client.post(XJTUMEN_URL_BASE)
    .header("Accept", "application/json; charset=utf-8")
    .header("Api-Key", api_key_anonymous)
    .header("Api-Username", "anonymous_user")
    .json(&map)
    .send()
    .await.unwrap();
  // println!("{}", res.status());
  if res.status().is_success() {
    let res_json = res.json::<serde_json::Value>().await.unwrap();
    // println!("{:?}", res_json);
    let response_post_id = res_json.get("post_number").unwrap().as_i64().unwrap_or(0);
    let reply_result_url = format!("https://{}/t/-/{}/{}", path.as_str(), form.topic_id, response_post_id);
    let data = json!({
    "hostname": path.as_str(),
      "topic_id": form.topic_id,
      "reply_result_url": reply_result_url
    });
    let body = hb.render("success-do_discourse_post_to_topic", &data).unwrap();
    HttpResponse::Ok().content_type(ContentType::html()).body(body)
  } else {
    HttpResponse::InternalServerError().body(
      format!("API Request Failed with {}: {:?}", res.status().as_str(), res.text().await.unwrap()))
  }
}

#[get("/xjtumen-custom-api/handle-reply-to-topic/{hostname}/{topic_id}/{tail:.*}")]
async fn handle_reply_topic(hb: web::Data<Handlebars<'_>>, path: web::Path<(String, String, String)>)
                            -> HttpResponse {
  let data = json!({
        "hostname": path.0,
        "topic_id": path.1,
        "title": path.2,
    });
  let body = hb.render("reply", &data).unwrap();

  HttpResponse::Ok().content_type(ContentType::html()).body(body)
}

#[actix_web::main]
async fn main() -> io::Result<()> {
  // A backend is responsible for storing rate limit data, and choosing whether to allow/deny requests
  let backend = InMemoryBackend::builder().build();
  // Handlebars uses a repository for the compiled templates. This object must be
  // shared between the application threads, and is therefore passed to the
  // Application Builder as an atomic reference-counted pointer.
  let mut handlebars = Handlebars::new();
  handlebars
    .register_templates_directory(".html", "./static/templates")
    .unwrap();
  let handlebars_ref = web::Data::new(handlebars);

  HttpServer::new(move || {
    let input = SimpleInputFunctionBuilder::new(Duration::from_secs(60), MAX_REQ_PER_60S)
      .peer_ip_key() // if use CDN, use `realip_remote_addr` instead
      .path_key() // rate limit at path level
      .build();
    let rate_limit = RateLimiter::builder(backend.clone(), input)
      .add_headers()
      .request_denied_response(|_|
        HttpResponse::build(StatusCode::TOO_MANY_REQUESTS).body(
          format!("You have reached the rate limit of\n\
           anonymous reply functionality,\n\
          which is {} / 60s for each topic.\nPlease try again later.", MAX_REQ_PER_60S)))
      .build();
    App::new()
      .wrap(error_handlers()) // middlewares' order matter!
      .wrap(rate_limit)
      .wrap(middleware::Logger::default())
      .app_data(handlebars_ref.clone())
      .service(handle_reply_topic)
      .service(do_discourse_post_to_topic)
  })
    .bind(("127.0.0.1", 7010))?
    .run()
    .await
}

// Custom error handlers, to return HTML responses when an error occurs.
fn error_handlers() -> ErrorHandlers<BoxBody> {
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
