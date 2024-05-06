


use serde::{Serialize, Deserialize};


/*  -----====-----====-----====-----====-----====-----====-----====-----====-----====-----====
    resp object macro, the most important section in the code the following facitilate sending 
    data back to the client by building a respone object every time the server wants to send data 
    back to the client, the macro however gets called from where the server is creating data to 
    send it, injecting headers and cookies logics must goes here, since this is macro the logic 
    will be built at compile time and once the api gets executed its body and all the data inside 
    will be dropped out of the ram even the response object which has been created this is good 
    quite frankly since by dropping all the data in the api Rust makes some space inside the heap 
    and clean extra allocation which helps having control on ram overhead, we owe Rust due to not 
    having gc rules. we can define as many as response object since once the scope or method or 
    the match arm gets executed the lifetime of the response object will be dropped from the ram 
    due to the fact that rust doesn't have gc :) 
    -----====-----====-----====-----====-----====-----====-----====-----====-----====-----====
*/
#[derive(Serialize, Deserialize, Debug)]
pub struct Response<'m, T>{
    pub data: Option<T>,
    pub message: &'m str, // &str are a slice of String thus they're behind a pointer and every pointer needs a valid lifetime which is 'm in here 
    pub status: u16,
    pub is_error: bool,
    pub meta: Option<serde_json::Value>
}

#[macro_export]
macro_rules! resp {
    (   
        $data_type:ty, // response type
        $data:expr, // response data
        $metadata:expr, // metadata
        $msg:expr, // response message
        $code:expr, // response code
        $cookie:expr, // response cookie
    ) => {

        {
            use models::http::Response;
            use actix_web::HttpResponse;
            use actix_web::http::header::Expires;
            use std::time::{SystemTime, Duration};
            
            let code = $code.as_u16();
            let mut res = HttpResponse::build($code);

            // building the response object
            let response_data = Response::<$data_type>{
                data: Some($data),
                message: $msg,
                status: code,
                is_error: if code == 200 || code == 201 || code == 302{
                    false
                } else{
                    true
                },
                meta: $metadata
            };
            
            // response expiration in client, the Expire gives the date/time after 
            // which the response is considered stale.
            let expiration = SystemTime::now() + Duration::from_secs(60); 
            // let redirect_url = format!("{}?data={}", state, jwt);
            let resp = if let Some(cookie) = $cookie{
                res
                    .cookie(cookie.clone())
                    .append_header(("cookie", cookie.value()))
                    .insert_header(Expires(expiration.into()))
                    // --------------- redirect client setup ---------------
                    // .status(StatusCode::TEMPORARY_REDIRECT)
                    // .append_header((actix_web::http::header::LOCATION, redirect_url))
                    // -----------------------------------------------------
                    .json(
                        response_data
                    )
            } else{
                res
                    .insert_header(Expires(expiration.into()))
                    // --------------- redirect client setup ---------------
                    // .status(StatusCode::TEMPORARY_REDIRECT)
                    // .append_header((actix_web::http::header::LOCATION, redirect_url))
                    // -----------------------------------------------------
                    .json(
                        response_data
                    )
            }; 

            return Ok(resp);
        }
    }
}