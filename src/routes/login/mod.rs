mod post;
pub use post::login;

use actix_web::http::header::ContentType;
use actix_web::HttpResponse;

pub async fn login_form() -> HttpResponse {
    HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(include_str!("login.html"))
}
