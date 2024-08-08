


use crate::*;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct UserData{
    pub id: i32,
    pub region: Option<String>,
    pub username: String,
    pub bio: Option<String>,
    pub avatar: Option<String>,
    pub banner: Option<String>,
    pub wallet_background: Option<String>,
    pub activity_code: String,
    pub twitter_username: Option<String>,
    pub facebook_username: Option<String>,
    pub discord_username: Option<String>,
    pub identifier: Option<String>,
    pub mail: Option<String>, /* unique */
    pub google_id: Option<String>, /* unique */
    pub microsoft_id: Option<String>, /* unique */
    pub is_mail_verified: bool,
    pub is_phone_verified: bool,
    pub phone_number: Option<String>, /* unique */
    pub paypal_id: Option<String>, /* unique */
    pub account_number: Option<String>, /* unique */
    pub device_id: Option<String>, /* unique */
    pub social_id: Option<String>, /* unique */
    pub cid: Option<String>, /* unique */
    pub screen_cid: Option<String>, /* unique */
    pub snowflake_id: Option<i64>, /* unique */
    pub stars: Option<i64>,
    pub user_role: String,
    pub token_time: Option<i64>,
    pub balance: Option<i64>,
    pub last_login: Option<String>,
    pub extra: Option<serde_json::Value>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct CheckTokenResponse{
    pub data: UserData,
    pub message: String,
    pub status: u16,
    pub is_error: bool
}