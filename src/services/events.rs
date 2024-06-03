


use crate::*;


// config is a mutable pointer to the web::ServiceConfig
// which makes to return nothing from each functions cause
// the state of the actual instance of web::ServiceConfig
// will be mutated in its scope


/*
     --------------------------------
    |     REGISTER EVENTS ROUTES
    | -------------------------------
    |
    |

*/
pub fn init(config: &mut web::ServiceConfig){

    config.service(apis::http::v1::events::notif::exports::get_notif);
    config.service(apis::http::v1::events::hoop::exports::get_hoop);
    config.service(apis::http::v1::events::hoop::exports::add_hoop);
    config.service(apis::http::v1::events::hoop::exports::update_hoop);
    config.service(apis::http::v1::events::hoop::exports::delete_hoop);
    config.service(apis::http::v1::events::hoop::exports::delete_hoop_by_owner);
    
    config.service(apis::http::v1::events::notif::exports::register_notif);
    config.service(apis::http::v1::events::notif::exports::get_notif);

}