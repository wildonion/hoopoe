


use chrono::{DateTime, NaiveDateTime};
use sea_orm::prelude::DateTimeWithTimeZone;
use serde::{Serialize, Deserialize};
use workers::notif::{ConsumeNotif, ProduceNotif};
use crate::*;



#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct EventQuery{
    pub owner: Option<String>, // the entity owner
    pub id: Option<i32>, // any entity id 
    pub from: Option<u64>,
    pub to: Option<u64>,
    pub page_size: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct UpdateHoopRequest{
    pub  hoop_id: i32,
}

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct NewHoopRequest{
    
}

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct GenerateTokenTimeQuery{
    pub exp_time: Option<u64>, // in seconds
    pub scope: Option<String>
}

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub enum TokenTimeScope{
    #[default]
    Write,
    Read
}

#[derive(Serialize, Deserialize, Clone, Default, Debug, ToSchema)]
pub struct ProduceNotifInfo{
    pub info: ProduceNotif,
}

#[derive(Serialize, Deserialize, Clone, Default, Debug, ToSchema)]
pub struct ConsumeNotifInfo{
    pub info: ConsumeNotif,
}

#[derive(Serialize, Deserialize, Clone, Default, Debug, ToSchema)]
#[derive(Extractible)]
#[salvo(extract(default_source(from="body")))]
pub struct RegisterNotif{
    pub producer_info: Option<ProduceNotifInfo>,
    pub consumer_info: Option<ConsumeNotifInfo>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct HoopEvent{
    pub etype: String, 
    pub manager: i32,
    pub entrance_fee: i64,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug, ToSchema)]
pub enum ActionType{ // all the action type that causes the notif to get fired
    #[default]
    ProductPurchased, // or minted
    Zerlog,
    EventCreated,
    EventExpired,
    EventLocked,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug, ToSchema)]
pub struct NotifData{
    pub id: String,
    pub receiver_info: String,
    pub action_data: serde_json::Value, // you can map this to any structure you know!
    pub actioner_info: String,
    pub action_type: ActionType,
    pub fired_at: i64,
    pub is_seen: bool,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct DbNotifData{
    pub id: i32,
    pub receiver_info: String,
    pub nid: String,
    pub action_data: serde_json::Value, // you can map this to any structure you know!
    pub actioner_info: String,
    pub action_type: String,
    pub fired_at: NaiveDateTime,
    pub is_seen: bool,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct DbHoopData{
    pub id: i32,
    pub etype: String,
    pub manager: i32,
    pub entrance_fee: i64,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}