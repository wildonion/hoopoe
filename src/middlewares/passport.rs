


use context::AppContext;
use interfaces::passport::Passport;
use crate::*;



/* ------------------  HOW THIS WORKS?
    this middleware responsible to passport verifier, in essence
    it checks the token time against the one on the redis and if 
    everything was ok a true flag will be set on the depot for 
    the passport_verified key otherwise responds the caller with
    the not ok response. so in those routers that uses this middelware
    we are free to check even the passport_verified key inside the 
    depot cause if we're inside that route guarded with this middleware
    we're sure that the passport is verified!
*/

#[handler]
pub async fn check_passport(
    req: &mut Request, 
    res: &mut Response,
    depot: &mut Depot,
    /* -----------
        ctrl controls the execution flow of the handlers like calling 
        the next handler in this one from the matched router tree, in 
        nodejs:
        app.get('/users', function(req, res, next) {
        
            next()

        });
    */
    ctrl: &mut FlowCtrl
){

    let app_ctx = depot.obtain::<Option<AppContext>>().unwrap(); // extracting shared app context
    match req.verify_token_time(app_ctx.clone(), "write").await{
        Ok(token_time) => {

            depot.insert("passport_verified", true);
        },
        Err(resp_err) => {

            ctrl.skip_rest(); // skip executing all rest handlers

            depot.insert("passport_verified", false);
            
            /* -------------------
                in order to mutate the content of a mutable pointer we should
                dereferencing it first then update the address which is pointing 
                to the old content with a new content, so in here we're:
                updating the state of the response object with the new one coming 
                from the error part, hence we're dereferencing it to override its 
                content with the new one, it would be same address but new value 
                or no binding.
            */
            *res = resp_err;

            /* 
                // new binding; new address and new value
                // not that the res object must be defined 
                // a a mutable object inside the handler
                // res = &mut res_err;

                // example of new valud vs new binding:
                let mut resp = String::from("");
                let mut p = &mut resp;
                println!("addres of resp: {:p}", p);
                
                // mutating with dereferencing
                *p = String::from("wildonion"); // value
                println!("addres of resp: {:p}", p); // same address
                    println!("value of resp: {}", resp);
                    
                // mutating with new binding
                let new_binding = &mut String::from("changed");
                p = new_binding;
                println!("addres of resp: {:p}", p); // new address
            */
        }
    }
}