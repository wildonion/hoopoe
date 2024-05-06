


use crate::*;
use config::EnvExt;
use models::user::UserData;
use types::*;


// async methods and functions need to be Send in order to share them
// between threads for future solvation
#[trait_variant::make(PassportSend: Send)]
pub trait Passport{

    type Request;

    async fn get_user(&self) -> Result<UserData, HoopoeHttpResponse>;
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
}