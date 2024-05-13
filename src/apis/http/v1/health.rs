



use crate::*;
use std::error::Error; // loading the Error trait allows us to call the source() method
use bytes::Buf;
use appstate::AppState;
use types::HoopoeHttpResponse;


pub mod index;
pub mod auth;