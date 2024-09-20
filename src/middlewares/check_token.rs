


use context::AppContext;
use interfaces::passport::Passport;
use models::user::UserData;
use crate::*;


/* 
    despite the feature of injecting data into the requst object in nodejs
    we can't do that directly in here cause the request object has limited 
    fields we can't be used to inject some data into that easily, solution 
    to this is using some initialized data structure to add data to it like 
    a map to move it around apis and that's what depot is being used for.

    when we call an api from a router with this this middleware registered on 
    the process takes place before handling the api body so it first checks the 
    request header to find the token then send that to the check token server 
    to verify the token then if anything was ok we'll inject some true flag 
    into the depo of the api which allows other apis obtain it to check the 
    status but the injection process is not quite necessary cause if the token 
    was invalid we'll fulfill the reponse object with an error. 
*/
#[handler]
pub async fn check_token(
    req: &mut Request, 
    res: &mut Response,
    depot: &mut Depot,
    ctrl: &mut FlowCtrl
){

    let app_ctx = depot.obtain::<Option<AppContext>>().unwrap(); // extracting shared app context
    let config = app_ctx.clone().unwrap().config;
    let base_url = config.unwrap().vars.GEM_BASE_URL.clone();

    match req.get_user(&base_url).await{
        Ok(user_data) => {
            
            // injecting the fetched user data into the depot 
            // other api handlers can access this
            depot.insert("user_data", user_data);

            // call the next handler after passing this middleware
            if ctrl.has_next(){
                ctrl.call_next(req, depot, res).await;
            }

        },
        Err(err_resp) => {

            // injecting none user data
            depot.insert("user_data", None::<UserData>);

            *res = err_resp; // fulfill the response object with error
        }
    }

}