#![allow(dead_code)]

use chrono::{Datelike, NaiveDate, NaiveDateTime};
use serde::{Deserialize, Serialize};
use web_push::{IsahcWebPushClient, SubscriptionInfo};

use crate::db_models::{DbGroupMembership, DbRating};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub token: String,
    pub username: String,
    pub password: String,
    pub color: String,
    pub ratings: Vec<Rating>,
    pub group_memberships: Vec<GroupMembership>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserClaims {
    pub id: String,
    pub username: String,
    pub exp: usize,
}

#[derive(Default, Clone)]
pub struct PushClient {
    pub vapid_public_key: String,
    pub vapid_private_key: String,
    pub client: IsahcWebPushClient,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NewPushSubscription {
    pub user_id: String,
    pub subscription_info: SubscriptionInfo,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PushSubscription {
    pub endpoint: String,
    pub user_id: String,
    pub p256dh: String,
    pub auth: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(rename_all = "lowercase")]
pub enum Role {
    Admin,
    #[default]
    Member,
}

impl std::convert::From<String> for Role {
    fn from(value: String) -> Self {
        match value.to_ascii_lowercase().as_str() {
            "admin" => Role::Admin,
            "member" => Role::Member,
            _ => Role::Member,
        }
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Group {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GroupMembership {
    pub id: i32,
    pub group_id: String,
    pub group: Group,
    pub user_id: String,
    pub role: Role,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

impl GroupMembership {
    pub fn from_db(db_group_membership: &DbGroupMembership, db_group: &Group) -> Self {
        Self {
            id: db_group_membership.id,
            group_id: db_group_membership.group_id.clone(),
            group: db_group.clone(),
            user_id: db_group_membership.user_id.clone(),
            role: db_group_membership.role.clone(),
            created_at: db_group_membership.created_at,
            updated_at: db_group_membership.updated_at,
        }
    }
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

impl std::convert::From<i32> for Period {
    fn from(value: i32) -> Self {
        match value {
            0 => Self::Q1,
            1 => Self::Q2,
            2 => Self::Q3,
            3 => Self::Q4,
            _ => unreachable!(),
        }
    }
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

    pub fn current_period_info() -> anyhow::Result<(i32, Period, (NaiveDate, NaiveDate))> {
        let now = chrono::Utc::now();
        let current_year = now.year();
        let current_period = Period::from_date(now.date_naive());
        let date_range = current_period.to_date_range(current_year)?;

        Ok((current_year, current_period, date_range))
    }

    pub fn current_period_date_range() -> anyhow::Result<(NaiveDate, NaiveDate)> {
        Ok(Self::current_period_info()?.2)
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
    pub group_id: String,
    pub color: Option<String>,
}

impl Default for Rating {
    fn default() -> Self {
        let default_datetime = NaiveDateTime::default();
        Self {
            created_at: default_datetime,
            updated_at: default_datetime,
            period: Period::from_date(default_datetime.date()),
            id: Default::default(),
            restaurant_id: Default::default(),
            user_id: Default::default(),
            username: Default::default(),
            score: Default::default(),
            group_id: Default::default(),
            color: Default::default(),
        }
    }
}

impl Rating {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: i32,
        group_id: String,
        restaurant_id: String,
        user_id: String,
        username: String,
        score: f32,
        created_at: NaiveDateTime,
        updated_at: NaiveDateTime,
        color: Option<String>,
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
            group_id,
            color,
        }
    }

    pub fn from_db(db_rating: &DbRating) -> Rating {
        Self::new(
            db_rating.id,
            db_rating.group_id.clone(),
            db_rating.restaurant_id.clone(),
            db_rating.user_id.clone(),
            db_rating.username.clone(),
            db_rating.score,
            db_rating.created_at,
            db_rating.updated_at,
            db_rating.color.clone(),
        )
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RatingsByPeriod {
    pub current_year: i32,
    pub current_period: Period,
    pub current_period_ratings: Vec<Rating>,
    pub historical_ratings: Vec<AverageRatingPerPeriod>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AverageRatingPerPeriod {
    pub restaurant_id: String,
    pub year: i32,
    pub period: Period,
    pub average_score: f64,
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
