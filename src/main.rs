use actix_cors::Cors;
use actix_multipart::Multipart;
use actix_web::{http, web, App, Error, HttpRequest, HttpResponse, HttpServer};
use chrono::offset::Utc;
use chrono::Duration;
use futures::{StreamExt, TryStreamExt};
use http::StatusCode;
use serde_json::json;
use std::env;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

#[cfg(feature = "v2")]
use ring::signature::Ed25519KeyPair;

use ring::signature::{self, KeyPair};

// use ring::ec::curve25519::ed25519::signing::Ed25519KeyPair;

use dotenv;

extern crate rand;

use rand::distributions::Alphanumeric;
use rand::Rng;

use std::process::Command;

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
    let as_key = ring::signature::Ed25519KeyPair::from_pkcs8(
        "some_random_key_that_we'll_get_from_an_env".as_bytes(),
    )
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

use std::num::ParseIntError;

pub fn decode_hex(s: &str) -> Result<Vec<u8>, ParseIntError> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16))
        .collect()
}

use std::io::prelude::*;
fn generate_token(path: &str, duration: f64) -> Result<String, &str> {
    /*
         FOR FUTURE REFERENCE IF WE WANNA CREATE A NEW KEY
          let rng = ring::rand::SystemRandom::new();
          let pkcs8_bytes = signature::Ed25519KeyPair::generate_token(&rng).unwrap();
          let pkcs8_bytes_more: &[u8] = pkcs8_bytes.as_ref();
    */

    // TODO store this in a global env at the beginning
    let mut f = std::fs::File::open("key.txt").unwrap();
    let mut buff: Vec<u8> = vec![];
    f.read_to_end(&mut buff).unwrap();
    let as_key = ring::signature::Ed25519KeyPair::from_pkcs8(buff.as_ref()).unwrap();
    let exp = Utc::now() + Duration::days(1);
    match paseto::tokens::PasetoBuilder::new()
        .set_ed25519_key(as_key)
        .set_issued_at(Some(Utc::now()))
        .set_expiration(exp)
        .set_claim(String::from("path"), json!(path))
        .set_claim(String::from("duration"), json!(duration.to_string()))
        .set_footer(String::from("xd"))
        .build()
    {
        Ok(value) => Ok(value),
        Err(_) => Err("Error generating token"),
    }
}

async fn post_mp3(mut payload: Multipart) -> Result<HttpResponse, Error> {
    let mut filepathend = String::from("");
    let random_id = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(23)
        .collect::<String>();
    fs::create_dir_all(format!("assets/media/{}/hls", random_id))
        .expect("We need to handle this error");
    fs::create_dir_all(format!("tmp/{}", random_id)).expect("We need to handle this error");
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
            let token = generate_token(
                random_id.as_str(),
                file_handling::get_duration_from_hls(path_hls_str),
            );
            match token {
                Ok(value) => Ok(HttpResponse::Ok()
                    .body(json!({
                        "status": 200,
                        "message": "Success!",
                        "data": {
                            "token": value
                        }
                    }))
                    .into()),
                Err(_) => Ok(HttpResponse::build(StatusCode::NOT_FOUND)
                    .body("Error creating it!")
                    .into_body()),
            }
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

// For testing purposes and the sake of simplicity!
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
    dotenv::dotenv().unwrap();
    println!("{}", generate_token("xd", 39.0).unwrap());
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
