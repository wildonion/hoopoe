

/* -ˋˏ✄┈┈┈┈
    mutator api handlers and controllers 
                    |
                     ------messge(data)-----> mutator actros 
                                                |                          __
                                                 ------seaorm/query-----> |db|
                                                                           --
    accessor api handlers and controllers 
                    |
                    accessor_actor_none_associated_method()
                                                |                          __
                                                 ------seaorm/query-----> |db|
                                                                           --
*/
// http and websocket controllers
pub mod http;
pub mod ws;