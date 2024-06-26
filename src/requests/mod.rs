



use serde::{Serialize, Deserialize};


#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Fetch{
    path: String,
    auth_token: String,
    base_url: String,
}

impl Fetch{

    pub fn builder(path: &str, auth_token: &str, base_url: &str) -> Self{
        Self{
            path: path.to_string(),
            auth_token: auth_token.to_string(),
            base_url: base_url.to_string()
        }
    }

    pub async fn get(&self) -> serde_json::Value{

        todo!()
    }

    pub async fn post(&self, body: serde_json::Value) -> serde_json::Value{

        todo!()
    }
}