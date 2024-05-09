


/* 
    actor worker based components:
    we have producer and consumer actors per each data streamer 
    we have mutator and accessor actors per each db model
    we have ws session and server actors per each route

    need actor worker if: 
        something needs to be checked or executed constantly in the background using an interval like producing data
        an state of a component needs to be updated during the app execution like sending message to a worker to start 
        consuming an specific data from an specific queue in the background 
    otherwise we would feel better if we use pure functions per consumer and producer instead of having actors.
*/ 
pub mod cqrs;
pub mod consumers;
pub mod producers;
pub mod ws;