



// a protocol which provides a full-duplex communication and streaming
// channel over a single TCP connection, SSE can only push notifications 
// from server to the browser but WS is used to allow clients send messages 
// to the server in real time, we've used redis pubsub over WS for realtime
// streaming


pub mod v1;