

use crate::{
    actors::cqrs::accessors::location::{LocationAccessorActor, RequestLocationBasicReport}, 
    consts::{STORAGE_IO_ERROR_CODE}, models::event::{FetchLocationBasicReport, ReportQuery}
};
use crate::{
    actors::consumers::location::{
        ActionType, ConsumeNotif, NotifData, ReceiverInfo
    }, 
    consts::MAILBOX_CHANNEL_ERROR_CODE
};
use crate::*;
use std::error::Error; // loading the Error trait allows us to call the source() method
use bytes::Buf;
use appstate::AppState;
use types::HoopoeHttpResponse;
use plugins::passport::Passport;
use actix_web::http::StatusCode;
use crate::cookie::Cookie;


pub mod get;
pub mod set;
pub mod update;