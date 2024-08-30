


use serde::{Serialize, Deserialize};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>>{

    struct EventLoop<T: Clone + Send + Sync + 'static>{
        pub queue: std::sync::Arc<tokio::sync::Mutex<tokio::sync::mpsc::Receiver<T>>>,
    }
    impl<T: Clone + Send + Sync + 'static> EventLoop<T>{
        pub async fn on<F>(&mut self, event_name: &str, triggerer: F) where F: Fn(T) + Send + Sync{
            if event_name == "click"{
                let mut get_queue = self.queue.lock().await;
                while let Some(event) = get_queue.recv().await{
                    triggerer(event);
                }
                
                trait Interface: std::fmt::Debug{}
                struct DynamicDisptachDepInjection<'valid>{
                    pub dep: Option<&'valid dyn Interface>,
                }
                #[derive(Debug)]
                struct UseMe{}
                impl Interface for UseMe{}
                let depInjection = &UseMe{} as &dyn Interface; // cast the instance to the trait
                let instance = DynamicDisptachDepInjection{
                    dep: Some(depInjection)
                };
                if let Some(interface) = instance.dep{
                    println!("interface: {:#?}", interface);
                }

                #[derive(Clone)]
                pub struct setState<T>(pub T);
                
                pub trait useState<G>{
                    fn getState(&self) -> G;
                }
                impl<String: Clone> useState<String> for setState<String>{
                    fn getState(&self) -> String {
                        self.clone().0
                    }
                }
                let state = setState(String::from("wildonion"));
                let value = state.getState();

                struct useRef<'valid>(pub &'valid [u8]);
                let bytes = useRef(String::from("wildonion").as_bytes());
            }
        }
    }

    #[derive(Clone)]
    struct Event{
        pub data: Vec<u8>
    }
    #[derive(Serialize, Deserialize, Clone)]
    struct EventData{
        pub owner: String,
        pub recv_time: i64
    }


    let (tx, mut rx) = tokio::sync::mpsc::channel::<Event>(100);
    let mut eventloop = EventLoop::<Event>{
        queue: std::sync::Arc::new(tokio::sync::Mutex::new(rx))
    };
    eventloop.on("click", |event|{
        
        let string = serde_json::
            from_slice::<EventData>(&event.data)
            .unwrap();

    }).await;

    Ok(())

}