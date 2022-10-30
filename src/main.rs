#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;

use rand::distributions::{Alphanumeric, DistString};
use rand::thread_rng;
use rocket::{form::Form, fs::NamedFile, fs::TempFile, http::Status, response::status, Config};
use std::{env, fs};

#[derive(FromForm)]
struct Upload<'r> {
    file: TempFile<'r>,
    #[field(default = "")]
    key: String,
}

#[post("/", data = "<upload>")]
async fn post_file(mut upload: Form<Upload<'_>>) -> status::Custom<String> {
    if env_use_key() && env_key() != upload.key {
        return status::Custom(
            Status::BadRequest,
            "key not found in the header".to_string(),
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
            return status::Custom(
                Status::InternalServerError,
                "Some stupid internal error occurred".to_string(),
            );
        }

        return status::Custom(Status::Ok, format!("{}/{}", env_user_url(), new_name));
    } else {
        return status::Custom(Status::BadRequest, "File name invalid".to_string());
    }
}

#[get("/<filename>")]
async fn get_file(filename: String) -> Option<NamedFile> {
    NamedFile::open(format!("{}/{}", env_root_dir(), filename))
        .await
        .ok()
}

#[get("/")]
fn index() -> String {
    format!(
        "Use curl to upload:\n\
         curl -F file=@\"[file]\" {url}\n\
         If key is enabled then a field \"key\" might be required in which case it would be\n\
         curl -F file=@\"[file]\" -F \"key=[key]\" {url}",
        url = env_user_url()
    )
}

fn env_root_dir() -> String {
    env::var("ROOT_DIR").unwrap_or("/var/files".to_string())
}

fn env_use_key() -> bool {
    env::var("USE_KEY")
        .unwrap_or("false".to_string())
        .parse::<bool>()
        .unwrap_or(false)
}

fn env_key() -> String {
    env::var("KEY").expect("KEY not set in the environment")
}

fn env_user_url() -> String {
    let default_config = Config::default();

    env::var("USER_URL").unwrap_or(format!(
        "http://{}:{}",
        default_config.address, default_config.port
    ))
}

#[launch]
fn rocket() -> _ {
    let cors = rocket_cors::CorsOptions::default().to_cors().unwrap();

    fs::create_dir_all(env_root_dir()).unwrap();
    println!("Starting");

    rocket::build()
        .attach(cors)
        .mount("/", routes![post_file, get_file, index])
}
