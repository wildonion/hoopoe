


pub use super::*;



#[get("/location/stats/get/basic/")]
pub(self) async fn get_basic_report(
    req: HttpRequest,
    report_info: web::Query<ReportQuery>,
    app_state: web::Data<AppState>,
) -> HoopoeHttpResponse{

    let zerlog_producer_actor = app_state.as_ref().actors.clone().unwrap().producer_actors.zerlog_actor;
    let redis_exp_time = app_state.config.as_ref().unwrap().vars.REDIS_SESSION_EXP_KEY.parse::<u64>().unwrap();
    let storage = app_state.app_storage.as_ref().unwrap();
    let db = storage.get_seaorm_pool().await.unwrap();
    let redis_pool = storage.get_redis_pool().await.unwrap();
    
    let actors = app_state.actors.as_ref().unwrap();
    let accessor_actor = actors.clone().cqrs_actors.accessors.location_accessor_actor;

    let ReportQuery{ imei, from, to } = report_info.0; // unpacking

    /* -ˋˏ✄┈┈┈┈
        fetching report by talking to the actor itself takes too much times for us 
        since actors don't support async handlers which enforces us to spawn async 
        task in the background like by using tokio::spawn, this solution makes the 
        situation very hard for us cause in order to return the response of an async
        method we must either use channels or cache it in redis. either way is too
        slow cause in the first way we couldn't use none async channels inside an 
        async code because it blocks the thread so we can't use std::sync::mpsc in
        tokio::spawn to send the result of async method to outside scope, the second
        way on the other hand is also not a suitable solution cause storing data in
        redis takes an approximate time of 1 second which is too slow in production
        cause we should wait 1 second to get the data in here!
        therefore having accessors as an actor to fetch data from 
        db is a dead solution
    */
    match redis_pool.get().await{
        Ok(mut redis_conn) => {

            // try to read data from redis cache, if the key is not there means the key is expired
            let basic_report_key = format!("basic_report_for:{}_from:{}_to:{}", &imei.as_ref().unwrap_or(&String::from("")), &from.as_ref().unwrap_or(&String::from("")), &to.as_ref().unwrap_or(&String::from("")));
            let is_data_stored: bool = redis_conn.exists(&basic_report_key).await.unwrap();
            let basic_report = if is_data_stored{

                let get_basic_report_from_redis: String = redis_conn.get(basic_report_key).await.unwrap();
                let decoded_report = serde_json::from_str::<FetchLocationBasicReport>(&get_basic_report_from_redis).unwrap();
                Some(decoded_report)
                
            } else{ // otherwise fetch from db and create a new expirable key again

                let basic_report = LocationAccessorActor::raw_get_basic_report_none_associated(
                    redis_exp_time, &imei.as_ref().unwrap_or(&String::from("")), &from.as_ref().unwrap_or(&String::from("")), 
                    &to.as_ref().unwrap_or(&String::from("")), db, redis_pool, zerlog_producer_actor.clone()
                )
                .await;
                basic_report
            };

            // we don't return &String for the resp_msg cause creating String and returning 
            // pointer to them at the same time is not valid since String will be dropped 
            // at the end of the scope and taking a pointer to them brings us invalid pointer
            let resp_msg = if basic_report.is_none(){ format!("ERROR: check the logs") } else { format!("SUCCESS: basic report fetched") };
            let status = if basic_report.is_none(){ StatusCode::NOT_ACCEPTABLE } else { StatusCode::OK };

            resp!{
                Option<FetchLocationBasicReport>,
                basic_report,
                None, // metadata
                &resp_msg,
                status,
                None::<Cookie<'_>>,
            }
        },
        Err(e) => {
            let source = &e.source().unwrap().to_string();
            let err_instance = crate::error::HoopoeErrorResponse::new(
                *STORAGE_IO_ERROR_CODE, // error hex (u16) code
                source.as_bytes().to_vec(), // text of error source in form of utf8 bytes
                crate::error::ErrorKind::Storage(error::StorageError::RedisPool(e)), // the actual source of the error caused at runtime
                &String::from("get_basic_report.redis_pool"), // current method name
                Some(&zerlog_producer_actor)
            ).await;
            return Ok(err_instance.error_response()); // it returns the error in form of actix http response

        }
    }

}


#[get("/location/stats/get/general/")]
pub(self) async fn get_general_report(
    req: HttpRequest,
    report_info: web::Query<ReportQuery>,
    app_state: web::Data<AppState>,
) -> HoopoeHttpResponse{


    // try to load all messages related to the passed in imei from redis cache
    // if the key is expired then fetch from db
    // ...
    
    todo!()

}


#[get("{_:/?}")] // eg: /events, /events/123, /events/abc -> fetching event
pub(self) async fn sse0_feed(
    req: HttpRequest,
    msg: web::Path<String>,
    app_state: web::Data<AppState>,
) -> HoopoeHttpResponse{

    // pageNumber
    // pageSize
    // "totalItems": 51,
    // "totalPages": 11,
    // "currentPage": 2,
    // "pageSize": 5
    // use seaorm paginate

    // send realtime data from redis cache and timescaledb
    // cach complex data with expirable key fetched from multiple tables in redis
    // ... 

    todo!()
}

#[get("/{msg}")]
pub(self) async fn sse1_feed(
    req: HttpRequest,
    msg: web::Path<String>,
    app_state: web::Data<AppState>,
) -> HoopoeHttpResponse{


    // send realtime data from redis cache and timescaledb
    // cach complex data with expirable key fetched from multiple tables in redis
    // ...

    todo!()
}

pub mod exports{
    pub use super::get_basic_report;
    pub use super::sse1_feed;
    pub use super::sse0_feed;
}