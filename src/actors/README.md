

## -ˋˏ✄┈┈┈┈ how amqp (rmq) works?

> refer to [this](https://www.cloudamqp.com/blog/part4-rabbitmq-for-beginners-exchanges-routing-keys-bindings.html#:~:text=The%20routing%20key%20is%20a%20message%20attribute%20added%20to%20the,routing%20key%20of%20the%20message.) for more comprehensive concepts.

`channel[producer] ----message----> broker[exchange[i]] ----message|>keys[i]----> queues[i]`
`channel[consumer]` <----message---- queue <-----routing key------ broker[exchange[i]]

### channels

every protocol operation happends on a channel like declare topology, publish, consume. channels are something like local mps jobq channels but they're inside a single TCP connection and are created per each thread cause they're not object safe thread or they're not bounded to `Send` and `Sync` traits in other words a connection is a TCP connection between your application and the RabbitMQ broker, a channel is a virtual and lightweight connection inside a connection, a channel multiplexes a TCP connection, typically, each process only creates one TCP connection, and uses 
multiple channels in that connection for different threads. 

### routing to queues from the exchange!

first we need to declare a queue for the messages we want to consume them after that we must bind the declared queue to an exchange (the way we want to produce and consume messages) over an sepecific key, when the producer produces the payload it sends the payload in a channel to an specific exchange specified by the routing key, then routing key knows where these messages must be sent to which queue cause the queue is already bounded to the exchange routing key. exchanges are the way of transferring message based on an specific routing key. routing key is used to route the message matches the key to the queue which is bounded to the exchange routing key, this helps a lot since there might be multiple different queues bounded to the same exchange but with different routing keys, rmq broker knows how to route and what messages must be sent to which queue with this logic.

a producer never sends a message directly to a queue, instead it uses an exchange as a routing mediator, therefore exchange facilitate the routing of messages to queues based on defined rules, rmq supports fanout, direct, topic and headers exchanges, typically queue can be bounded into the exchange then exchange uses routing key as an address to decide how to route the message, a message goes to the queue with the binding key that is exactly matches the routing key. since we might have more than one queue, a queue gets bounded to an exchange using a binding key which is called routing key and is a unique key used to tell the exchange that use this pattern to route the message to a queue that is already bounded to this binding key:

> an exchange can send messages to a queue that is bounded to it already, in a multiple different ways based on routing key, in other words multiple queues can receive messages from a single exchange, each of which in a different way only using a routing key, each queue receives different messages than each other.

**direct exchange** | "": for the direct exchange the routing key is the same as the queue name (cause it's direct sending!), rmq doesn't allow to bind a queue to an exchange of type direct.

**fanout exchange** | "amq.fanout": a fanout exchange copies and routes a received message to all queues that are bounded to it regardless of routing keys or pattern matching as with direct and topic exchanges, the keys provided will simply be ignored.

**topic exchange** | "amq.topic" : it's a PubSub pattern in rmq! topic exchanges route messages to queues based on wildcard matches between the routing key and the routing pattern, which is specified by the queue binding. messages are routed to one or many queues based on a matching between a message routing key and this pattern in other words all messages with a routing key that match the routing pattern are routed to the queue and stay there until the consumer consumes the message. unlike fanout and direct patterns in this pattern the messge won't get poped out of the queue after it's consumed this behavior allows multiple consumers to receive the same message from the exchange like a real PubSub pattern.

**header exchange** | "amq.headers": a headers exchange routes messages based on arguments containing headers and optional values. headers exchanges are very similar to topic exchanges, but route messages based on header values instead of routing keys. a message matches if the value of the header equals the value specified upon binding. a special argument named "x-match", added in the binding between exchange and queue, specifies if all headers must match or just one. either any common header between the message and the binding count as a match, or all the headers referenced in the binding need to be present in the message for it to match. the "x-match" property can have two different values: "any" or "all", where "all" is the default value. a value of "all" means all header pairs (key, value) must match, while value of "any" means at least one of the header pairs must match. headers can be constructed using a wider range of data types, integer or hash for example, instead of a string. The headers exchange type (used with the binding argument "any") is useful for directing messages which contain a subset of known (unordered) criteria.

### what is happening in each queue?

the messages stay in the queue until they are handled by a consumer. if a queue is bounded to an exchange routing key and the consumer is not up yet, all the messages get accumulated inside the queue until the consumer get back online and starts consuming them.

## Final words

we can declare as much as queue per each consumer we want and bind it to an specific exchange routing key this logic enables each consumer consume message coming from an exchange from its own queue cause the queue is already bounded to that exchange routing key generally any queue that is bounded to an specific routing key will receive the messages coming from that exchange for any exchange pattern build a queue for each consumer and bind it to the routing key cause routing key specifies to which queue message must be sent and it works if there are multiple queues with different names cause they are bounded to the same routing key change.