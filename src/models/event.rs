


use serde::{Serialize, Deserialize};
use crate::*;


#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct ReportQuery{
    pub imei: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>
}

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct ProduceNotifInfo{
    pub info: ProduceNotif,
}

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct ConsumeNotifInfo{
    pub info: ConsumeNotif,
}

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
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

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct LocationEventMessage{
    pub timestamp: Option<String>,
    pub correlationId: Option<String>,
    pub deviceId: Option<serde_json::Value>,
    pub imei: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub date: Option<chrono::NaiveDateTime>,
    pub positionStatus: Option<bool>,
    pub speed: Option<f64>,
    pub heading: Option<f64>,
    pub altitude: Option<f64>,
    pub satellites: Option<i64>,
    pub hdop: Option<f64>,
    pub gsmSignal: Option<i64>,
    pub odometer: Option<f64>,
    pub mobileCountryCode: Option<i64>,
    pub mobileNetworkCode: Option<i64>,
    pub locationAreaCode: Option<i64>,
    pub cellId: Option<i64>
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct LocationEvent{
    pub messageId: Option<serde_json::Value>,
    pub requestId: Option<serde_json::Value>,
    pub correlationId: Option<serde_json::Value>,
    pub conversationId: Option<serde_json::Value>,
    pub initiatorId: Option<serde_json::Value>,
    pub sourceAddress: Option<serde_json::Value>,
    pub destinationAddress: Option<serde_json::Value>,
    pub responseAddress: Option<serde_json::Value>,
    pub faultAddress: Option<serde_json::Value>,
    pub messageType: Option<serde_json::Value>,
    pub message: LocationEventMessage,
    pub expirationTime: Option<serde_json::Value>,
    pub sentTime: Option<serde_json::Value>,
    pub headers: Option<serde_json::Value>,
    pub host: Option<serde_json::Value>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct FetchLocationBasicReport{
    pub avg: f64,
    pub min: f64,
    pub max: f64,
    pub mileage: f64,
}