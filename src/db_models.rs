use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

use crate::models::Role;

#[derive(Serialize, Deserialize, Debug)]
pub struct DbUser {
    pub id: String,
    pub username: String,
    pub password: String,
    pub color: String,
}

#[derive(Serialize, Deserialize)]
pub struct NewUser {
    pub id: String,
    pub username: String,
    pub password: String,
    pub color: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DbGroup {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Serialize, Deserialize)]
pub struct NewGroup {
    pub name: String,
    pub description: Option<String>,
    pub creator_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DbGroupMembership {
    pub id: i32,
    pub group_id: String,
    pub user_id: String,
    pub role: Role,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Serialize, Deserialize)]
pub struct NewGroupMembership {
    pub group_id: String,
    pub user_id: String,
    pub role: Role,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DbRestaurant {
    pub id: i32,
    pub restaurant_code: String,
    pub group_id: String,
    pub cuisine: String,
}

#[derive(Serialize, Deserialize)]
pub struct NewRestaurant {
    pub restaurant_code: String,
    pub group_id: String,
    pub cuisine: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DbRating {
    pub id: i32,
    pub restaurant_id: i32,
    pub restaurant_code: String,
    pub user_id: String,
    pub username: String,
    pub score: f32,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub group_id: String,
    pub color: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct NewRating {
    pub restaurant_id: i32,
    pub user_id: String,
    pub username: String,
    pub score: f32,
    pub group_id: String,
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

#[derive(Serialize, Deserialize, Debug)]
pub struct DbOidcLink {
    pub id: String,
    pub user_id: String,
    pub provider: String,
    pub subject: String,
    pub created_at: Option<NaiveDateTime>,
}
