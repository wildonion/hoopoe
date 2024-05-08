


pub use super::*;


#[get("/hoop/get/")]
pub(self) async fn get_hoop(
    req: HttpRequest,
    app_state: web::Data<AppState>,
) -> HoopoeHttpResponse{

    // get hoop by sending message to accessor 
    // then fetch from redis
    // ...
    
    todo!()

}

#[get("/notif/get/")]
pub(self) async fn get_notif(
    req: HttpRequest,
    app_state: web::Data<AppState>,
) -> HoopoeHttpResponse{

    // get notif by sending message to notif accessor
    // then fetch from redis
    // ...
    
    todo!()

}

pub mod exports{
    pub use super::get_hoop;
    pub use super::get_notif;
}