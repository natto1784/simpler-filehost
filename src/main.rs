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
    response::Redirect,
    Config,
};
use rocket_dyn_templates::{context, Template};
use std::{env, fs};

#[derive(FromForm)]
struct Upload<'r> {
    file: TempFile<'r>,
    #[field(default = "")]
    key: String,
    #[field(default = false)]
    redirect: bool,
}

#[derive(Responder)]
enum RData {
    Raw(String),
    Redir(Redirect),
}

#[post("/", data = "<upload>")]
async fn post_file(mut upload: Form<Upload<'_>>) -> (Status, RData) {
    if env_use_key() && env_key() != upload.key {
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

        if upload.redirect {
            return (
                Status::SeeOther,
                RData::Redir(Redirect::to(file_url)),
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
fn index() -> Template {
    Template::render("index", context! {user_url: env_user_url()})
}

fn env_root_dir() -> String {
    env::var("ROOT_DIR").unwrap_or(String::from("/var/files"))
}

fn env_use_key() -> bool {
    env::var("USE_KEY")
        .unwrap_or(String::from("false"))
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
            .attach(Template::fairing())
            .mount("/", routes![post_file, get_file, index])
    } else {
        rocket::build()
            .attach(Template::fairing())
            .mount("/", routes![post_file, get_file, index])
    }
}
