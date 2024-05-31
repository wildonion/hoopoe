

use std::time::Duration;
use once_cell::sync::Lazy;
use serde::{Serialize, Deserialize};
use tokio::io::AsyncWriteExt;
use rand::Rng;
use rand::prelude::SliceRandom;
use rand::random;

pub mod queries;
use queries::*;


pub const CHARSNUMSSPECIAL: &str = "!@#$%&*0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnporstuvwxyz";
pub const CHARSET: &[u8] = b"0123456789";

// an atomic vector of mutual exclusion products Lazy make a constant 
// value from none constant ones cause static requires a constant value 
// since its value can't be changed during the lifetime of the app
pub static PURCHASE_DEMO_LOCK_RWLOCK: Lazy<std::sync::Arc<tokio::sync::RwLock<Vec<i32>>>> = Lazy::new(||{
    std::sync::Arc::new(
        tokio::sync::RwLock::new(
            vec![]
        )
    )
});

pub static PURCHASE_DEMO_LOCK_MUTEX: Lazy<std::sync::Arc<tokio::sync::Mutex<Vec<i32>>>> = Lazy::new(||{
    std::sync::Arc::new(
        tokio::sync::Mutex::new(
            vec![]
        )
    )
});


pub const APP_NAME: &str = "Hoopoe";
pub const LOGS_FOLDER_ERROR_KIND: &str = "logs/error-kind";

pub const SERVERS: &[&str] = &["http", "tcp", "grpc"];

// every code has 2 bytes long since the code value is larger than 
// 255 we've used u16 which is 2 chars in hex
pub static SERVER_IO_ERROR_CODE: &u16 = &0xFFFE;
pub static STORAGE_IO_ERROR_CODE: &u16 = &0xFFFF;
pub static CHRONO_ERROR_CODE: &u16 = &0xFFAE;
pub static CODEC_ERROR_CODE: &u16 = &0xFFFB;
pub static MAILBOX_CHANNEL_ERROR_CODE: &u16 = &0xFFFD;
pub static HTTP_RESPONSE_ERROR_CODE: &u16 = &0xFFFA;
pub static CRYPTER_THEMIS_ERROR_CODE: &u16 = &0xFFAB;
pub static THIRDPARTYAPI_ERROR_CODE: &u16 = &0xFFFC;
pub const PING_INTERVAL: Duration = Duration::from_secs(10);


pub const INVALID_EVENT_TYPE: &str = "Invalid Event Type";
pub const SEE_CLIENT_ADDED: &str = "Client Added";
pub const SEE_EVENT_SENT: &str = "Event Broadcasted";


/* converting an slice array of u8 bytes into an array with 32 byte length */
pub fn convert_into_u8_32(data: &[u8]) -> Option<[u8; 32]>{
    data.try_into().ok()
}

// convert any generic of Vec into a generic slice by leaking and consuming the 
// memory of the vector to return an static reference to the leacked part since 
// that part will never be freed until the lifetime of the app
pub fn vector_slice<T>(s: Vec<T>) -> &'static [T]{
    Box::leak(s.into_boxed_slice())
}

pub fn gen_random_chars(size: u32) -> String{
    let mut rng = rand::thread_rng();
    (0..size).map(|_|{
        /* converint the generated random ascii to char */
        char::from_u32(rng.gen_range(33..126)).unwrap() // generating a char from the random output of type u32 using from_u32() method
    }).collect()
}

pub fn gen_random_number(from: u32, to: u32) -> u32{
    let mut rng = rand::thread_rng(); // we can't share this between threads and across .awaits
    rng.gen_range(from..to)
} 

pub fn gen_random_idx(idx: usize) -> usize{
    if idx < CHARSET.len(){
        idx
    } else{
        gen_random_idx(random::<u8>() as usize)
    }
}

pub fn gen_random_passwd(size: u32) -> String{
    let u8vec = CHARSNUMSSPECIAL.as_bytes().to_vec();
    let mut rng = rand::thread_rng();
    (0..size)
        .map(|_|{
            let rand_c = u8vec.choose(&mut rng).unwrap();
            char::from_u32(*rand_c as u32).unwrap()
        })
        .collect()
}

pub fn string_to_static_str(s: String) -> &'static str { 
    /* 
        we cannot obtain &'static str from a String because Strings may not live 
        for the entire life of our program, and that's what &'static lifetime means. 
        we can only get a slice parameterized by String own lifetime from it, we can 
        obtain a static str but it involves leaking the memory of the String. this is 
        not something we should do lightly, by leaking the memory of the String, this 
        guarantees that the memory will never be freed (thus the leak), therefore, any 
        references to the inner object can be interpreted as having the 'static lifetime.
        
        also here it's ok to return the reference from function since our reference lifetime 
        is static and is valid for the entire life of the app

        leaking the memory of the heap data String which allows us to have an 
        unfreed allocation that can be used to define static str using it since
        static means we have static lifetime during the whole lifetime of the app
        and reaching this using String is not possible because heap data types 
        will be dropped from the heap once their lifetime destroyed in a scope
        like by moving them into another scope hence they can't be live longer 
        than static lifetime

        Note: this will leak memory! the memory for the String will not be freed 
        for the remainder of the program. Use this sparingly
    */
    Box::leak(s.into_boxed_str()) 

}

pub fn vector_to_static_slice(s: Vec<u32>) -> &'static [u32] { 
    /* 
        we cannot obtain &'static str from a Vec because Vecs may not live 
        for the entire life of our program, and that's what &'static lifetime means. 
        we can only get a slice parameterized by Vec own lifetime from it, we can 
        obtain a static str but it involves leaking the memory of the Vec. this is 
        not something we should do lightly, by leaking the memory of the Vec, this 
        guarantees that the memory will never be freed (thus the leak), therefore, any 
        references to the inner object can be interpreted as having the 'static lifetime.
        
        also here it's ok to return the reference from function since our reference lifetime 
        is static and is valid for the entire life of the app

        leaking the memory of the heap data Vec which allows us to have an 
        unfreed allocation that can be used to define static str using it since
        static means we have static lifetime during the whole lifetime of the app
        and reaching this using Vec is not possible because heap data types 
        will be dropped from the heap once their lifetime destroyed in a scope
        like by moving them into another scope hence they can't be live longer 
        than static lifetime

        Note: this will leak memory! the memory for the Vec will not be freed 
        for the remainder of the program. Use this sparingly
    */
    Box::leak(s.into_boxed_slice()) 

}