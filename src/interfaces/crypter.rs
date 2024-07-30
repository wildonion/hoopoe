




use constants::CRYPTER_THEMIS_ERROR_CODE;
use wallexerr::misc::SecureCellConfig;
use wallexerr::misc::Wallet;
use crate::*;


/* -----------------
    https://boats.gitlab.io/blog/post/async-methods-i/
    GATs can now have generic params which allows us to have async 
    method in traits cause the future returned by an async function 
    captures all lifetimes inputed into the function, in order 
    to express that lifetime relationship, the Future type needs 
    to be generic over a lifetime, so that it can capture the 
    lifetime from the self argument on the method.
    generic and lifetime wasn't supported in GAT till Rust 1.79
    by the result we can have async methods in traits without 
    using third party crates.
    the fact that async method wasn't supported in traits was
    due to the unspported feature of generic and lifetime in GAT
    which wouldn't allow to return a future object from the trait
    method cause future obejcts capture lifetimes forces us to pass
    the GAT with lifetime as the return type of async trait method
    hence using the GAT as the return type of async trait method 
    wasn't supported therefore having future objects in trait method 
    return type was invalid.
    it's notable that traits with async methods can't be object safe 
    and Boxed with Box<dyn we can't use the builtin async method 
    instead we should either use the async_trait crate or remove 
    the async keywords.
*/
pub trait Crypter{
    fn encrypt(&self, secure_cell_config: &mut SecureCellConfig);
    fn decrypt(&self, secure_cell_config: &mut SecureCellConfig);
}

// used for en(de)crypting image in form of Vec<u8> slice or &[u8]
impl Crypter for &[u8]{
    fn encrypt(&self, secure_cell_config: &mut wallexerr::misc::SecureCellConfig){
        match wallexerr::misc::Wallet::secure_cell_encrypt(secure_cell_config){ // passing the redis secure_cell_config instance
            Ok(data) => {
                secure_cell_config.data = data
            },
            Err(e) => {

                tokio::spawn(async move{
                    let source = &e.to_string(); // we know every goddamn type implements Error trait, we've used it here which allows use to call the source method on the object
                    let err_instance = crate::error::HoopoeErrorResponse::new(
                        *constants::CRYPTER_THEMIS_ERROR_CODE, // error hex (u16) code
                        source.as_bytes().to_vec(), // text of error source in form of utf8 bytes
                        crate::error::ErrorKind::Crypter(crate::error::CrypterError::Themis(e)), // the actual source of the error caused at runtime
                        &String::from("CrypterInterface.encrypt.Wallet::secure_cell_decrypt"), // current method name
                        None
                    ).await;
                });

                // don't update data field in secure_cell_config instance
                // the encrypted data remains the same as before.
            }
        };
    }

    fn decrypt(&self, secure_cell_config: &mut wallexerr::misc::SecureCellConfig){
        match wallexerr::misc::Wallet::secure_cell_decrypt(secure_cell_config){
            Ok(encrypted) => {
                
                let stringified_data = hex::encode(&encrypted);
                // update the data field with the encrypted content bytes
                secure_cell_config.data = encrypted; 

            },
            Err(e) => {

                // log the error in the a lightweight thread of execution inside tokio threads
                // since we don't need output or any result from the task inside the thread thus
                // there is no channel to send data to outside of tokio::spawn
                tokio::spawn(async move{
                    let source = &e.to_string(); // we know every goddamn type implements Error trait, we've used it here which allows use to call the source method on the object
                    let err_instance = crate::error::HoopoeErrorResponse::new(
                        *constants::CRYPTER_THEMIS_ERROR_CODE, // error hex (u16) code
                        source.as_bytes().to_vec(), // text of error source in form of utf8 bytes
                        crate::error::ErrorKind::Crypter(crate::error::CrypterError::Themis(e)), // the actual source of the error caused at runtime
                        &String::from("CrypterInterface.encrypt.Wallet::secure_cell_encrypt"), // current method name
                        None
                    ).await;
                });
                
                // don't update data field in secure_cell_config instance
                // the raw data remains the same as before.
            }
        };

    }

} 


// used for en(de)crypting image in form of Vec<u8>
impl Crypter for Vec<u8>{
    fn encrypt(&self, secure_cell_config: &mut wallexerr::misc::SecureCellConfig){
        match wallexerr::misc::Wallet::secure_cell_encrypt(secure_cell_config){ // passing the redis secure_cell_config instance
            Ok(data) => {
                secure_cell_config.data = data
            },
            Err(e) => {

                tokio::spawn(async move{
                    let source = &e.to_string(); // we know every goddamn type implements Error trait, we've used it here which allows use to call the source method on the object
                    let err_instance = crate::error::HoopoeErrorResponse::new(
                        *constants::CRYPTER_THEMIS_ERROR_CODE, // error hex (u16) code
                        source.as_bytes().to_vec(), // text of error source in form of utf8 bytes
                        crate::error::ErrorKind::Crypter(crate::error::CrypterError::Themis(e)), // the actual source of the error caused at runtime
                        &String::from("CrypterInterface.encrypt.Wallet::secure_cell_decrypt"), // current method name
                        None
                    ).await;
                });

                // don't update data field in secure_cell_config instance
                // the encrypted data remains the same as before.
            }
        };
    }

    fn decrypt(&self, secure_cell_config: &mut wallexerr::misc::SecureCellConfig){
        match wallexerr::misc::Wallet::secure_cell_decrypt(secure_cell_config){
            Ok(encrypted) => {
                
                let stringified_data = hex::encode(&encrypted);
                // update the data field with the encrypted content bytes
                secure_cell_config.data = encrypted; 

            },
            Err(e) => {

                // log the error in the a lightweight thread of execution inside tokio threads
                // since we don't need output or any result from the task inside the thread thus
                // there is no channel to send data to outside of tokio::spawn
                tokio::spawn(async move{
                    let source = &e.to_string(); // we know every goddamn type implements Error trait, we've used it here which allows use to call the source method on the object
                    let err_instance = crate::error::HoopoeErrorResponse::new(
                        *constants::CRYPTER_THEMIS_ERROR_CODE, // error hex (u16) code
                        source.as_bytes().to_vec(), // text of error source in form of utf8 bytes
                        crate::error::ErrorKind::Crypter(crate::error::CrypterError::Themis(e)), // the actual source of the error caused at runtime
                        &String::from("CrypterInterface.encrypt.Wallet::secure_cell_encrypt"), // current method name
                        None
                    ).await;
                });
                
                // don't update data field in secure_cell_config instance
                // the raw data remains the same as before.
            }
        };

    }

}

// used for en(de)crypting data in form of string
impl Crypter for String{
    fn decrypt(&self, secure_cell_config: &mut SecureCellConfig){
        match Wallet::secure_cell_decrypt(secure_cell_config){ // passing the redis secure_cell_config instance
            Ok(data) => {
                secure_cell_config.data = data
            },
            Err(e) => {

                tokio::spawn(async move{
                    let source = &e.to_string(); // we know every goddamn type implements Error trait, we've used it here which allows use to call the source method on the object
                    let err_instance = crate::error::HoopoeErrorResponse::new(
                        *CRYPTER_THEMIS_ERROR_CODE, // error hex (u16) code
                        source.as_bytes().to_vec(), // text of error source in form of utf8 bytes
                        crate::error::ErrorKind::Crypter(crate::error::CrypterError::Themis(e)), // the actual source of the error caused at runtime
                        &String::from("CrypterInterface.encrypt.Wallet::secure_cell_decrypt"), // current method name
                        None
                    ).await;
                });

                // don't update data field in secure_cell_config instance
                // the encrypted data remains the same as before.
            }
        };

    }
    fn encrypt(&self, secure_cell_config: &mut SecureCellConfig){
       match Wallet::secure_cell_encrypt(secure_cell_config){
            Ok(encrypted) => {
                
                let stringified_data = hex::encode(&encrypted);
                // update the data field with the encrypted content bytes
                secure_cell_config.data = encrypted; 

            },
            Err(e) => {

                // log the error in the a lightweight thread of execution inside tokio threads
                // since we don't need output or any result from the task inside the thread thus
                // there is no channel to send data to outside of tokio::spawn
                tokio::spawn(async move{
                    let source = &e.to_string(); // we know every goddamn type implements Error trait, we've used it here which allows use to call the source method on the object
                    let err_instance = crate::error::HoopoeErrorResponse::new(
                        *CRYPTER_THEMIS_ERROR_CODE, // error hex (u16) code
                        source.as_bytes().to_vec(), // text of error source in form of utf8 bytes
                        crate::error::ErrorKind::Crypter(crate::error::CrypterError::Themis(e)), // the actual source of the error caused at runtime
                        &String::from("CrypterInterface.encrypt.Wallet::secure_cell_encrypt"), // current method name
                        None
                    ).await;
                });
                
                // don't update data field in secure_cell_config instance
                // the raw data remains the same as before.
            }
        };

    }

}