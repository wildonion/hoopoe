


use crate::*;


// middleware to insert some data into the current depot of the request
#[handler]
pub async fn set_data(
    req: &mut Request, 
    res: &mut Response, 
    depot: &mut Depot, 
    ctrl: &mut FlowCtrl
){

    depot.insert("username", "hooper");
}