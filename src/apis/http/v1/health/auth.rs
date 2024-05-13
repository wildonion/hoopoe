


use interfaces::passport::Passport;

pub use super::*;



#[post("/generate-access-token")]
pub(self) async fn generate_access_token(
    req: HttpRequest,
    app_state: web::Data<AppState>
) -> HoopoeHttpResponse{
    
    match req.get_secret(app_state.clone()).await{
        Ok(sec) => {

            // 1 - generate an aes256 hash of current time + secret key 
            // 2 - return the hash with expiration time in meta

            todo!()            
        },
        Err(resp_err) => resp_err
    }

}

pub mod exports{
    pub use super::generate_access_token;
}