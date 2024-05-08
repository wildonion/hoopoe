


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

    config.service(apis::http::v1::events::set::exports::register_notif);
    config.service(apis::http::v1::events::get::exports::get_hoop);
    config.service(apis::http::v1::events::get::exports::get_notif);

}