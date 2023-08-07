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

#[derive(Debug, Deserialize)]
pub struct PostToTopicForm {
  content: String,
  topic_id: String,
}

#[derive(Debug, Deserialize)]
pub struct NewTopicForm {
  topic_content: String,
  topic_title: String,
}


#[post("/{hostname}")]
async fn do_discourse_post_to_topic(hb: web::Data<Handlebars<'_>>, form: web::Form<PostToTopicForm>, path: web::Path<String>) -> HttpResponse {
  let mut map = HashMap::from([
    ("category", ""),
    ("title", ""),
    ("raw", &*form.content),
    ("topic_id", &*form.topic_id),
  ]);
  map.insert("body", "json");

  let client = reqwest::Client::new();
  let api_key_anonymous = env::var("DISCOURSE_API_KEY_ANONYMOUS").unwrap();

  let res = client.post(crate::XJTUMEN_URL_BASE)
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
async fn do_discourse_new_topic(hb: web::Data<Handlebars<'_>>, form: web::Form<NewTopicForm>, path: web::Path<String>) -> HttpResponse {
  let mut map = HashMap::from([
    ("category", "4"),
    ("title", &*form.topic_title),
    ("raw", &*form.topic_content),
  ]);
  map.insert("body", "json");

  let client = reqwest::Client::new();
  let api_key_anonymous = env::var("DISCOURSE_API_KEY_ANONYMOUS").unwrap();

  let res = client.post(crate::XJTUMEN_URL_BASE)
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