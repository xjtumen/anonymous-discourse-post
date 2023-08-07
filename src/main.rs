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
use actix_web::dev::ServiceRequest;
use handlebars::Handlebars;
use serde_json::json;
use serde::Deserialize;

const XJTUMEN_URL_BASE: &str = "https://xjtu.men/posts.json";

#[derive(Debug, Deserialize)]
pub struct post_to_topic_Form {
  content: String,
  topic_id: String,
}

#[derive(Debug, Deserialize)]
pub struct new_topic_Form {
  topic_content: String,
  topic_title: String,
}

#[post("/{hostname}")]
async fn do_discourse_post_to_topic(hb: web::Data<Handlebars<'_>>, form: web::Form<post_to_topic_Form>, path: web::Path<String>) -> HttpResponse {
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


#[post("/{hostname}")]
async fn do_discourse_new_topic(hb: web::Data<Handlebars<'_>>, form: web::Form<new_topic_Form>, path: web::Path<String>) -> HttpResponse {
  let mut map = HashMap::from([
    ("category", "4"),
    ("title", &*form.topic_title),
    ("raw", &*form.topic_content),
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
    let response_topic_id = res_json.get("topic_id").unwrap().as_i64().unwrap_or(0);
    let reply_result_url = format!("https://{}/t/-/{}/", path.as_str(), response_topic_id);
    let data = json!({
      "hostname": path.as_str(),
      "topic_id": response_topic_id,
      "reply_result_url": reply_result_url,
    });
    let body = hb.render("success-do_discourse_new_topic", &data).unwrap();
    HttpResponse::Ok().content_type(ContentType::html()).body(body)
  } else {
    HttpResponse::InternalServerError().body(
      format!("API Request Failed with {}: {:?}", res.status().as_str(), res.text().await.unwrap()))
  }
}

#[get("/handle-reply-to-topic/{hostname}/")]
async fn handle_new_topic(hb: web::Data<Handlebars<'_>>, path: web::Path<String>) -> HttpResponse {
  let data = json!({
        "hostname": path.as_str(),
    });
  let body = hb.render("new-topic", &data).unwrap();

  HttpResponse::Ok().content_type(ContentType::html()).body(body)
}


#[get("/handle-reply-to-topic/{hostname}/{topic_id}/{tail:.*}")]
async fn handle_reply_topic(hb: web::Data<Handlebars<'_>>, path: web::Path<(String, String, String)>) -> HttpResponse {
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
  env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

  let mut handlebars = Handlebars::new();
  handlebars
    .register_templates_directory(".html", "templates")
    .unwrap();
  let handlebars_ref = web::Data::new(handlebars);
  let backend_customapi_general = InMemoryBackend::builder().build();
  let backend_reply = InMemoryBackend::builder().build();
  let backend_new_topic = InMemoryBackend::builder().build();

  HttpServer::new(move || {
    App::new()
      .wrap(error_handlers())
      .wrap(middleware::Logger::default())
      .app_data(handlebars_ref.clone())
      .service(
        web::scope("/xjtumen-custom-api")
          .wrap(RateLimiter::builder(backend_customapi_general.clone(), SimpleInputFunctionBuilder::new(Duration::from_secs(3600), 600)
            .peer_ip_key() // if use CDN, use `realip_remote_addr` instead
            // .path_key() // rate limit at path level, should not be set as it's easy to escape
            .build())
            .add_headers()
            .request_denied_response(move |_|
              HttpResponse::build(StatusCode::TOO_MANY_REQUESTS).insert_header(actix_web::http::header::ContentType::plaintext()).body(
                format!("为防滥用，{}s内仅能{}{}次，请稍后再试", 3600, "访问匿名API", 600)))
            .build())

          // .wrap(get_rate_limiter(60, 3, "尝试"))
          .service(handle_reply_topic)
          .service(handle_new_topic)
          .service(web::scope("/call-discourse-api")
            .service(
              web::scope("/new-topic")
                // TODO handle duplications of rate limit code
                .wrap(RateLimiter::builder(backend_new_topic.clone(), SimpleInputFunctionBuilder::new(Duration::from_secs(3600), 2)
                  .peer_ip_key() // if use CDN, use `realip_remote_addr` instead
                  // .path_key() // rate limit at path level, should not be set as it's easy to escape
                  .build())
                  .add_headers()
                  .request_denied_response(move |_|
                    HttpResponse::build(StatusCode::TOO_MANY_REQUESTS).insert_header(actix_web::http::header::ContentType::plaintext()).body(
                      format!("为防滥用，{}s内仅能{}{}次，请稍后再试", 3600, "尝试新建话题", 2))
                  )
                  .build())
                .service(do_discourse_new_topic)
            )
            .service(
              web::scope("/post-to-topic")
                .wrap(RateLimiter::builder(backend_reply.clone(), SimpleInputFunctionBuilder::new(Duration::from_secs(1800), 10)
                  .peer_ip_key() // if use CDN, use `realip_remote_addr` instead
                  // .path_key() // rate limit at path level, should not be set as it's easy to escape
                  .build())
                  .add_headers()
                  .request_denied_response(move |_|
                    HttpResponse::build(StatusCode::TOO_MANY_REQUESTS).insert_header(actix_web::http::header::ContentType::plaintext()).body(
                      format!("为防滥用，{}s内仅能{}{}次，请稍后再试", 1800, "尝试回复", 10)))
                  .build())
                .service(do_discourse_post_to_topic)
            )
          )
      )
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
