#![allow(dead_code)]

use chrono::{Datelike, NaiveDate, NaiveDateTime};
use serde::{Deserialize, Serialize};

use crate::db_models::DbRating;

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
    // TODO: Add active, so that we can enable or disable in the admin panel
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MenuItem {
    pub id: i32,
    pub restaurant_id: String,
    pub name: String,
    pub price: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Period {
    Q1,
    Q2,
    Q3,
    Q4,
}

impl Period {
    pub fn from_date(date: NaiveDate) -> Self {
        match date.month() {
            1..=3 => Period::Q1,
            4..=6 => Period::Q2,
            7..=9 => Period::Q3,
            _ => Period::Q4,
        }
    }

    pub fn to_month_range(&self) -> (u32, u32) {
        match self {
            Period::Q1 => (1, 3),
            Period::Q2 => (4, 6),
            Period::Q3 => (7, 9),
            Period::Q4 => (10, 12),
        }
    }

    pub fn to_date_range(&self, year: i32) -> anyhow::Result<(NaiveDate, NaiveDate)> {
        let month_range = self.to_month_range();

        let start_date = NaiveDate::from_ymd_opt(year, month_range.0, 1).ok_or_else(|| {
            anyhow::anyhow!(
                "Invalid start date for year: {}, month: {}",
                year,
                month_range.0
            )
        })?;

        let end_date = NaiveDate::from_ymd_opt(
            if month_range.1 == 12 { year + 1 } else { year },
            if month_range.1 == 12 {
                1
            } else {
                month_range.1 + 1
            },
            1,
        )
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Invalid end date calculation for year: {}, month: {}",
                year,
                month_range.1
            )
        })?
        .pred_opt()
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Failed to calculate the last day of the month for year: {}, month: {}",
                year,
                month_range.1
            )
        })?;

        Ok((start_date, end_date))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Rating {
    pub id: i32,
    pub restaurant_id: String,
    pub user_id: String,
    pub username: String,
    pub score: f32,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub period: Period,
}

impl Default for Rating {
    fn default() -> Self {
        let default_datetime = NaiveDateTime::default();
        Self {
            id: Default::default(),
            restaurant_id: Default::default(),
            user_id: Default::default(),
            username: Default::default(),
            score: Default::default(),
            created_at: default_datetime,
            updated_at: default_datetime,
            period: Period::from_date(default_datetime.date()),
        }
    }
}

impl Rating {
    pub fn new(
        id: i32,
        restaurant_id: String,
        user_id: String,
        username: String,
        score: f32,
        created_at: NaiveDateTime,
        updated_at: NaiveDateTime,
    ) -> Rating {
        Self {
            id,
            restaurant_id,
            user_id,
            username,
            score,
            created_at,
            updated_at,
            period: Period::from_date(updated_at.date()),
        }
    }

    pub fn from_db(db_rating: &DbRating) -> Rating {
        Self::new(
            db_rating.id,
            db_rating.restaurant_id.clone(),
            db_rating.user_id.clone(),
            db_rating.username.clone(),
            db_rating.score,
            db_rating.created_at,
            db_rating.updated_at,
        )
    }
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
