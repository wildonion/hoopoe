



// actor workers for rmq notif producers, consumers and db cqrs accessors/mutators
// at any time, we'll send them command to execute what we want during the app execution
/* 
    use actors for cron scheduling and check something in the background thread asyncluy and periodically
    use actors for keeping some state of the app isolated and safe from other parts and only update it by sending message 
    use actors for communication with other parts through the local jobq channel based like mpsc
    use actors to execute asynchronous tasks and jobs in the background lightweight thread of execution without blocking 
    the communication however can take place using mpsc in local and capnp rpc remotely to call methods of each actor worker object
*/


pub mod cqrs; // cqrs actor components
pub mod notif; // broker actor component
pub mod zerlog; // zerlog actor component