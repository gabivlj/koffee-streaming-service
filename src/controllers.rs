use actix_multipart::Multipart;
use actix_web::{http, web, Error, HttpRequest, HttpResponse};
use futures::{StreamExt, TryStreamExt};
use http::StatusCode;
use rand::distributions::Alphanumeric;
use rand::Rng;
use serde_json::json;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;

use super::file_handling;
use super::token;

pub struct Controller;

impl Controller {
    pub async fn post_mp3(req: HttpRequest, mut payload: Multipart) -> Result<HttpResponse, Error> {
        let auth = match req.headers().get("Authorization") {
            Some(value) => value.to_str(),
            None => {
                return Ok(HttpResponse::build(StatusCode::UNAUTHORIZED)
                    .body("Unauthorized")
                    .into_body())
            }
        };
        if let Err(_) = auth {
            return Ok(HttpResponse::build(StatusCode::UNAUTHORIZED)
                .body("Unauthorized")
                .into_body());
        };
        let auth = auth.unwrap();
        if let None = token::Token::validate_token(auth) {
            return Ok(HttpResponse::build(StatusCode::UNAUTHORIZED)
                .body("Unauthorized")
                .into_body());
        }
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
                let token = token::Token::generate_token(
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

    pub fn index() -> HttpResponse {
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

    pub async fn get_hls_file(req: HttpRequest) -> HttpResponse {
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

    pub fn config(cfg: &mut web::ServiceConfig) {
        cfg.service(web::resource("/streaming"))
            .route(
                "/media/{m_id}/{filename}",
                web::get().to(Controller::get_hls_file),
            )
            .route("/", web::get().to(Controller::index))
            .route("/", web::post().to(Controller::post_mp3));
    }
}
