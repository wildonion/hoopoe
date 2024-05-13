


use crate::{error::HoopoeErrorResponse, *};
use config::EnvExt;
use models::user::UserData;
use types::*;

use self::appstate::AppState;


// async methods and functions need to be Send in order to share them
// between threads for future solvation
#[trait_variant::make(PassportSend: Send)]
pub trait Passport{

    type Request;

    async fn get_user(&self) -> Result<UserData, HoopoeHttpResponse>;
    async fn get_secret(&self, app_state: web::Data<AppState>) -> Result<String, HoopoeHttpResponse>;
}

// only traits defined in the current crate can be implemented for types defined outside of the crate
// Passport trait is defined here and must be implemented in this crate for HttpRequest which is a type
// outside of this crate, we can't add the following codes in server or any other crates.
impl Passport for actix_web::HttpRequest{

    type Request = Self;

    async fn get_user(&self) -> Result<UserData, HoopoeHttpResponse>{

        let req = self; // self is a mutable reference to the HttpRequest object

        let jwt = "";
        let api = requests::Request::builder(
            "/health/check-token", 
            jwt,
            &String::from("")
        );

        todo!()
    }

    async fn get_secret(&self, app_state: web::Data<AppState>) -> Result<String, HoopoeHttpResponse>{

        let req = self;
        let config = app_state.config.as_ref().unwrap();
        let secret_key = &config.vars.SECRET_KEY;
        let actors = app_state.actors.as_ref().unwrap();
        let zerlog_producer_actor = actors.clone().producer_actors.zerlog_actor;


        let headers = req.headers();
        let get_secret_key = headers.get("secret_key");
        let Some(header_secret_key) = get_secret_key else{
            
            todo!()
        };

        if header_secret_key != secret_key{

        }

        todo!()

    }
}