



use crate::*;
use apis::v1::http::hoop::register_controller as hoop_router;
use apis::v1::http::health::register_controller as health_router;
use apis::v1::http::auth::register_controller as auth_router;
use apis::v1::http::notif::register_controller as notif_router;
use apis::v1::ws::notif::register_controller as notif_ws_router;

pub fn register_app_controllers() -> Router{
    Router::new()
        .push(hoop_router())
        .push(health_router())
        .push(auth_router())
        .push(notif_router())
        .push(notif_ws_router())
        
}