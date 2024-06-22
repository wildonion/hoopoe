

/* ---------------------------------------------------------------
  ╰┈➤    code level consensus logics for a distributed lock
   ---------------------------------------------------------------

    https://martin.kleppmann.com/2016/02/08/how-to-do-distributed-locking.html

    embarking on the journey towards distributed architectures distributed locking 
    becomes an indispensable component in maintaining data integrity and avoiding 
    conflicts they prevent concurrent access to a resource, ensuring that only one 
    process can perform an operation at a time to maintain data integrity, consider
    the following examples:
   
    0 ▶ accessing a shared resources by threads/nodes at the same time leads us to have 
        local/remote race conditions and deadlocks only one thread or node can acquire 
        the lock like there are no two clients can purchase a product at the same time
        there must be a lock on the first request and make the product a unique item cause 
        no two owners can own an item at a time for example client1 send mint request app 
        proceeds with the client1 request it processes it take 30 seconds to respond the 
        caller meanwhile client2 sends the mint request for the same item we should reject 
        his request cause we have an ingoing process we must reject his request with "another 
        user is minting this".

    1 ▶ users want to purchase a product in stock concurrently this must be taken 
        care of in a cluster using a distributed lock mechanism to prevent race conditions 
        and ensure data consistency, in a single node however can be done using mutex 
        to ensure the product can only be purchased only once at a time, generally using 
        mutex in a single node is enough to ensure the item is being mutated by only 
        one thread at a time.
   
    2 ▶ there are multiple instances of a WS based Rust server which uses pg and redis 
        as an storage management with atomic transaction support and a redis pubsub logic 
        for data synchronisation between other nodes as follows:
        1) a peer sends chat like "hello server" to ws server 
        2) ws server publish chat to redis `peer_chat` channel
        3) all cluster nodes subsribe to `peer_chat` channel
        4) all cluster `nodes` receive the data from `peer_chat` channel
        5) in this stage the receievd data or the peer chat message must be stored in pg db
            but how to stroe received `peer_chat` data into db only once? which node is responsible 
            to store it in such a way other nodes know that they don't need to store it cause 
            a node has stored it already, in other words how notice other nodes when a node has 
            decided to store the data, how to make synchronisation for storing data in db? how
            to execute a task only once in a cluster?

    solution: distributed locks coordinates database writes across multiple 
                nodes and prevents duplicate insertions, acquire the lock -> insert 
                into db -> release the lock by deleting the key


    weakness of redlock algorithm:
        client1 wants to hold the lock in all redis nodes
        client1 process gets paused (gc, network delay, async issue, crashes, ...)
        exp time of lock gets ended
        all nodes release the lock 
        client2 wants the lock in all nodes
        client1 process has started to continuing to write operations into db
        client2 started writing operations into db 
        ------ RACE CONDITIONS MET ------
        client1 long delay messed up the logic
*/

use crate::*;
use rslock::*;


/* implementation of zookeeper and redis distributed lock */
// https://stackoverflow.com/questions/57571043/cancelling-a-distributed-lock-inside-a-mutex-from-another-thread-causes-a-deadlo
// https://users.rust-lang.org/t/cancelling-a-distributed-lock-inside-a-mutex-from-another-thread-causes-a-deadlock/31624/4

#[derive(Clone)]
pub struct DistLock{
    manager: ManagerKind,
    locker: Option<std::sync::Arc<rslock::LockManager>>,
}

#[derive(Clone)]
pub enum ManagerKind{
    RedLock,
    ZooKeeper
}

impl DistLock{
    
    pub fn new_redlock(locker: Option<std::sync::Arc<rslock::LockManager>>) -> DistLock{

        Self{
            manager: ManagerKind::RedLock,
            locker
        }

    }
}