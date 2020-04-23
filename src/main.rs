use actix_cors::Cors;
use actix_web::{web, App, HttpServer};
use dotenv;

extern crate paseto;
extern crate rand;

mod controllers;
pub mod file_handling;
pub mod token;

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    // validate_token("v2.public.eyJlbWFpbCI6ImV4YW1wbGVAZ21haWwuY29tIiwiZXhwIjoiMjAyMC0wNC0xOVQyMzoxMzozNCswMjowMCIsImlkIjoiMiJ982M6Qq3QaieYH0QUp2FqoODmdPbAzbNh8CaXvpU8ZPd783tX3R3DobSR3oyNFnAC4cJX3E_p9P0pB7Cx_mdbAA").unwrap();
    token::init("key.txt");
    println!("{}", token::Token::generate_token("xd", 39.0).unwrap());
    dotenv::dotenv().unwrap();
    // Print a token to see that it works
    HttpServer::new(|| {
        App::new()
            .wrap(Cors::new().finish())
            .configure(controllers::Controller::config)
    })
    .bind("0.0.0.0:8088")?
    .run()
    .await
}
