use actix_web::http::header::ContentType;
use actix_web::HttpResponse;
use actix_web_flash_messages::IncomingFlashMessages;
use std::fmt::Write;

pub async fn change_password_form(
    flash_messages: IncomingFlashMessages,
) -> Result<HttpResponse, actix_web::Error> {
    let mut flash_message_html = String::new();
    flash_messages.iter().for_each(|message| {
        writeln!(
            &mut flash_message_html,
            "<p><i>{}</i></p>",
            message.content()
        )
        .expect("Could not write flash message")
    });

    Ok(HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(format!(
            r#"
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta http-equiv="content-type" content="text/html; charset=utf-8">
                <title>Change Password</title>
            </head>
            <body>
                {flash_message_html}
                <form action="/admin/password" method="post">
                    <label>Current password
                        <input
                            type="password"
                            placeholder="Enter current password"
                            name="old_password"
                        >
                    </label><br/>
                    <label>New password
                        <input
                            type="password"
                            placeholder="Enter new password"
                            name="new_password"
                        >
                    </label><br/>
                    <label>Confirm new password
                        <input
                            type="password"
                            placeholder="Type the new password again"
                            name="new_password_confirm"
                        >
                    </label><br/>
                    <button type="submit">Change password</button>
                </form>
                <p><a href="/admin/dashboard">&lt;- Back</a></p>
            </body>
        </html>
        "#,
        )))
}
