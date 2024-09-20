

## -ˋˏ✄┈┈┈┈ how amqp (rmq) works?

> refer to [this](https://www.cloudamqp.com/blog/part4-rabbitmq-for-beginners-exchanges-routing-keys-bindings.html#:~:text=The%20routing%20key%20is%20a%20message%20attribute%20added%20to%20the,routing%20key%20of%20the%20message.) for more comprehensive concepts.

`channel[producer] ----message----> broker[exchange[i]] ----message|>keys[i]----> queues[i]`
`channel[consumer]` <----message---- queue <-----routing key------ broker[exchange[i]]

### channels

every protocol operation happends on a channel like declare topology, publish, consume. channels are something like local mpcs jobq channels but they're inside a single TCP connection and are created per each thread cause they're not object safe thread or they're not bounded to `Send` and `Sync` traits in other words a connection is a TCP connection between your application and the RabbitMQ broker, a channel is a virtual and lightweight connection (per thread) inside a connection, a channel multiplexes a TCP connection, typically, each process only creates one TCP connection, and uses multiple channels in that connection for different threads. 

### routing to queues from the exchange!

first we need to declare a queue for the messages we want to consume them after that we must bind the declared queue to an exchange (the way we want to produce and consume messages) over an sepecific key, when the producer produces the payload it sends the payload in a channel to an specific exchange specified by the routing key, then routing key knows where these messages must be sent to which queue cause the queue is already bounded to the exchange routing key. exchanges are the way of transferring message based on an specific routing key. routing key is used to route the message matches the key to the queue which is bounded to the exchange routing key, this helps a lot since there might be multiple different queues bounded to the same exchange but with different routing keys, rmq broker knows how to route and what messages must be sent to which queue with this logic.

a producer never sends a message directly to a queue, instead it uses an exchange as a routing mediator, therefore exchange facilitate the routing of messages to queues based on defined rules, rmq supports fanout, direct, topic and headers exchanges, typically queue can be bounded into the exchange then exchange uses routing key as an address to decide how to route the message, a message goes to the queue with the binding key that is exactly matches the routing key. since we might have more than one queue, a queue gets bounded to an exchange using a binding key which is called routing key and is a unique key used to tell the exchange that use this pattern to route the message to a queue that is already bounded to this binding key:

> an exchange can send messages to a queue that is bounded to it already, in a multiple different ways based on routing key, in other words multiple queues can receive messages from a single exchange, each of which in a different way only using a routing key, each queue receives different messages than each other.

**direct exchange** | "": for the direct exchange the routing key is the same as the queue name (cause it's direct sending!), rmq doesn't allow to bind a queue to an exchange of type direct.

**fanout exchange** | "amq.fanout": a fanout exchange copies and routes a received message to all queues that are bounded to it regardless of routing keys or pattern matching as with direct and topic exchanges, the keys provided will simply be ignored.

**topic exchange** | "amq.topic" : it's a PubSub pattern in rmq! topic exchanges route messages to queues based on wildcard matches between the routing key and the routing pattern, which is specified by the queue binding. messages are routed to one or many queues based on a matching between a message routing key and this pattern in other words all messages with a routing key that match the routing pattern are routed to the queue and stay there until the consumer consumes the message. unlike fanout and direct patterns in this pattern the messge won't get poped out of the queue after it's consumed this behavior allows multiple consumers to receive the same message from the exchange like a real PubSub pattern.

when a queue is bound with `#` (hash) binding key - it will receive all the messages, regardless of the routing key - like in fanout exchange. when special characters `*` (star) and `#` (hash) aren't used in bindings, the topic exchange will behave just like a direct one.

**header exchange** | "amq.headers": a headers exchange routes messages based on arguments containing headers and optional values. headers exchanges are very similar to topic exchanges, but route messages based on header values instead of routing keys. a message matches if the value of the header equals the value specified upon binding. a special argument named "x-match", added in the binding between exchange and queue, specifies if all headers must match or just one. either any common header between the message and the binding count as a match, or all the headers referenced in the binding need to be present in the message for it to match. the "x-match" property can have two different values: "any" or "all", where "all" is the default value. a value of "all" means all header pairs (key, value) must match, while value of "any" means at least one of the header pairs must match. headers can be constructed using a wider range of data types, integer or hash for example, instead of a string. The headers exchange type (used with the binding argument "any") is useful for directing messages which contain a subset of known (unordered) criteria.

### what is happening in each queue?

the messages stay in the queue until they are handled by a consumer. if a queue is bounded to an exchange routing key and the consumer is not up yet, all the messages get accumulated inside the queue until the consumer get back online and starts consuming them.

## Final words

we can declare as much as queue per each consumer we want and bind it to an specific exchange routing key this logic enables each consumer consume message coming from an exchange from its own queue cause the queue is already bounded to that exchange routing key generally any queue that is bounded to an specific routing key will receive the messages coming from that exchange for any exchange pattern build a queue for each consumer and bind it to the routing key cause routing key specifies to which queue message must be sent and it works if there are multiple queues with different names cause they are bounded to the same routing key change.

## Handling RPC Communication with RMQ

To handle **RPC (Remote Procedure Call)** queues in RabbitMQ using **`deadpool-lapin`** in Rust, you'll need to follow a few steps. RabbitMQ supports RPC via the use of a **"reply-to" queue** where a client sends a message with the expectation of receiving a response on that queue.

In RabbitMQ, the basic flow for **RPC** involves:
1. The **client** sends a message to a queue and specifies a **reply-to** queue.
2. The **server** consumes the message, processes it, and sends a response back to the **reply-to** queue specified by the client.
3. The **client** listens for the response on the **reply-to** queue.

Here’s how you can implement an RPC queue handler using `deadpool-lapin` in Rust.

### **Step 1: Set Up Deadpool-Lapin in `Cargo.toml`**
Make sure you have the following dependencies in your `Cargo.toml` file:

```toml
[dependencies]
deadpool-lapin = "0.11"
lapin = "2.0" # or the latest version of lapin
tokio = { version = "1", features = ["full"] }
futures = "0.3"
```

### **Step 2: RabbitMQ RPC Flow Using `deadpool-lapin`**

#### **Client Side: Sending the RPC Request**
The client sends a message with a `reply-to` property so that the server knows where to send the response.

Here’s an example of how you can implement an RPC client using `deadpool-lapin`:

```rust
use deadpool_lapin::{Config, Pool};
use lapin::{
    options::{BasicPublishOptions, QueueDeclareOptions, BasicConsumeOptions, BasicAckOptions},
    BasicProperties, message::DeliveryResult, Channel, types::FieldTable,
};
use tokio_amqp::LapinTokioExt;
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Step 1: Configure and create the RabbitMQ pool
    let config = Config::default();
    let pool = config.create_pool(Some(tokio_amqp::LapinTokioConnectionManager)).unwrap();
    let connection = pool.get().await?;

    // Step 2: Open a channel and declare the queues
    let channel = connection.create_channel().await?;
    let request_queue = "rpc_queue";

    // Declare the queue (ensure the queue exists)
    let _ = channel
        .queue_declare(
            request_queue,
            QueueDeclareOptions::default(),
            FieldTable::default(),
        )
        .await?;

    // Step 3: Declare a temporary queue for the reply-to messages (auto-delete)
    let reply_queue = channel
        .queue_declare(
            "",
            QueueDeclareOptions {
                exclusive: true,
                auto_delete: true,
                ..QueueDeclareOptions::default()
            },
            FieldTable::default(),
        )
        .await?;

    let correlation_id = uuid::Uuid::new_v4().to_string(); // Use a unique correlation ID

    // Step 4: Publish the request message
    let request_payload = b"Request Payload";

    channel
        .basic_publish(
            "", // "" means direct exchange
            request_queue, // in direct exchange routing key is the same as the queue name
            BasicPublishOptions::default(),
            request_payload.to_vec(),
            BasicProperties::default()
                .with_reply_to(reply_queue.name().as_bytes().to_vec()) // Set reply-to queue
                .with_correlation_id(correlation_id.clone().into()),
        )
        .await?;

    println!("Request sent. Waiting for the response...");

    // Step 5: Consume the reply message from the reply-to queue
    let mut consumer = channel
        .basic_consume(
            reply_queue.name().as_str(),
            "",
            BasicConsumeOptions::default(),
            FieldTable::default(),
        )
        .await?;

    while let Some(Ok(delivery)) = consumer.next().await {
        if let Some(corr_id) = delivery.properties.correlation_id().as_ref() {
            if corr_id.as_str() == correlation_id {
                // Response received with the matching correlation_id
                println!("Received RPC response: {:?}", String::from_utf8_lossy(&delivery.data));
                delivery.ack(BasicAckOptions::default()).await.unwrap();
                break;
            }
        }
    }

    Ok(())
}
```

#### **Server Side: Processing the Request and Sending the Response**
The server listens for requests on the queue, processes the messages, and sends a response back to the client using the `reply-to` queue specified by the client.

Here’s an example of how to implement the RPC server:

```rust
use deadpool_lapin::{Config, Pool};
use lapin::{
    options::{BasicAckOptions, BasicPublishOptions, BasicConsumeOptions, QueueDeclareOptions},
    BasicProperties, Channel, message::DeliveryResult, types::FieldTable,
};
use tokio_amqp::LapinTokioExt;
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Step 1: Configure and create the RabbitMQ pool
    let config = Config::default();
    let pool = config.create_pool(Some(tokio_amqp::LapinTokioConnectionManager)).unwrap();
    let connection = pool.get().await?;

    // Step 2: Open a channel and declare the queue
    let channel = connection.create_channel().await?;
    let request_queue = "rpc_queue";

    let _ = channel
        .queue_declare(
            request_queue,
            QueueDeclareOptions::default(),
            FieldTable::default(),
        )
        .await?;

    // Step 3: Start consuming messages from the queue
    let mut consumer = channel
        .basic_consume(
            request_queue,
            "",
            BasicConsumeOptions::default(),
            FieldTable::default(),
        )
        .await?;

    while let Some(Ok(delivery)) = consumer.next().await {
        let reply_to = delivery.properties.reply_to().clone();
        let correlation_id = delivery.properties.correlation_id().clone();

        // Process the message (in this case, we just echo back the received payload)
        println!("Received RPC request: {:?}", String::from_utf8_lossy(&delivery.data));

        // Step 4: Send the response back to the reply-to queue
        if let Some(reply_to) = reply_to {
            let response_payload = b"Response Payload";
            channel
                .basic_publish(
                    "", // "" means direct exchange
                    reply_to.as_str(), // in direct exchange routing key is the same as the queue name 
                    BasicPublishOptions::default(),
                    response_payload.to_vec(),
                    BasicProperties::default()
                        .with_correlation_id(correlation_id), // Attach the same correlation ID
                )
                .await?;
        }

        // Acknowledge the message
        delivery.ack(BasicAckOptions::default()).await?;
    }

    Ok(())
}
```

### **Explanation of Key Components**

1. **Reply-to Queue**:
   - The **client** declares a temporary (or persistent) queue to receive the RPC response.
   - This is done using:
     ```rust
     let reply_queue = channel
         .queue_declare(
             "", // Empty name to create a temporary exclusive queue
             QueueDeclareOptions {
                 exclusive: true,
                 auto_delete: true,
                 ..QueueDeclareOptions::default()
             },
             FieldTable::default(),
         )
         .await?;
     ```

2. **Correlation ID**:
   - This is used to uniquely identify the request/response pair. The client generates a **UUID** for the correlation ID and sets it in the message properties when publishing the request.
   - The **server** echoes this correlation ID back in the response to allow the client to match the response with the original request.

3. **Client-Side Logic**:
   - The client sends a request and then consumes messages from the **reply-to** queue, checking the `correlation_id` to ensure the response matches the original request.

4. **Server-Side Logic**:
   - The server consumes messages from the **request queue** (in this case, `rpc_queue`) and sends the response back to the `reply-to` queue specified in the message properties.

### **Handling Multiple RPC Requests**

Since RabbitMQ queues are shared, you can handle multiple clients and requests by:
1. Using **unique correlation IDs** for each request.
2. Each client can declare its own exclusive **reply-to queue** to avoid interference from other clients.
   
Alternatively, you could use a **shared reply-to queue** but rely on **correlation IDs** to properly route responses to the correct clients.

### **Conclusion**

This example shows how to implement an **RPC pattern** using **`deadpool-lapin`** in Rust for RabbitMQ. The client sends a message with a `reply-to` queue and a unique `correlation_id`, and the server processes the request and sends the response back to the specified `reply-to` queue, allowing the client to consume the response asynchronously. This pattern is useful for implementing request-response communication in distributed systems where the client expects a response for each request sent to RabbitMQ.