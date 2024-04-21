use crate::session_state::TypedSession;
use crate::utils;
use actix_web::HttpResponse;
use actix_web_flash_messages::FlashMessage;

pub async fn logout(session: TypedSession) -> Result<HttpResponse, actix_web::Error> {
    session.logout();
    FlashMessage::info("You have successfully logged out").send();
    Ok(utils::see_other("/login"))
}
