


use crate::*;

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

        let check_token_endpoint = format!("{}{}", self.base_url, self.path);
        let jwt = self.auth_token.clone();
        
        let req = reqwest::Client::new();
        let res = req
            .get(check_token_endpoint)
            .header("Authorization", &jwt)
            .send()
            .await
            .unwrap();

        let json_data = res.json::<serde_json::Value>().await.unwrap();
        json_data

    }

    pub async fn delete(&self, body: serde_json::Value) -> serde_json::Value{
        
        let check_token_endpoint = format!("{}{}", self.base_url, self.path);
        let jwt = self.auth_token.clone();
                    
        let req = reqwest::Client::new();
        let res = req
            .delete(check_token_endpoint)
            .header("Authorization", &jwt)
            .json(&body) // json value can only be sent using json() method
            .send()
            .await
            .unwrap();
        
        let json_data = res.json::<serde_json::Value>().await.unwrap();
        json_data

    }

    pub async fn put(&self, body: serde_json::Value) -> serde_json::Value{
        
        let check_token_endpoint = format!("{}{}", self.base_url, self.path);
        let jwt = self.auth_token.clone();
                    
        let req = reqwest::Client::new();
        let res = req
            .put(check_token_endpoint)
            .header("Authorization", &jwt)
            .json(&body) // json value can only be sent using json() method
            .send()
            .await
            .unwrap();
        
        let json_data = res.json::<serde_json::Value>().await.unwrap();
        json_data

    }

    pub async fn patch(&self, body: serde_json::Value) -> serde_json::Value{
        
        let check_token_endpoint = format!("{}{}", self.base_url, self.path);
        let jwt = self.auth_token.clone();
                    
        let req = reqwest::Client::new();
        let res = req
            .patch(check_token_endpoint)
            .header("Authorization", &jwt)
            .json(&body) // json value can only be sent using json() method
            .send()
            .await
            .unwrap();
        
        let json_data = res.json::<serde_json::Value>().await.unwrap();
        json_data

    }

    pub async fn post(&self, body: serde_json::Value) -> serde_json::Value{

        let check_token_endpoint = format!("{}{}", self.base_url, self.path);
        let jwt = self.auth_token.clone();
                    
        let req = reqwest::Client::new();
        let res = req
            .post(check_token_endpoint)
            .header("Authorization", &jwt)
            .json(&body) // json value can only be sent using json() method
            .send()
            .await
            .unwrap();
        
        let json_data = res.json::<serde_json::Value>().await.unwrap();
        json_data
        
    }

}