use actix_cors::Cors;
use actix_multipart::Multipart;
use actix_web::{http, web, App, Error, HttpRequest, HttpResponse, HttpServer};
use futures::{StreamExt, TryStreamExt};
use http::StatusCode;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

extern crate rand;

use rand::distributions::Alphanumeric;
use rand::Rng;

use std::process::Command;

use ring::signature::Ed25519KeyPair;

extern crate paseto;

mod file_handling;

// use {
//   chrono::prelude::*,
//   ring::rand::SystemRandom,
//   ring::signature::Ed25519KeyPair,
//   serde_json::json,
// };

// #[warn(dead_code)]
fn validate_token(s: &str) -> Option<u64> {
    let footer = "";
    let as_key =
        Ed25519KeyPair::from_pkcs8("some_random_key_that_we'll_get_from_an_env".as_bytes())
            .expect("Failed to parse keypair");
    let verified = paseto::validate_public_token(
        &s,
        Some(footer),
        &paseto::tokens::PasetoPublicKey::ED25519KeyPair(as_key),
    );
    let val = match verified {
        Ok(value) => value,
        Err(_) => return None,
    };
    return match val.get("id") {
        Some(value) => value.as_u64(),
        _ => None,
    };
}

fn generate_token(path: &str, duration: f64) -> &str {
    ""
}

async fn post_mp3(mut payload: Multipart) -> Result<HttpResponse, Error> {
    let mut filepathend = String::from("");
    let random_id = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(10)
        .collect::<String>();
    fs::create_dir_all(format!("assets/media/{}/hls", random_id))
        .expect("We need to handle this error");
    fs::create_dir_all(format!("tmp/{}", random_id)).expect("We need to handle this error");
    // that will return the filepath for this...
    while let Ok(Some(mut field)) = payload.try_next().await {
        let content_type = field.content_disposition().unwrap();
        let filename = content_type.get_filename().unwrap();
        let filepath = format!("./tmp/{}/{}", random_id, filename);
        filepathend = filepath.to_string();
        // File::create is blocking operation, use threadpool
        let mut f = web::block(|| std::fs::File::create(filepath))
            .await
            .unwrap();
        // Field in turn is stream of *Bytes* object
        while let Some(chunk) = field.next().await {
            let data = chunk.unwrap();
            // filesystem operations are blocking, we have to use threadpool
            f = web::block(move || f.write_all(&data).map(|_| f)).await?;
        }
    }
    // ffmpeg -i "Feels Like Summer.mp3" -c:a aac -b:a 64k -vn -hls_list_size 0 -hls_time 20 output.m3u8
    let path_hls = format!("./assets/media/{}/hls/outputlist.m3u8", random_id);
    let path_hls_str = path_hls.as_str();
    let mut child = Command::new("ffmpeg")
        .args(&[
            "-i",
            filepathend.as_str(),
            "-c:a",
            "aac",
            "-b:a",
            "128k",
            "-vn",
            "-hls_list_size",
            "0",
            "-hls_time",
            "20",
            path_hls_str,
        ])
        .spawn()
        .expect("Error saving...");

    // Get the duration of the mp3

    match child.wait() {
        Ok(_) => {
            println!(
                "Duration: {}s",
                file_handling::get_duration_from_hls(path_hls_str)
            );
            Ok(HttpResponse::Ok().body("Success!").into())
        }
        Err(_) => Ok(HttpResponse::build(StatusCode::NOT_FOUND)
            .body("Error creating it!")
            .into_body()),
    }
}

async fn get_hls_file(req: HttpRequest) -> HttpResponse {
    // req.headers()
    println!("HITTED");
    fs::create_dir_all("assets/media");
    let (path, m_id): (PathBuf, PathBuf) = (
        req.match_info().query("filename").parse().unwrap(),
        req.match_info().query("m_id").parse().unwrap(),
    );
    let value = match path.to_str() {
        Some(val) => val,
        // todo: better handling
        _ => panic!("HANDLE XD"),
    };
    let base_path = format!(
        "assets/media/{}/hls",
        match m_id.to_str() {
            Some(val) => val,
            // todo: better handling
            _ => panic!("HANDLE."),
        }
    );
    let file_to_use = if value != "stream" {
        value
    } else {
        "outputlist.m3u8"
    };
    let complete_file = format!("{}/{}", base_path, file_to_use);
    println!("{}", complete_file);
    let file = match fs::read(complete_file) {
        Ok(f) => f,
        Err(_) => panic!("Error, not found"),
    };
    HttpResponse::Ok()
        .content_type("application/x-mpegURL")
        .body(file)
}

fn index() -> HttpResponse {
    let html = r#"<html>
        <head><title>Upload Test</title></head>
        <body>
            <form target="/" method="post" enctype="multipart/form-data">
                <input type="file" multiple name="file"/>
                <input type="submit" value="Submit"></button>
            </form>
        </body>
    </html>"#;

    HttpResponse::Ok().body(html)
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    println!("Done.");
    HttpServer::new(|| {
        App::new()
            .wrap(Cors::new().finish())
            .route("/media/{m_id}/{filename}", web::get().to(get_hls_file))
            .route("/", web::post().to(post_mp3))
            .route("/", web::get().to(index))
    })
    .bind("0.0.0.0:8088")?
    .run()
    .await
}
