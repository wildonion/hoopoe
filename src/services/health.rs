




use crate::*;


// config is a mutable pointer to the web::ServiceConfig
// which makes to return nothing from each functions cause
// the state of the actual instance of web::ServiceConfig
// will be mutated in its scope

/*
     --------------------------------
    |     REGISTER HEALTH ROUTES
    | -------------------------------
    |
    |

*/
pub fn init(config: &mut web::ServiceConfig){

    config.service(apis::http::v1::health::index::exports::test_stream);
    config.service(apis::http::v1::health::index::exports::check);
    config.service(apis::http::v1::health::index::exports::mint_demo);

}