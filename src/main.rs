use std::io;
use std::time::Duration;

use actix_extensible_rate_limit::RateLimiter;
use actix_extensible_rate_limit::backend::SimpleInputFunctionBuilder;
use actix_extensible_rate_limit::backend::memory::InMemoryBackend;
use actix_web::{App,
                http::StatusCode,
                HttpResponse, HttpServer, middleware, web};
use handlebars::Handlebars;
use crate::routes::NewTopicForm;

mod read_request_body;
mod routes;
mod error_handling;

const XJTUMEN_URL_BASE: &str = "https://xjtu.men/posts.json";


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
      .wrap(error_handling::error_handlers())
      .wrap(middleware::Logger::default())
      .app_data(handlebars_ref.clone())
      // .app_data(web::Form::<NewTopicForm>::configure(|cfg| cfg.limit(256*1024)))
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
          .service(routes::handle_reply_topic)
          .service(routes::handle_new_topic)
          .service(web::scope("/call-discourse-api")
            .wrap(read_request_body::Logging)
            .service(
              web::scope("/new-topic")
                // TODO handle duplications of rate limit code
                .wrap(RateLimiter::builder(backend_new_topic.clone(), SimpleInputFunctionBuilder::new(Duration::from_secs(3600), 2)
                  .peer_ip_key() // if use CDN, use `realip_remote_addr` instead
                  .path_key() // rate limit at path level, should not be set as it's easy to escape
                  .build())
                  .add_headers()
                  .request_denied_response(move |_|
                    HttpResponse::build(StatusCode::TOO_MANY_REQUESTS).insert_header(actix_web::http::header::ContentType::plaintext()).body(
                      format!("为防滥用，{}s内仅能{}{}次，请稍后再试", 3600, "尝试新建话题", 2))
                  )
                  .build())
                .service(routes::do_discourse_new_topic)
            )
            .service(
              web::scope("/post-to-topic")
                .wrap(RateLimiter::builder(backend_reply.clone(), SimpleInputFunctionBuilder::new(Duration::from_secs(1800), 10)
                  .peer_ip_key() // if use CDN, use `realip_remote_addr` instead
                  .path_key() // rate limit at path level, should not be set as it's easy to escape
                  .build())
                  .add_headers()
                  .request_denied_response(move |_|
                    HttpResponse::build(StatusCode::TOO_MANY_REQUESTS).insert_header(actix_web::http::header::ContentType::plaintext()).body(
                      format!("为防滥用，{}s内仅能{}{}次，请稍后再试", 1800, "尝试回复", 10)))
                  .build())
                .service(routes::do_discourse_post_to_topic)
            )
          )
      )
  })
    .bind(("127.0.0.1", 7010))?
    .run()
    .await
}
