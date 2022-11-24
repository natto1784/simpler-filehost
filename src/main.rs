#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;

use rand::{
    distributions::{Alphanumeric, DistString},
    thread_rng,
};
use rocket::{
    form::Form,
    fs::{NamedFile, TempFile},
    http::Status,
    response::content::RawHtml,
    Config,
};
use std::{env, fs};

#[derive(FromForm)]
struct Upload<'r> {
    file: TempFile<'r>,
    #[field(default = "")]
    key: String,
    #[field(default = false)]
    custom: bool,
}

#[derive(Responder)]
enum RData {
    Raw(String),
    Html(RawHtml<String>),
}

#[post("/", data = "<upload>")]
async fn post_file(mut upload: Form<Upload<'_>>) -> (Status, RData) {
    let key = env_key();
    if !key.is_empty() && key != upload.key {
        return (
            Status::BadRequest,
            RData::Raw(String::from("key not found in the header")),
        );
    }

    if let Some(name) = upload.file.raw_name() {
        let new_name = format!(
            "{}-{}",
            Alphanumeric.sample_string(&mut thread_rng(), 4),
            name.dangerous_unsafe_unsanitized_raw()
        );

        let uploaded = upload
            .file
            .copy_to(format!("{}/{}", env_root_dir(), new_name))
            .await;

        if let Err(error) = uploaded {
            println!("Error while copying from temp file: {:?}", error);
            return (
                Status::InternalServerError,
                RData::Raw(String::from("Some stupid internal error occurred")),
            );
        }

        let file_url = format!("{}/{}", env_user_url(), new_name);

        if upload.custom {
            return (
                Status::SeeOther,
                RData::Html(RawHtml(format!(
                    r#"Here is your file: <a href="{url}">{url}</a>"#,
                    url = file_url
                ))),
            );
        }

        return (Status::Ok, RData::Raw(file_url));
    } else {
        return (
            Status::BadRequest,
            RData::Raw(String::from("File name invalid")),
        );
    }
}

#[get("/<filename>")]
async fn get_file(filename: String) -> Option<NamedFile> {
    NamedFile::open(format!("{}/{}", env_root_dir(), filename))
        .await
        .ok()
}

#[get("/")]
fn index() -> RawHtml<String> {
    RawHtml(format!(
        r#"
<html>

<head>
  <title>
    {title}
  </title>
</head>

<body>
  <p>
    Use curl to upload:
    <br>
    <code>
    curl -F file=@"[file]" {user_url}
    </code>
    <br>
    If key is enabled then a field "key" might be required in which case it would be
    <br>
    <code>
     curl -F file=@"[file]" -F "key=[key]" {user_url}
    </code>
  </p>
  <form method="POST" enctype="multipart/form-data">
      <label for="key">Key: </label>
      <input type="text" id="key"> <br>
      <input type="file" name="file" id="file">
      <input type="hidden" name="custom" id="true">
      <input type="submit" value="Upload">
  </form>

</body>

</html>
"#,
        title = env_title(),
        user_url = env_user_url()
    ))
}

fn env_root_dir() -> String {
    env::var("ROOT_DIR").unwrap_or(String::from("/var/files"))
}

fn env_key() -> String {
    env::var("KEY").unwrap_or(String::new())
}

fn env_user_url() -> String {
    let default_config = Config::default();

    env::var("USER_URL").unwrap_or(format!(
        "http://{}:{}",
        default_config.address, default_config.port
    ))
}

fn env_title() -> String {
    env::var("TITLE").unwrap_or(String::from("Simpler Filehost"))
}

fn env_cors() -> bool {
    env::var("USE_CORS")
        .unwrap_or(String::from("false"))
        .parse::<bool>()
        .unwrap_or(false)
}

#[launch]
fn rocket() -> _ {
    let cors = rocket_cors::CorsOptions::default().to_cors().unwrap();

    fs::create_dir_all(env_root_dir()).unwrap();
    println!("Starting");

    if env_cors() {
        rocket::build()
            .attach(cors)
            .mount("/", routes![post_file, get_file, index])
    } else {
        rocket::build().mount("/", routes![post_file, get_file, index])
    }
}
