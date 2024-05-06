

use crate::*;


// config is a mutable pointer to the web::ServiceConfig
// which makes to return nothing from each functions cause
// the state of the actual instance of web::ServiceConfig
// will be mutated in its scope

/*
     --------------------------------
    |        REGISTER WS ROUTES
    | -------------------------------
    |
    |

*/
pub fn init(config: &mut web::ServiceConfig){

    config.service(apis::ws::v1::hoop::exports::index);

}