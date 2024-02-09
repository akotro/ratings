use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub token: String,
    pub username: String,
    pub password: String,
    pub ratings: Vec<Rating>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserClaims {
    pub id: String,
    pub username: String,
    pub exp: usize,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Restaurant {
    pub id: String,
    pub cuisine: String,
    pub menu: Vec<MenuItem>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MenuItem {
    pub id: i32,
    pub restaurant_id: String,
    pub name: String,
    pub price: f32,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Rating {
    pub id: i32,
    pub restaurant_id: String,
    pub user_id: String,
    pub username: String,
    pub score: f32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Ip {
    pub ip_address: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub message: String,
    pub data: Option<T>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        ApiResponse {
            success: true,
            message: String::new(),
            data: Some(data),
        }
    }

    pub fn error(message: String) -> ApiResponse<()> {
        ApiResponse {
            success: false,
            message,
            data: None,
        }
    }
}

impl<T> ApiResponse<T>
where
    T: serde::de::DeserializeOwned,
{
    pub fn from_response_body(response_body: &str) -> Result<T, Box<dyn std::error::Error>> {
        let api_response: ApiResponse<T> = serde_json::from_str(response_body)?;
        if api_response.success {
            if let Some(data) = api_response.data {
                Ok(data)
            } else {
                Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Data is missing in successful response",
                )))
            }
        } else {
            Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                api_response.message,
            )))
        }
    }
}
