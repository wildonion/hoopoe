



use serde::{Serialize, Deserialize};


#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Request{
    path: String,
    jwt: String,
    base_url: String,
}

impl Request{

    pub fn builder(path: &str, jwt: &str, base_url: &str) -> Self{
        Self{
            path: path.to_string(),
            jwt: jwt.to_string(),
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