use std::collections::HashMap;
use std::env;

use actix_web::{get,
                http::header::ContentType,
                HttpResponse, post, web};
use handlebars::Handlebars;
use log::error;
use reqwest::StatusCode;
use serde::Deserialize;
use serde_json::json;

#[derive(Debug, Deserialize)]
pub struct PostToTopicForm {
  content: String,
  topic_id: String,
}

#[derive(Debug, Deserialize)]
pub struct NewTopicForm {
  category: String,
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
  let api_key_anonymous = env::var("DISCOURSE_API_KEY_ANONYMOUS").expect("DISCOURSE_API_KEY_ANONYMOUS not set");

  let res = client.post(crate::XJTUMEN_URL_BASE)
    .header("Accept", "application/json; charset=utf-8")
    .header("Api-Key", api_key_anonymous)
    .header("Api-Username", "anonymous_user")
    .json(&map)
    .send()
    .await.unwrap();
  // println!("{}", res.status());
  if res.status().is_success() {
    let res_json = res.json::<serde_json::Value>().await;
    if let Err(e) = res_json {
      error!("discourse responded with invalid json");
      error!("{}", e);
      let data = json!({
            "request_method":"POST",
            "request_uri": format!("{}", path.as_str()),
            "error": format!("{}", e),
            "status_code": "",
            "error_info": format!("{} {}", form.topic_id, form.content)
            });
      let body = hb.render("error", &data);
      if let Err(e) = body {
        return HttpResponse::build(StatusCode::INTERNAL_SERVER_ERROR)
          .content_type(ContentType::plaintext())
          .body(format!("{}", e));
      } else {
        return HttpResponse::build(StatusCode::INTERNAL_SERVER_ERROR)
          .content_type(ContentType::html())
          .body(body.unwrap());
      }
    }
    let res_json = res_json.unwrap();
    // println!("{:?}", res_json);
    let response_post_id = res_json.get("post_number");
    if let None = response_post_id {
      return HttpResponse::build(StatusCode::INTERNAL_SERVER_ERROR)
        .content_type(ContentType::html())
        .body("failed to get post number from returned json");
    }
    let response_post_id = response_post_id.unwrap().as_i64().unwrap_or(0);
    let reply_result_url = format!("https://{}/t/-/{}/{}", path.as_str(), form.topic_id, response_post_id);
    let data = json!({
    "hostname": path.as_str(),
      "topic_id": form.topic_id,
      "reply_result_url": reply_result_url
    });
    let body = hb.render("reply-succeeded", &data).unwrap();
    HttpResponse::Ok().content_type(ContentType::html()).body(body)
  } else {
    HttpResponse::InternalServerError().body(
      format!("API Request Failed with {}: {:?}", res.status().as_str(), res.text().await.unwrap()))
  }
}


#[post("/{hostname}")]
async fn do_discourse_new_topic(hb: web::Data<Handlebars<'_>>, form: web::Form<NewTopicForm>, path: web::Path<String>) -> HttpResponse {
  println!("{:?}", form);
  let mut map = HashMap::from([
    ("category", &*form.category),
    ("title", &*form.topic_title),
    ("raw", &*form.topic_content),
  ]);
  map.insert("body", "json");

  let client = reqwest::Client::new();
  let api_key_anonymous = env::var("DISCOURSE_API_KEY_ANONYMOUS").expect("DISCOURSE_API_KEY_ANONYMOUS not set");

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
    let body = hb.render("new-topic-succeeded", &data).unwrap();
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