


## How Capnpc works?

in RPC every method call is a round trip in networking, canpnp pack all calls together in only one round trip, it uses the promise pipelining feature which every call is a future object which can be solved by awaiting in which it returns all the results from all the calls sent to the server it's like `foo().bar().end()` takes only 1 round trip which by awaiting on them it returns all the result from the server, it can call methods without waiting just take a round trip. call results are returned to the client before the request even arrives at the server, this is the feature of promise it's a place holder for the result of each call and once we await on them all the results will be arrived in one round trip.