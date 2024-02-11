use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct DbUser {
    pub id: String,
    pub username: String,
    pub password: String,
}

#[derive(Serialize, Deserialize)]
pub struct NewUser {
    pub id: String,
    pub username: String,
    pub password: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DbRestaurant {
    pub id: String,
    pub cuisine: String,
}

#[derive(Serialize, Deserialize)]
pub struct NewRestaurant {
    pub id: String,
    pub cuisine: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DbMenuItem {
    pub id: i32,
    pub name: String,
    pub price: f32,
}

#[derive(Serialize, Deserialize)]
pub struct NewMenuItem {
    pub name: String,
    pub price: f32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DbRating {
    pub id: i32,
    pub restaurant_id: String,
    pub user_id: String,
    pub username: String,
    pub score: f32,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Serialize, Deserialize)]
pub struct NewRating {
    pub restaurant_id: String,
    pub user_id: String,
    pub username: String,
    pub score: f32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DbIp {
    pub id: i32,
    pub ip_address: String,
}

#[derive(Serialize, Deserialize)]
pub struct NewIp<'a> {
    pub ip_address: &'a str,
}
