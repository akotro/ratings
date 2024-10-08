#![allow(dead_code)]

use std::{env, str::FromStr};

use anyhow::{anyhow, Error, Result};
use chrono::Utc;
use dotenvy::dotenv;
use serde_json::json;
use sqlx::{
    migrate,
    migrate::Migrator,
    mysql::{MySqlConnectOptions, MySqlQueryResult},
    pool::PoolConnection,
    Acquire, ConnectOptions, MySql, MySqlConnection, MySqlExecutor, MySqlPool, Row,
};
use uuid::Uuid;
use web_push::{
    IsahcWebPushClient, SubscriptionInfo, VapidSignatureBuilder, WebPushClient, WebPushError,
    WebPushMessageBuilder,
};

use crate::{db_models::*, models::*};

// NOTE: Database

const DB_URL: &str = "DATABASE_URL";
const PUBLIC_VAPID_PUBLIC_KEY: &str = "PUBLIC_VAPID_PUBLIC_KEY";
const VAPID_PRIVATE_KEY: &str = "VAPID_PRIVATE_KEY";

static MIGRATOR: Migrator = migrate!();

pub async fn init_database() -> Result<MySqlPool, Error> {
    dotenv().ok();
    let database_url = env::var(DB_URL).expect("DATABASE_URL must be set");

    let connect_options =
        MySqlConnectOptions::from_str(&database_url)?.log_statements(log::LevelFilter::max());
    let pool = MySqlPool::connect_with(connect_options).await?;

    MIGRATOR.run(&pool).await?;

    Ok(pool)
}

pub async fn get_connection(pool: &MySqlPool) -> Option<PoolConnection<MySql>> {
    pool.acquire().await.ok()
}

// NOTE: Users

pub async fn create_user(conn: &mut MySqlConnection, new_user: &NewUser) -> Result<User> {
    let mut tx = conn.begin().await?;

    let existing_user = get_user_by_credentials(&mut tx, &new_user.username).await;
    if existing_user.is_ok_and(|u| u.is_some()) {
        tx.rollback().await?;
        return Err(anyhow!(
            "User already exists with username: {}",
            new_user.username
        ));
    }

    let query = sqlx::query_as!(
        User,
        "INSERT INTO users (id, username, password, color) VALUES (?, ?, ?, ?)",
        new_user.id,
        new_user.username,
        new_user.password,
        new_user.color
    );
    let result = match query.execute(&mut *tx).await {
        Ok(query_result) => query_result,
        Err(err) => {
            tx.rollback().await?;
            return Err(anyhow!("Could not create user: {err}"));
        }
    };

    if result.rows_affected() == 1 {
        let db_user = match get_user_by_credentials(&mut tx, &new_user.username.clone()).await {
            Ok(Some(user)) => user,
            Ok(None) => {
                tx.rollback().await?;
                return Err(anyhow!("Could not get db_user"));
            }
            Err(err) => {
                tx.rollback().await?;
                return Err(anyhow!("Could not get db_user: {err}"));
            }
        };

        tx.commit().await?;

        Ok(User {
            id: db_user.id,
            username: db_user.username,
            password: db_user.password,
            color: db_user.color,
            token: String::new(),
            ratings: Vec::new(),
            group_memberships: Vec::new(),
        })
    } else {
        Err(anyhow::anyhow!("Failed to create user."))
    }
}

pub async fn get_users(pool: &MySqlPool) -> Result<Vec<User>> {
    let mut conn = get_connection(pool)
        .await
        .ok_or(anyhow!("Failed to get connection."))?;
    let mut tx = conn.begin().await?;

    let db_users = match sqlx::query_as!(DbUser, "SELECT id, username, password, color FROM users")
        .fetch_all(&mut *tx)
        .await
    {
        Ok(db_users) => db_users,
        Err(err) => {
            tx.rollback().await?;
            return Err(anyhow::anyhow!(err));
        }
    };

    let tasks: Vec<_> = db_users
        .into_iter()
        .map(|db_user| {
            let pool = pool.clone();
            async move {
                let mut conn = get_connection(&pool)
                    .await
                    .ok_or(anyhow!("Failed to get connection."))?;
                let mut tx = conn.begin().await?;

                let ratings = match get_ratings_by_user(&mut tx, &db_user.id).await {
                    Ok(ratings_by_period) => ratings_by_period.current_period_ratings,
                    Err(err) => {
                        tx.rollback().await?;
                        return Err(anyhow::anyhow!(err));
                    }
                };
                let group_memberships =
                    match get_group_memberships_by_user(&mut tx, &db_user.id).await {
                        Ok(group_memberships) => group_memberships,
                        Err(err) => {
                            tx.rollback().await?;
                            return Err(anyhow::anyhow!(err));
                        }
                    };

                tx.commit().await?;

                Ok(User {
                    id: db_user.id,
                    token: String::new(),
                    username: db_user.username,
                    password: db_user.password,
                    color: db_user.color,
                    ratings,
                    group_memberships,
                })
            }
        })
        .collect();

    let results: Result<Vec<User>> = futures::future::join_all(tasks).await.into_iter().collect();

    tx.commit().await?;

    results
}

pub async fn get_user_by_credentials(
    conn: &mut MySqlConnection,
    username: &str,
) -> Result<Option<User>> {
    let mut tx = Acquire::begin(conn).await?;

    let query = sqlx::query_as!(
        DbUser,
        "SELECT id, username, password, color FROM users WHERE username = ?",
        username
    );
    let db_user_result = match query.fetch_optional(&mut *tx).await {
        Ok(query_result) => query_result,
        Err(err) => {
            tx.rollback().await?;
            return Err(anyhow!("User not found: {err}"));
        }
    };

    match db_user_result {
        Some(db_user) => {
            let ratings = match get_ratings_by_user(&mut tx, &db_user.id).await {
                Ok(ratings_by_period) => ratings_by_period.current_period_ratings,
                Err(err) => {
                    tx.rollback().await?;
                    return Err(anyhow!("User's ratings not found: {err}"));
                }
            };

            let group_memberships = match get_group_memberships_by_user(&mut tx, &db_user.id).await
            {
                Ok(query_result) => query_result,
                Err(err) => {
                    tx.rollback().await?;
                    return Err(anyhow!("User's group memberships not found: {err}"));
                }
            };

            tx.commit().await?;

            Ok(Some(User {
                id: db_user.id,
                token: String::new(),
                username: db_user.username,
                password: db_user.password,
                color: db_user.color,
                ratings,
                group_memberships,
            }))
        }
        None => {
            tx.rollback().await?;
            Err(anyhow!("User not found"))
        }
    }
}

pub async fn update_user(
    conn: &mut MySqlConnection,
    user_id: &str,
    user: &NewUser,
) -> Result<User> {
    let mut tx = conn.begin().await?;

    let existing_user = get_user_by_credentials(&mut tx, &user.username).await;
    if existing_user.is_ok_and(|u| u.is_some()) {
        tx.rollback().await?;
        return Err(anyhow!(
            "User already exists with username: {}",
            user.username
        ));
    }

    let _ = match sqlx::query!(
        "UPDATE users
         SET username = ?, color = ?
         WHERE id = ?;",
        user.username,
        user.color,
        user_id
    )
    .execute(&mut *tx)
    .await
    {
        Ok(query_result) => query_result,
        Err(err) => {
            tx.rollback().await?;
            return Err(anyhow!("Could not update user: {err}"));
        }
    };

    let updated_user = match get_user_by_credentials(&mut tx, &user.username).await {
        Ok(updated_user_option) => match updated_user_option {
            Some(updated_user) => updated_user,
            None => {
                tx.rollback().await?;
                return Err(anyhow!("Could not get updated user"));
            }
        },
        Err(err) => {
            tx.rollback().await?;
            return Err(anyhow!("Could not get updated user: {err}"));
        }
    };

    tx.commit().await?;

    Ok(updated_user)
}

pub async fn delete_user(conn: &mut MySqlConnection, user_id: &str) -> Result<MySqlQueryResult> {
    let result = sqlx::query!("DELETE FROM users WHERE id = ?", user_id)
        .execute(conn)
        .await?;

    Ok(result)
}

// NOTE: Push Notifications

pub fn init_push_notifications() -> Result<PushClient, Error> {
    dotenv().ok();
    let vapid_public_key =
        env::var(PUBLIC_VAPID_PUBLIC_KEY).expect("PUBLIC_VAPID_PUBLIC_KEY must be set");
    let vapid_private_key = env::var(VAPID_PRIVATE_KEY).expect("VAPID_PRIVATE_KEY must be set");

    let client = IsahcWebPushClient::new()?;

    Ok(PushClient {
        vapid_public_key,
        vapid_private_key,
        client,
    })
}

pub async fn create_push_subscription(
    conn: &mut MySqlConnection,
    user_id: &str,
    subscription_info: &SubscriptionInfo,
) -> Result<PushSubscription> {
    let mut tx = conn.begin().await?;

    let query = sqlx::query_as!(
        PushSubscription,
        "INSERT INTO push_subscriptions (endpoint, user_id, p256dh, auth)
         VALUES (?, ?, ?, ?)
         ON DUPLICATE KEY UPDATE
         p256dh = VALUES(p256dh), auth = VALUES(auth)",
        subscription_info.endpoint,
        user_id,
        subscription_info.keys.p256dh,
        subscription_info.keys.auth,
    );
    let result = match query.execute(&mut *tx).await {
        Ok(query_result) => query_result,
        Err(err) => {
            tx.rollback().await?;
            return Err(anyhow!("Could not create push subscription: {err}"));
        }
    };

    if result.rows_affected() > 0 {
        let push_subscription =
            match get_push_subscription(&mut tx, &subscription_info.endpoint).await {
                Ok(push_subscription) => push_subscription,
                Err(err) => {
                    tx.rollback().await?;
                    return Err(anyhow!("Could not get push subscription: {err}"));
                }
            };

        tx.commit().await?;

        Ok(push_subscription)
    } else {
        Err(anyhow::anyhow!("Failed to create push subscription"))
    }
}

pub async fn get_push_subscription(
    conn: &mut MySqlConnection,
    endpoint: &str,
) -> Result<PushSubscription> {
    let query = sqlx::query_as!(
        PushSubscription,
        "SELECT endpoint, user_id, p256dh, auth
         FROM push_subscriptions
         WHERE endpoint = ?",
        endpoint,
    );
    let push_subscription_result = query.fetch_one(conn).await;

    match push_subscription_result {
        Ok(push_subscription) => Ok(push_subscription),
        Err(err) => Err(anyhow!("Push subscription not found: {err}")),
    }
}

pub async fn delete_push_subscription(
    conn: &mut MySqlConnection,
    endpoint: &str,
) -> Result<MySqlQueryResult> {
    let result = sqlx::query!(
        "DELETE FROM push_subscriptions WHERE endpoint = ?",
        endpoint
    )
    .execute(conn)
    .await?;

    Ok(result)
}

pub async fn create_rating_notification(
    conn: &mut MySqlConnection,
    restaurant_id: &str,
    group_id: &str,
) -> Result<RatingNotification> {
    let mut tx = conn.begin().await?;

    let query = sqlx::query_as!(
        RatingNotification,
        "INSERT INTO rating_notifications (restaurant_id, group_id)
         VALUES(?, ?)",
        restaurant_id,
        group_id,
    );
    let result = match query.execute(&mut *tx).await {
        Ok(query_result) => query_result,
        Err(err) => {
            tx.rollback().await?;
            return Err(anyhow!("Could not create rating notification: {err}"));
        }
    };

    if result.rows_affected() > 0 {
        let last_insert_id = result.last_insert_id();

        let rating_notification =
            match get_rating_notification_by_id(&mut tx, &(last_insert_id as i32)).await {
                Ok(rating_notification) => rating_notification,
                Err(err) => {
                    tx.rollback().await?;
                    return Err(anyhow!("Could not get push subscription: {err}"));
                }
            };

        tx.commit().await?;

        Ok(rating_notification)
    } else {
        Err(anyhow::anyhow!("Failed to create rating notification"))
    }
}

pub async fn get_rating_notification_by_id(
    conn: &mut MySqlConnection,
    id: &i32,
) -> Result<RatingNotification> {
    let query = sqlx::query_as!(
        RatingNotification,
        "SELECT id, restaurant_id, group_id, notified_at
         FROM rating_notifications
         WHERE id = ?",
        id
    );
    let rating_notification_result = query.fetch_one(conn).await;

    match rating_notification_result {
        Ok(rating_notification) => Ok(rating_notification),
        Err(err) => Err(anyhow!("Rating notification not found: {err}")),
    }
}

pub async fn rating_notification_sent(
    conn: &mut MySqlConnection,
    restaurant_id: &str,
    group_id: &str,
) -> Result<bool> {
    let date_range = Period::current_period_date_range()?;

    let rating_notification_exists = match sqlx::query_scalar!(
        "SELECT EXISTS (
            SELECT 1
            FROM rating_notifications
            WHERE restaurant_id = ? AND group_id = ? AND notified_at >= ? AND notified_at <= ?
         ) as rating_notification_exists",
        restaurant_id,
        group_id,
        date_range.0,
        date_range.1
    )
    .fetch_one(conn)
    .await
    {
        Ok(exists_result) => exists_result,
        Err(err) => {
            return Err(anyhow!(
                "Could not check if rating notification has been sent: {err}"
            ));
        }
    };

    if rating_notification_exists == 1 {
        return Ok(true);
    }

    Ok(false)
}

pub async fn send_notification_to_group(
    push_client: &PushClient,
    pool: &MySqlPool,
    group_id: &str,
    message: &str,
) -> Result<()> {
    let push_subscriptions_query = sqlx::query!(
        "SELECT DISTINCT ps.*, g.name as group_name
         FROM group_memberships gm
         JOIN groups g on g.id = gm.group_id
         JOIN push_subscriptions ps ON ps.user_id = gm.user_id
         WHERE gm.group_id = ?",
        group_id,
    );

    let mut conn = get_connection(pool).await.unwrap();
    let push_subscriptions = match push_subscriptions_query.fetch_all(&mut *conn).await {
        Ok(result) => result,
        Err(err) => return Err(anyhow!("Could not get push_subscriptions: {err}")),
    };

    let futures = push_subscriptions.into_iter().map(|push_subscription| {
        let push_client = push_client.clone();
        let subscription_info = SubscriptionInfo::new(
            push_subscription.endpoint,
            push_subscription.p256dh,
            push_subscription.auth,
        );
        let body = format!("{}: {}", push_subscription.group_name, message);

        let new_pool = pool.clone();
        actix_web::rt::spawn(async move {
            let mut conn = get_connection(&new_pool).await.unwrap();

            if let Err(err) =
                send_notification(&mut conn, &push_client, &subscription_info, &body).await
            {
                eprintln!(
                    "ERROR: failed sending notification to {}: {err}",
                    subscription_info.endpoint
                );
            }
        })
    });

    futures::future::join_all(futures).await;

    Ok(())
}

pub async fn send_notification(
    conn: &mut MySqlConnection,
    push_client: &PushClient,
    subscription_info: &SubscriptionInfo,
    body: &str,
) -> Result<()> {
    let vapid_builder = VapidSignatureBuilder::from_base64_no_sub(
        &push_client.vapid_private_key,
        web_push::URL_SAFE_NO_PAD,
    )?;

    let vapid_signature = vapid_builder
        .clone()
        .add_sub_info(subscription_info)
        .build()?;

    let mut push_builder = WebPushMessageBuilder::new(subscription_info);

    push_builder.set_vapid_signature(vapid_signature.clone());

    let payload = json!({
        "body": body
    })
    .to_string();
    push_builder.set_payload(web_push::ContentEncoding::Aes128Gcm, payload.as_bytes());

    let response = push_client.client.send(push_builder.build()?).await;

    match response {
        Ok(_) => Ok(()),
        Err(err) => {
            let should_remove = matches!(
                err,
                WebPushError::EndpointNotValid | WebPushError::EndpointNotFound
            );

            if should_remove {
                delete_push_subscription(conn, &subscription_info.endpoint).await?;
            }

            Err(anyhow!("Error sending notification: {err}"))
        }
    }
}

// NOTE: Groups

pub async fn create_group(
    conn: &mut MySqlConnection,
    new_group: &NewGroup,
) -> Result<GroupMembership> {
    let mut tx = conn.begin().await?;

    let id = Uuid::new_v4().to_string();
    let created_at = Utc::now().naive_utc();
    let updated_at = created_at;

    let group_query = sqlx::query_as!(
        Group,
        "INSERT INTO groups (id, name, description, created_at, updated_at) VALUES (?, ?, ?, ?, ?)",
        id,
        new_group.name,
        new_group.description,
        created_at,
        updated_at,
    );

    let result = match group_query.execute(&mut *tx).await {
        Ok(query_result) => query_result,
        Err(err) => {
            tx.rollback().await?;
            return Err(anyhow!("Could not create group: {err}"));
        }
    };

    if result.rows_affected() == 1 {
        let db_group = match get_group(&mut tx, &id).await {
            Ok(group) => group,
            Err(err) => {
                tx.rollback().await?;
                return Err(anyhow!("Could not get db group: {err}"));
            }
        };

        let db_group_membership = match create_group_membership(
            &mut tx,
            &NewGroupMembership {
                group_id: db_group.id.clone(),
                user_id: new_group.creator_id.to_owned(),
                role: Role::Admin,
            },
        )
        .await
        {
            Ok(mut group_membership) => {
                group_membership.group = db_group;
                group_membership
            }
            Err(err) => {
                tx.rollback().await?;
                return Err(anyhow!("Could not get db group membership: {err}"));
            }
        };

        tx.commit().await?;

        Ok(db_group_membership)
    } else {
        Err(anyhow::anyhow!("Failed to create group"))
    }
}

pub async fn get_group(conn: &mut MySqlConnection, id: &str) -> Result<Group> {
    let query = sqlx::query_as!(
        Group,
        "SELECT id, name, description, created_at, updated_at
         FROM groups
         WHERE id = ?",
        id,
    );
    let group = query.fetch_optional(conn).await?;

    match group {
        Some(group) => Ok(group),
        None => Err(anyhow!("Group not found")),
    }
}

pub async fn get_group_by_membership(
    conn: &mut MySqlConnection,
    membership_id: &i32,
) -> Result<Group> {
    let query = sqlx::query_as!(
        Group,
        "SELECT g.id, g.name, g.description, g.created_at, g.updated_at
         FROM groups g
         JOIN group_memberships gm ON gm.group_id = g.id
         WHERE gm.id = ?",
        membership_id,
    );
    let group = query.fetch_optional(conn).await?;

    match group {
        Some(group) => Ok(group),
        None => Err(anyhow!("Group not found")),
    }
}

pub async fn delete_group(conn: &mut MySqlConnection, id: &str) -> Result<MySqlQueryResult> {
    let result = sqlx::query!("DELETE FROM groups WHERE id = ?", id)
        .execute(conn)
        .await?;

    Ok(result)
}

pub async fn create_group_membership(
    conn: &mut MySqlConnection,
    new_group_membership: &NewGroupMembership,
) -> Result<GroupMembership> {
    let mut tx = conn.begin().await?;

    let created_at = Utc::now().naive_utc();
    let updated_at = created_at;

    if check_group_membership_exists(
        &mut tx,
        &new_group_membership.user_id,
        &new_group_membership.group_id,
    )
    .await?
    {
        tx.rollback().await?;
        return Err(anyhow!("Group membership already exists"));
    }

    let membership_query = sqlx::query_as!(
        GroupMembership,
        "INSERT INTO group_memberships (group_id, user_id, role, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?)",
        new_group_membership.group_id,
        new_group_membership.user_id,
        new_group_membership.role,
        created_at,
        updated_at
    );

    let result = match membership_query.execute(&mut *tx).await {
        Ok(query_result) => query_result,
        Err(err) => {
            tx.rollback().await?;
            return Err(anyhow!("Could not create group membership: {err}"));
        }
    };

    if result.rows_affected() == 1 {
        let membership_id = result.last_insert_id();

        let db_group_membership = match get_group_membership(&mut tx, &(membership_id as i32)).await
        {
            Ok(group_membership) => group_membership,
            Err(err) => {
                tx.rollback().await?;
                return Err(anyhow!("Could not get db group membership: {err}"));
            }
        };

        tx.commit().await?;

        Ok(db_group_membership)
    } else {
        Err(anyhow::anyhow!("Failed to create group membership"))
    }
}

pub async fn check_group_membership_exists(
    conn: &mut MySqlConnection,
    user_id: &str,
    group_id: &str,
) -> Result<bool> {
    let group_membership_exists_query = sqlx::query_scalar!(
        "SELECT EXISTS (
            SELECT 1
            FROM group_memberships
            WHERE user_id = ? AND group_id = ?
         ) as group_membership_exists;",
        user_id,
        group_id
    );
    let group_membership_exists = match group_membership_exists_query.fetch_one(&mut *conn).await {
        Ok(query_result) => query_result,
        Err(err) => {
            return Err(anyhow!("Could not validate group membership: {err}"));
        }
    };

    if group_membership_exists == 1 {
        return Ok(true);
    }

    Ok(false)
}

pub async fn get_group_membership(conn: &mut MySqlConnection, id: &i32) -> Result<GroupMembership> {
    let mut tx = conn.begin().await?;

    let query = sqlx::query_as!(
        DbGroupMembership,
        "SELECT id, group_id, user_id, role, created_at, updated_at
         FROM group_memberships
         WHERE id = ?",
        id,
    );
    let group_membership = query.fetch_optional(&mut *tx).await?;

    match group_membership {
        Some(group_membership) => {
            let db_group = match get_group_by_membership(&mut tx, &group_membership.id).await {
                Ok(group) => group,
                Err(err) => {
                    tx.rollback().await?;
                    return Err(anyhow!("Could not get db group from membership: {err}"));
                }
            };

            tx.commit().await?;

            Ok(GroupMembership::from_db(&group_membership, &db_group))
        }
        None => Err(anyhow!("Group membership not found")),
    }
}

pub async fn get_group_memberships_by_user(
    conn: &mut MySqlConnection,
    user_id: &str,
) -> Result<Vec<GroupMembership>> {
    let mut tx = conn.begin().await?;

    let db_group_memberships = match sqlx::query_as!(
        DbGroupMembership,
        "SELECT id, group_id, user_id, role, created_at, updated_at
         FROM group_memberships
         WHERE user_id = ?",
        user_id,
    )
    .fetch_all(&mut *tx)
    .await
    {
        Ok(result) => result,
        Err(err) => {
            tx.rollback().await?;
            return Err(anyhow!("Group membership not found: {err}"));
        }
    };

    let mut group_memberships = Vec::new();

    for db_group_membership in db_group_memberships {
        let db_group = match get_group_by_membership(&mut tx, &db_group_membership.id).await {
            Ok(group) => group,
            Err(err) => {
                tx.rollback().await?;
                return Err(anyhow!("Could not get db group from membership: {err}"));
            }
        };

        group_memberships.push(GroupMembership::from_db(&db_group_membership, &db_group));
    }

    tx.commit().await?;

    Ok(group_memberships)
}

pub async fn delete_group_membership(
    conn: &mut MySqlConnection,
    id: &i32,
) -> Result<MySqlQueryResult> {
    let result = sqlx::query!("DELETE FROM group_memberships WHERE id = ?", id)
        .execute(conn)
        .await?;

    Ok(result)
}

// NOTE: Restaurants

pub async fn create_restaurant(
    conn: &mut MySqlConnection,
    restaurant: &Restaurant,
) -> Result<Restaurant> {
    let mut tx = conn.begin().await?;

    let query = sqlx::query_as!(
        Restaurant,
        "INSERT INTO restaurants (id, cuisine) VALUES (?, ?)",
        restaurant.id,
        restaurant.cuisine
    );
    let result = match query.execute(&mut *tx).await {
        Ok(query_result) => query_result,
        Err(err) => {
            tx.rollback().await?;
            return Err(anyhow!("Could not create restaurant: {err}"));
        }
    };

    if result.rows_affected() == 1 {
        let db_restaurant = match get_restaurant(&mut tx, &restaurant.id).await {
            Ok(restaurant) => restaurant,
            Err(err) => {
                tx.rollback().await?;
                return Err(anyhow!("Could not get db restaurant: {err}"));
            }
        };

        tx.commit().await?;

        Ok(db_restaurant)
    } else {
        Err(anyhow::anyhow!("Failed to create restaurant."))
    }
}

pub async fn get_restaurants(conn: &mut MySqlConnection) -> Result<Vec<Restaurant>> {
    let query = sqlx::query!("SELECT id, cuisine FROM restaurants");
    let rows = query.fetch_all(conn).await?;

    let restaurants = rows
        .into_iter()
        .map(|row| Restaurant {
            id: row.id,
            cuisine: row.cuisine,
            menu: Vec::<MenuItem>::new(),
        })
        .collect();

    Ok(restaurants)
}

pub async fn get_restaurant(conn: &mut MySqlConnection, restaurant_id: &str) -> Result<Restaurant> {
    let mut tx = Acquire::begin(conn).await?;

    let query = sqlx::query_as!(
        DbRestaurant,
        "SELECT id, cuisine FROM restaurants WHERE id = ?",
        restaurant_id
    );
    let db_restaurant_result = match query.fetch_optional(&mut *tx).await {
        Ok(query_result) => query_result,
        Err(err) => {
            tx.rollback().await?;
            return Err(anyhow!("Restaurant not found: {err}"));
        }
    };

    match db_restaurant_result {
        Some(db_restaurant) => {
            let menu_items = match get_menu_items(&mut tx, &db_restaurant.id).await {
                Ok(query_result) => query_result,
                Err(err) => {
                    tx.rollback().await?;
                    return Err(anyhow!("Restaurants's menu items not found: {err}"));
                }
            };
            Ok(Restaurant {
                id: db_restaurant.id,
                cuisine: db_restaurant.cuisine,
                menu: menu_items,
            })
        }
        None => Err(anyhow!("Restaurant not found")),
    }
}

pub async fn update_restaurant(
    conn: &mut MySqlConnection,
    restaurant_id: &str,
    restaurant: &Restaurant,
) -> Result<MySqlQueryResult> {
    let result = sqlx::query!(
        "UPDATE restaurants
         SET id = ?, cuisine = ?
         WHERE id = ?;",
        restaurant.id,
        restaurant.cuisine,
        restaurant_id
    )
    .execute(conn)
    .await?;

    Ok(result)
}

pub async fn delete_restaurant(
    conn: &mut MySqlConnection,
    restaurant_id: &str,
) -> Result<MySqlQueryResult> {
    let result = sqlx::query!("DELETE FROM restaurants WHERE id = ?", restaurant_id)
        .execute(conn)
        .await?;

    Ok(result)
}

pub async fn is_restaurant_rating_complete(
    pool: &MySqlPool,
    push_client: Option<&PushClient>,
    restaurant_id: &str,
    group_id: &str,
) -> Result<bool> {
    let mut conn = get_connection(pool).await.unwrap();
    let mut tx = conn.begin().await?;

    let date_range = Period::current_period_date_range()?;

    let result = match sqlx::query_scalar!(
        "SELECT
            (
                SELECT COUNT(*)
                FROM group_memberships
                WHERE group_id = ?
            ) = (
                SELECT COUNT(*)
                FROM ratings
                WHERE group_id = ? AND restaurant_id = ? AND created_at >= ? AND created_at <= ?
            ) AS is_complete;",
        group_id,
        group_id,
        restaurant_id,
        date_range.0,
        date_range.1
    )
    .fetch_one(&mut *tx)
    .await
    {
        Ok(result) => result,
        Err(err) => {
            tx.rollback().await?;
            return Err(anyhow!(
                "Could not check if restaurant rating is complete: {err}"
            ));
        }
    };

    let is_complete = match result {
        Some(is_complete) => is_complete == 1,
        None => {
            tx.rollback().await?;
            return Err(anyhow!("Failed to check if restaurant rating is complete."));
        }
    };

    if is_complete && push_client.is_some() {
        let notification_sent =
            match rating_notification_sent(&mut tx, restaurant_id, group_id).await {
                Ok(notification_sent) => notification_sent,
                Err(err) => {
                    tx.rollback().await?;
                    return Err(err);
                }
            };

        if !notification_sent {
            if let Err(err) = create_rating_notification(&mut tx, restaurant_id, group_id).await {
                tx.rollback().await?;
                return Err(err);
            }

            tx.commit().await?;

            let push_client = push_client.unwrap().clone();
            let group_id = group_id.to_string();
            let restaurant_id = restaurant_id.to_string();
            let new_pool = pool.clone();

            actix_web::rt::spawn(async move {
                if let Err(err) = send_notification_to_group(
                    &push_client,
                    &new_pool,
                    &group_id,
                    &format!("{restaurant_id} ratings are complete!"),
                )
                .await
                {
                    eprintln!("ERROR: Failed to send notification to group {group_id} for restaurant {restaurant_id}: {err}");
                }
            });
        } else {
            tx.commit().await?;
        }
    } else {
        tx.commit().await?;
    }

    Ok(is_complete)
}

pub async fn get_avg_rating(
    pool: &MySqlPool,
    restaurant_id: &str,
    group_id: &str,
) -> Result<Option<f64>> {
    if is_restaurant_rating_complete(pool, None, restaurant_id, group_id).await? {
        let mut conn = get_connection(pool).await.unwrap();

        let date_range = Period::current_period_date_range()?;

        let avg_rating: Option<f64> = sqlx::query_scalar!(
            "SELECT AVG(score)
             FROM ratings
             WHERE group_id = ? and restaurant_id = ? AND created_at >= ? AND created_at <= ?",
            group_id,
            restaurant_id,
            date_range.0,
            date_range.1
        )
        .fetch_one(&mut *conn)
        .await?;

        Ok(avg_rating)
    } else {
        Ok(None)
    }
}

pub async fn get_restaurants_with_avg_rating(
    conn: &mut MySqlConnection,
    group_id: &str,
) -> Result<Vec<(Restaurant, f64)>> {
    let mut tx = Acquire::begin(conn).await?;

    let date_range = Period::current_period_date_range()?;

    let db_restaurants_with_avg_rating_result = sqlx::query!(
        "SELECT *
         FROM (
            SELECT r.id, r.cuisine,
                IF(
                    (
                        SELECT COUNT(*) FROM group_memberships gm
                        WHERE gm.group_id = ?
                    ) = (
                        SELECT COUNT(*) FROM ratings ra2
                        WHERE ra2.group_id = ? AND ra2.restaurant_id = r.id AND ra2.created_at >= ? AND ra2.created_at <= ?
                    ),
                    AVG(ra.score),
                    NULL
                ) AS avg_rating,
                COUNT(ra.score) AS num_ratings,
                (
                    SELECT COUNT(*) FROM ratings ra3
                    WHERE ra3.group_id = ? AND ra3.restaurant_id = r.id AND ra3.created_at >= ? AND ra3.created_at <= ?
                ) AS has_any_rating
             FROM restaurants r
             LEFT JOIN ratings ra ON ra.group_id = ? AND ra.restaurant_id = r.id AND ra.created_at >= ? AND ra.created_at <= ?
             GROUP BY r.id
         ) AS subquery
         ORDER BY
            avg_rating IS NULL ASC,
            has_any_rating DESC,
            avg_rating DESC,
            id",
        group_id,
        group_id,
        date_range.0,
        date_range.1,
        group_id,
        date_range.0,
        date_range.1,
        group_id,
        date_range.0,
        date_range.1,
    )
    .fetch_all(&mut *tx)
    .await;

    let db_restaurants_with_avg_rating_result = match db_restaurants_with_avg_rating_result {
        Ok(rows) => {
            let mapped_rows: Result<Vec<(DbRestaurant, Option<f64>)>, Error> = rows
                .into_iter()
                .map(|row| {
                    Ok((
                        DbRestaurant {
                            id: row.id,
                            cuisine: row.cuisine,
                        },
                        row.avg_rating,
                    ))
                })
                .collect();

            mapped_rows
        }
        Err(e) => {
            tx.rollback().await?;
            return Err(e.into());
        }
    };

    let mut results = Vec::new();

    for (db_restaurant, avg_rating) in db_restaurants_with_avg_rating_result? {
        let menu_result = get_menu_items(&mut tx, &db_restaurant.id).await;
        if menu_result.is_err() {
            tx.rollback().await?;
            return Err(menu_result.err().unwrap());
        }
        let menu = menu_result.unwrap_or_default();

        results.push((
            Restaurant {
                id: db_restaurant.id.clone(),
                cuisine: db_restaurant.cuisine.clone(),
                menu,
            },
            avg_rating.unwrap_or(0.0),
        ));
    }

    tx.commit().await?;

    Ok(results)
}

// NOTE: Menu Items

pub async fn create_menu_item(
    conn: &mut MySqlConnection,
    menu_item: &MenuItem,
) -> Result<MenuItem> {
    let mut tx = conn.begin().await?;

    let menu_item_result = sqlx::query("INSERT INTO menu_items (name, price) VALUES (?, ?)")
        .bind(&menu_item.name)
        .bind(menu_item.price)
        .execute(&mut *tx)
        .await?;

    let last_insert_id = menu_item_result.last_insert_id();

    sqlx::query("INSERT INTO restaurant_menu_items (restaurant_id, menu_item_id) VALUES (?, ?)")
        .bind(&menu_item.restaurant_id)
        .bind(last_insert_id)
        .execute(&mut *tx)
        .await?;

    if menu_item_result.rows_affected() == 1 {
        tx.commit().await?;

        Ok(MenuItem {
            id: last_insert_id as i32,
            name: menu_item.name.clone(),
            price: menu_item.price,
            restaurant_id: menu_item.restaurant_id.clone(),
        })
    } else {
        tx.rollback().await?;
        Err(anyhow!("Failed to create menu item."))
    }
}

async fn get_menu_items(pool: &mut MySqlConnection, restaurant_id: &str) -> Result<Vec<MenuItem>> {
    let mut menu_items = Vec::new();

    let query = r#"
        SELECT mi.id, mi.name, mi.price
        FROM restaurant_menu_items rmi
        INNER JOIN menu_items mi ON rmi.menu_item_id = mi.id
        WHERE rmi.restaurant_id = ?
    "#;

    let rows = sqlx::query(query)
        .bind(restaurant_id)
        .fetch_all(pool)
        .await?;

    for row in rows {
        let menu_item = MenuItem {
            id: row.get("id"),
            restaurant_id: restaurant_id.to_owned(),
            name: row.get("name"),
            price: row.get("price"),
        };
        menu_items.push(menu_item);
    }

    Ok(menu_items)
}

pub async fn delete_menu_item(
    conn: &mut MySqlConnection,
    menu_item_id: i32,
) -> Result<MySqlQueryResult> {
    let result = sqlx::query!("DELETE FROM menu_items WHERE id = ?", menu_item_id)
        .execute(conn)
        .await?;

    Ok(result)
}

// NOTE: Ratings

pub async fn create_rating(conn: &mut MySqlConnection, rating: &NewRating) -> Result<Rating> {
    let mut tx = conn.begin().await?;

    let created_at = Utc::now().naive_utc();
    let updated_at = created_at;

    if !check_group_membership_exists(&mut tx, &rating.user_id, &rating.group_id).await? {
        tx.rollback().await?;
        return Err(anyhow!("User does not belong to group"));
    }

    let query = sqlx::query_as!(
        Rating,
        "INSERT INTO ratings (group_id, restaurant_id, user_id, username, score, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
        rating.group_id,
        rating.restaurant_id,
        rating.user_id,
        rating.username,
        rating.score,
        created_at,
        updated_at
    );
    let result = match query.execute(&mut *tx).await {
        Ok(query_result) => query_result,
        Err(err) => {
            tx.rollback().await?;
            return Err(anyhow::anyhow!(err));
        }
    };

    if result.rows_affected() == 1 {
        let last_insert_id = result.last_insert_id();

        tx.commit().await?;

        Ok(Rating::new(
            last_insert_id as i32,
            rating.group_id.clone(),
            rating.restaurant_id.clone(),
            rating.user_id.clone(),
            rating.username.clone(),
            rating.score,
            created_at,
            updated_at,
            None,
        ))
    } else {
        tx.rollback().await?;
        Err(anyhow::anyhow!("Failed to create rating."))
    }
}

pub async fn get_ratings_by_user(
    conn: &mut MySqlConnection,
    user_id: &str,
) -> Result<RatingsByPeriod> {
    let mut tx = conn.begin().await?;

    let (current_year, current_period, date_range) = Period::current_period_info()?;

    let current_period_query = sqlx::query_as!(
        DbRating,
        "SELECT r.id, r.group_id, r.restaurant_id, r.user_id, r.score, r.username, r.created_at, r.updated_at, u.color
         FROM ratings r
         JOIN users u on u.id = r.user_id
         WHERE r.user_id = ? AND r.created_at >= ? AND r.created_at <= ?",
        user_id,
        date_range.0,
        date_range.1
    );
    let current_period_ratings = match current_period_query.fetch_all(&mut *tx).await {
        Ok(current_period_ratings) => current_period_ratings,
        Err(err) => {
            tx.rollback().await?;
            return Err(anyhow::anyhow!(err));
        }
    };
    let current_period_ratings = current_period_ratings.iter().map(Rating::from_db).collect();

    let historical_average_ratings_query = sqlx::query_as!(
        AverageRatingPerPeriod,
        "SELECT
             r.restaurant_id,
             IFNULL(YEAR(r.created_at), 0) as year,
             IFNULL(
                 CASE
                     WHEN MONTH(r.created_at) BETWEEN 1 AND 3 THEN 0
                     WHEN MONTH(r.created_at) BETWEEN 4 AND 6 THEN 1
                     WHEN MONTH(r.created_at) BETWEEN 7 AND 9 THEN 2
                     ELSE 3
                 END,
                 0
             ) as period,
             IFNULL(AVG(r.score), 0) as average_score
         FROM ratings r
         WHERE r.user_id = ?
         GROUP BY r.restaurant_id, year, period
         ORDER BY year ASC, period ASC",
        user_id,
    );
    let historical_ratings = match historical_average_ratings_query.fetch_all(&mut *tx).await {
        Ok(rows) => rows,
        Err(err) => {
            tx.rollback().await?;
            return Err(anyhow::anyhow!(err));
        }
    };

    let ratings_by_period = RatingsByPeriod {
        current_year,
        current_period,
        current_period_ratings,
        historical_ratings,
    };

    tx.commit().await?;

    Ok(ratings_by_period)
}

pub async fn get_ratings_by_user_and_group(
    conn: &mut MySqlConnection,
    user_id: &str,
    group_id: &str,
) -> Result<RatingsByPeriod> {
    let mut tx = conn.begin().await?;

    let (current_year, current_period, date_range) = Period::current_period_info()?;

    let query = sqlx::query_as!(
        DbRating,
        "SELECT r.id, r.group_id, r.restaurant_id, r.user_id, r.score, r.username, r.created_at, r.updated_at, u.color
         FROM ratings r
         JOIN users u on u.id = r.user_id
         WHERE user_id = ? and group_id = ? AND created_at >= ? AND created_at <= ?",
        user_id,
        group_id,
        date_range.0,
        date_range.1
    );
    let current_period_ratings = match query.fetch_all(&mut *tx).await {
        Ok(current_period_ratings) => current_period_ratings,
        Err(err) => {
            tx.rollback().await?;
            return Err(anyhow::anyhow!(err));
        }
    };
    let current_period_ratings = current_period_ratings.iter().map(Rating::from_db).collect();

    let historical_average_ratings_query = sqlx::query_as!(
        AverageRatingPerPeriod,
        "SELECT
             r.restaurant_id,
             IFNULL(YEAR(r.created_at), 0) as year,
             IFNULL(
                 CASE
                     WHEN MONTH(r.created_at) BETWEEN 1 AND 3 THEN 0
                     WHEN MONTH(r.created_at) BETWEEN 4 AND 6 THEN 1
                     WHEN MONTH(r.created_at) BETWEEN 7 AND 9 THEN 2
                     ELSE 3
                 END,
                 0
             ) as period,
             IFNULL(AVG(r.score), 0) as average_score
         FROM ratings r
         WHERE r.user_id = ? AND r.group_id = ?
         GROUP BY r.restaurant_id, year, period
         ORDER BY year ASC, period ASC;",
        user_id,
        group_id,
    );
    let historical_ratings = match historical_average_ratings_query.fetch_all(&mut *tx).await {
        Ok(rows) => rows,
        Err(err) => {
            tx.rollback().await?;
            return Err(anyhow::anyhow!(err));
        }
    };

    let ratings_by_period = RatingsByPeriod {
        current_year,
        current_period,
        current_period_ratings,
        historical_ratings,
    };

    tx.commit().await?;

    Ok(ratings_by_period)
}

pub async fn get_ratings_by_restaurant(
    conn: &mut MySqlConnection,
    restaurant_id: &str,
    group_id: &str,
) -> Result<RatingsByPeriod> {
    let mut tx = conn.begin().await?;

    let (current_year, current_period, _) = Period::current_period_info()?;

    let current_period_ratings = match get_ratings_by_restaurant_per_period(
        &mut tx,
        group_id,
        restaurant_id,
        current_year,
        &current_period,
    )
    .await
    {
        Ok(current_period_ratings) => current_period_ratings,
        Err(err) => {
            tx.rollback().await?;
            return Err(anyhow::anyhow!(err));
        }
    };

    let historical_average_ratings_query = sqlx::query_as!(
        AverageRatingPerPeriod,
        "SELECT
            r.restaurant_id,
            IFNULL(YEAR(r.created_at), 0) as year,
            IFNULL(
                CASE
                    WHEN MONTH(r.created_at) BETWEEN 1 AND 3 THEN 0
                    WHEN MONTH(r.created_at) BETWEEN 4 AND 6 THEN 1
                    WHEN MONTH(r.created_at) BETWEEN 7 AND 9 THEN 2
                    ELSE 3
                END,
                0
            ) as period,
            IFNULL(AVG(r.score), 0) as average_score
         FROM ratings r
         WHERE r.group_id = ? AND r.restaurant_id = ?
         GROUP BY year, period
         ORDER BY year ASC, period ASC",
        group_id,
        restaurant_id,
    );
    let historical_ratings = match historical_average_ratings_query.fetch_all(&mut *tx).await {
        Ok(rows) => rows,
        Err(err) => {
            tx.rollback().await?;
            return Err(anyhow::anyhow!(err));
        }
    };

    let ratings_by_period = RatingsByPeriod {
        current_year,
        current_period,
        current_period_ratings,
        historical_ratings,
    };

    tx.commit().await?;

    Ok(ratings_by_period)
}

pub async fn get_ratings_by_restaurant_per_period(
    conn: &mut MySqlConnection,
    group_id: &str,
    restaurant_id: &str,
    year: i32,
    period: &Period,
) -> Result<Vec<Rating>> {
    let date_range = period.to_date_range(year)?;

    let query = sqlx::query_as!(
        DbRating,
        "SELECT r.id, r.group_id, r.restaurant_id, r.user_id, r.score, r.username, r.created_at, r.updated_at, u.color
         FROM ratings r
         JOIN users u on u.id = r.user_id
         WHERE group_id = ? and restaurant_id = ? AND created_at >= ? AND created_at <= ?",
        group_id,
        restaurant_id,
        date_range.0,
        date_range.1
    );
    let ratings = query
        .fetch_all(conn)
        .await?
        .iter()
        .map(Rating::from_db)
        .collect();

    Ok(ratings)
}

pub async fn get_rating_by_restaurant(
    conn: &mut MySqlConnection,
    user_id: &str,
    group_id: &str,
    restaurant_id: &str,
) -> Result<Rating> {
    let date_range = Period::current_period_date_range()?;

    let query = sqlx::query_as!(
        DbRating,
        "SELECT r.id, r.group_id, r.restaurant_id, r.user_id, r.score, r.username, r.created_at, r.updated_at, u.color
         FROM ratings r
         JOIN users u on u.id = r.user_id
         WHERE user_id = ? AND group_id = ? AND restaurant_id = ? AND created_at >= ? AND created_at <= ?",
        user_id,
        group_id,
        restaurant_id,
        date_range.0,
        date_range.1
    );
    let rating = query.fetch_optional(conn).await?;

    match rating {
        Some(rating) => Ok(Rating::from_db(&rating)),
        None => Err(anyhow!("Rating not found")),
    }
}

pub async fn is_restaurant_rated_by_user(
    conn: &mut MySqlConnection,
    restaurant_id: &str,
    user_id: &str,
    group_id: &str,
) -> Result<bool> {
    let query = sqlx::query_as!(
        DbRating,
        "SELECT r.id, r.group_id, r.restaurant_id, r.user_id, r.score, r.username, r.created_at, r.updated_at, u.color
         FROM ratings r
         JOIN users u on u.id = r.user_id
         WHERE restaurant_id = ? AND user_id = ? AND group_id = ?",
        restaurant_id,
        user_id,
        group_id
    );
    let ratings = query.fetch_all(conn).await?;

    Ok(!ratings.is_empty())
}

pub async fn update_rating(
    conn: &mut MySqlConnection,
    rating: &NewRating,
    user_id: &str,
) -> Result<Rating> {
    let mut tx = conn.begin().await?;

    if !check_group_membership_exists(&mut tx, user_id, &rating.group_id).await? {
        tx.rollback().await?;
        return Err(anyhow!("User does not belong to group"));
    }

    let date_range = Period::current_period_date_range()?;
    let updated_at = Utc::now().naive_utc();

    let _ = match sqlx::query!(
        "UPDATE ratings
         SET score = ?, username = ?, updated_at = ?
         WHERE group_id = ? AND user_id = ? AND restaurant_id = ? AND created_at >= ? AND created_at <= ?",
        rating.score,
        rating.username,
        updated_at,
        rating.group_id,
        user_id,
        rating.restaurant_id,
        date_range.0,
        date_range.1
    )
    .execute(&mut *tx)
    .await
    {
        Ok(query_result) => query_result,
        Err(err) => {
            tx.rollback().await?;
            return Err(anyhow!("Could not update rating: {err}"));
        }
    };

    let updated_rating =
        match get_rating_by_restaurant(&mut tx, user_id, &rating.group_id, &rating.restaurant_id)
            .await
        {
            Ok(updated_rating) => updated_rating,
            Err(err) => {
                tx.rollback().await?;
                return Err(anyhow!("Could not get rating: {err}"));
            }
        };

    tx.commit().await?;

    Ok(updated_rating)
}

pub async fn delete_rating(
    conn: &mut MySqlConnection,
    rating_id: i32,
    user_id: &str,
    group_id: &str,
) -> Result<MySqlQueryResult> {
    let result = sqlx::query!(
        "DELETE FROM ratings WHERE id = ? AND group_id = ? AND user_id = ?",
        rating_id,
        group_id,
        user_id
    )
    .execute(conn)
    .await?;

    Ok(result)
}

// NOTE: Ips

pub async fn create_ip_blacklist(
    conn: &mut MySqlConnection,
    ips: &[Ip],
) -> Result<Vec<MySqlQueryResult>> {
    let new_ips: Vec<NewIp> = ips
        .iter()
        .map(|ip| NewIp {
            ip_address: ip.ip_address.as_str(),
        })
        .collect();

    let mut tx = Acquire::begin(conn).await?;

    let mut results = Vec::<MySqlQueryResult>::new();
    for ip in new_ips {
        let result = sqlx::query!(
            "INSERT INTO ip_blacklist (ip_address) VALUES (?)",
            ip.ip_address
        )
        .execute(&mut *tx)
        .await;
        if let Err(e) = result {
            tx.rollback().await?;
            return Err(e.into());
        }
        results.push(result?);
    }

    tx.commit().await?;

    Ok(results)
}

pub async fn get_ip_blacklist(conn: impl MySqlExecutor<'_>) -> Result<Vec<Ip>> {
    let db_ips = sqlx::query_as!(Ip, "SELECT ip_address FROM ip_blacklist")
        .fetch_all(conn)
        .await?;

    Ok(db_ips)
}

pub async fn delete_ip(conn: &mut MySqlConnection, ip: &str) -> Result<MySqlQueryResult> {
    let result = sqlx::query!("DELETE FROM ip_blacklist WHERE ip_address = ?", ip)
        .execute(conn)
        .await?;

    Ok(result)
}

#[cfg(test)]
mod tests {
    use chrono::Datelike;

    use super::*;

    const USER_ID_1: &str = "test_id";
    const USER_USERNAME_1: &str = "test_username";
    const USER_PASSWORD_1: &str = "test_password";
    const USER_COLOR_1: &str = "#9a79cf";
    const USER_ID_2: &str = "test_id2";
    const USER_USERNAME_2: &str = "test_username2";
    const USER_ID_3: &str = "test_id3";

    const GROUP_ID_1: &str = "test_group_id1";
    const GROUP_NAME_1: &str = "test_group1";
    const GROUP_DESCRIPTION_1: &str = "this is test group 1 (users: test_id, test_id2)";
    const GROUP_ID_2: &str = "test_group_id2";
    const GROUP_NAME_2: &str = "test_group2";
    const GROUP_DESCRIPTION_2: &str = "this is test group 2 (users: test_id, test_id3)";

    const RESTAURANT_ID: &str = "test_restaurant";

    #[sqlx::test]
    async fn test_create_user(pool: MySqlPool) -> Result<()> {
        let new_user = NewUser {
            id: USER_ID_1.to_owned(),
            username: USER_USERNAME_1.to_owned(),
            password: USER_PASSWORD_1.to_owned(),
            color: USER_COLOR_1.to_owned(),
        };

        let mut conn = get_connection(&pool)
            .await
            .ok_or(anyhow!("Failed to get connection."))?;
        let user_result = create_user(&mut conn, &new_user).await;
        assert!(user_result.is_ok());

        let user = user_result?;
        assert_eq!(user.id, new_user.id);
        assert_eq!(user.username, new_user.username);
        assert_eq!(user.password, new_user.password);
        assert_eq!(user.color, new_user.color);

        Ok(())
    }

    #[sqlx::test(fixtures(path = "./../fixtures", scripts("users")))]
    async fn test_get_users(pool: MySqlPool) -> Result<()> {
        let users_result = get_users(&pool).await;
        assert!(users_result.is_ok());

        let users = users_result?;
        assert!(!users.is_empty());

        let user = users.first().unwrap();
        assert_eq!(user.id, USER_ID_1);
        assert_eq!(user.username, USER_USERNAME_1);
        assert_eq!(user.password, USER_PASSWORD_1);
        assert_eq!(user.color, USER_COLOR_1);
        assert_eq!(
            user.group_memberships
                .iter()
                .map(|gm| gm.group_id.clone())
                .collect::<Vec<String>>(),
            vec![GROUP_ID_1, GROUP_ID_2]
        );
        assert!(user
            .group_memberships
            .iter()
            .all(|gm| gm.role == Role::Admin));

        Ok(())
    }

    #[sqlx::test(fixtures(path = "./../fixtures", scripts("users")))]
    async fn test_get_user_by_credentials(pool: MySqlPool) -> Result<()> {
        let mut conn = get_connection(&pool)
            .await
            .ok_or(anyhow!("Failed to get connection."))?;

        let user_result = get_user_by_credentials(&mut conn, USER_USERNAME_1).await;
        assert!(user_result.is_ok());

        let user_option = user_result?;
        assert!(user_option.is_some());

        let user = user_option.unwrap();
        assert_eq!(user.id, USER_ID_1);
        assert_eq!(user.username, USER_USERNAME_1);
        assert_eq!(user.password, USER_PASSWORD_1);
        assert_eq!(user.color, USER_COLOR_1);
        assert_eq!(
            user.group_memberships
                .iter()
                .map(|gm| gm.group_id.clone())
                .collect::<Vec<String>>(),
            vec![GROUP_ID_1, GROUP_ID_2]
        );
        assert!(user
            .group_memberships
            .iter()
            .all(|gm| gm.role == Role::Admin));

        Ok(())
    }

    #[sqlx::test(fixtures(path = "./../fixtures", scripts("users")))]
    async fn test_update_user(pool: MySqlPool) -> Result<()> {
        let mut conn = get_connection(&pool)
            .await
            .ok_or(anyhow!("Failed to get connection."))?;

        const NEW_USERNAME: &str = "new_username1";
        const NEW_COLOR: &str = "#000000";
        let user = NewUser {
            id: USER_ID_1.to_owned(),
            username: NEW_USERNAME.to_owned(),
            password: USER_PASSWORD_1.to_owned(),
            color: NEW_COLOR.to_owned(),
        };

        let update_user_result = update_user(&mut conn, &user.id, &user).await;
        assert!(update_user_result.is_ok());

        let updated_user = update_user_result?;
        assert_eq!(updated_user.id, USER_ID_1);
        assert_eq!(updated_user.username, NEW_USERNAME);
        assert_eq!(updated_user.password, USER_PASSWORD_1);
        assert_eq!(updated_user.color, NEW_COLOR);
        assert_eq!(
            updated_user
                .group_memberships
                .iter()
                .map(|gm| gm.group_id.clone())
                .collect::<Vec<String>>(),
            vec![GROUP_ID_1, GROUP_ID_2]
        );
        assert!(updated_user
            .group_memberships
            .iter()
            .all(|gm| gm.role == Role::Admin));

        Ok(())
    }

    #[sqlx::test(fixtures(path = "./../fixtures", scripts("users")))]
    async fn test_delete_user(pool: MySqlPool) -> Result<()> {
        let mut conn = get_connection(&pool)
            .await
            .ok_or(anyhow!("Failed to get connection."))?;

        let delete_user_result = delete_user(&mut conn, USER_ID_1).await;
        assert!(delete_user_result.is_ok());

        let query_result = delete_user_result?;
        assert_eq!(query_result.rows_affected(), 1);

        Ok(())
    }

    #[sqlx::test(fixtures(path = "./../fixtures", scripts("users")))]
    async fn test_create_push_subscription(pool: MySqlPool) -> Result<()> {
        let subscription_info =
            SubscriptionInfo::new("https://test_endpoint.com", "p256dh", "auth");

        let mut conn = get_connection(&pool)
            .await
            .ok_or(anyhow!("Failed to get connection."))?;
        let push_subscription_result =
            create_push_subscription(&mut conn, USER_ID_1, &subscription_info).await;
        assert!(push_subscription_result.is_ok());

        let initial_push_subscription = push_subscription_result?;
        assert_eq!(initial_push_subscription.user_id, USER_ID_1);
        assert_eq!(
            initial_push_subscription.endpoint,
            subscription_info.endpoint
        );
        assert_eq!(
            initial_push_subscription.p256dh,
            subscription_info.keys.p256dh
        );
        assert_eq!(initial_push_subscription.auth, subscription_info.keys.auth);

        let updated_subscription_info =
            SubscriptionInfo::new("https://test_endpoint.com", "p256dh1", "auth1");

        let push_subscription_result =
            create_push_subscription(&mut conn, USER_ID_1, &updated_subscription_info).await;
        assert!(push_subscription_result.is_ok());

        let updated_push_subscription = push_subscription_result?;
        assert_eq!(
            updated_push_subscription.endpoint,
            initial_push_subscription.endpoint
        );
        assert_eq!(
            updated_push_subscription.endpoint,
            updated_subscription_info.endpoint
        );
        assert_eq!(updated_push_subscription.user_id, USER_ID_1);
        assert_eq!(
            updated_push_subscription.p256dh,
            updated_subscription_info.keys.p256dh
        );
        assert_eq!(
            updated_push_subscription.auth,
            updated_subscription_info.keys.auth
        );

        Ok(())
    }

    #[sqlx::test(fixtures(path = "./../fixtures", scripts("users", "restaurants")))]
    async fn test_create_rating_notification(pool: MySqlPool) -> Result<()> {
        let mut conn = get_connection(&pool)
            .await
            .ok_or(anyhow!("Failed to get connection."))?;
        let create_rating_notification_result =
            create_rating_notification(&mut conn, RESTAURANT_ID, GROUP_ID_1).await;
        assert!(create_rating_notification_result.is_ok());

        let rating_notification = create_rating_notification_result?;
        assert_eq!(rating_notification.restaurant_id, RESTAURANT_ID);
        assert_eq!(rating_notification.group_id, GROUP_ID_1);

        let current_period = Period::current_period()?;
        assert_eq!(
            Period::from_date(rating_notification.notified_at.into()),
            current_period
        );

        Ok(())
    }

    #[sqlx::test(fixtures(path = "./../fixtures", scripts("users")))]
    async fn test_create_group(pool: MySqlPool) -> Result<()> {
        let new_group = NewGroup {
            name: "test_group3".to_owned(),
            description: Some("group 3".to_owned()),
            creator_id: USER_ID_1.to_owned(),
        };

        let mut conn = get_connection(&pool)
            .await
            .ok_or(anyhow!("Failed to get connection."))?;
        let create_group_result = create_group(&mut conn, &new_group).await;
        assert!(create_group_result.is_ok());

        let group_membership = create_group_result?;
        assert_eq!(group_membership.group.name, new_group.name);
        assert_eq!(group_membership.group.description, new_group.description);

        Ok(())
    }

    #[sqlx::test(fixtures(path = "./../fixtures", scripts("users")))]
    async fn test_get_group(pool: MySqlPool) -> Result<()> {
        let mut conn = get_connection(&pool)
            .await
            .ok_or(anyhow!("Failed to get connection."))?;

        let group_result = get_group(&mut conn, GROUP_ID_1).await;
        assert!(group_result.is_ok());

        let group = group_result?;
        assert_eq!(group.id, GROUP_ID_1);
        assert_eq!(group.name, GROUP_NAME_1);
        assert_eq!(group.description, Some(GROUP_DESCRIPTION_1.to_owned()));

        Ok(())
    }

    #[sqlx::test(fixtures(path = "./../fixtures", scripts("users")))]
    async fn test_get_group_by_membership(pool: MySqlPool) -> Result<()> {
        let mut conn = get_connection(&pool)
            .await
            .ok_or(anyhow!("Failed to get connection."))?;

        let group_result = get_group_by_membership(&mut conn, &1).await;
        assert!(group_result.is_ok());

        let group = group_result?;
        assert_eq!(group.id, GROUP_ID_1);
        assert_eq!(group.name, GROUP_NAME_1);
        assert_eq!(group.description, Some(GROUP_DESCRIPTION_1.to_owned()));

        Ok(())
    }

    #[sqlx::test(fixtures(path = "./../fixtures", scripts("users")))]
    async fn test_delete_group(pool: MySqlPool) -> Result<()> {
        let mut conn = get_connection(&pool)
            .await
            .ok_or(anyhow!("Failed to get connection."))?;

        let group_result = delete_group(&mut conn, GROUP_ID_2).await;
        assert!(group_result.is_ok());

        let query_result = group_result?;
        assert_eq!(query_result.rows_affected(), 1);

        Ok(())
    }

    #[sqlx::test(fixtures(path = "./../fixtures", scripts("users")))]
    async fn test_create_group_membership(pool: MySqlPool) -> Result<()> {
        let mut conn = get_connection(&pool)
            .await
            .ok_or(anyhow!("Failed to get connection."))?;

        let new_group_membership = NewGroupMembership {
            group_id: GROUP_ID_2.to_owned(),
            user_id: USER_ID_3.to_owned(),
            role: Role::Member,
        };
        let create_group_membership_result =
            create_group_membership(&mut conn, &new_group_membership).await;
        assert!(create_group_membership_result.is_ok());

        let group_membership = create_group_membership_result?;
        assert_eq!(group_membership.group_id, GROUP_ID_2);
        assert_eq!(group_membership.group.id, GROUP_ID_2);
        assert_eq!(group_membership.group.name, GROUP_NAME_2);
        assert_eq!(
            group_membership.group.description,
            Some(GROUP_DESCRIPTION_2.to_owned())
        );
        assert_eq!(group_membership.user_id, USER_ID_3);
        assert_eq!(group_membership.role, Role::Member);

        let create_group_membership_result =
            create_group_membership(&mut conn, &new_group_membership).await;
        assert!(create_group_membership_result.is_err());

        Ok(())
    }

    #[sqlx::test(fixtures(path = "./../fixtures", scripts("users")))]
    async fn test_get_group_membership(pool: MySqlPool) -> Result<()> {
        let mut conn = get_connection(&pool)
            .await
            .ok_or(anyhow!("Failed to get connection."))?;

        let get_group_membership_result = get_group_membership(&mut conn, &1).await;
        assert!(get_group_membership_result.is_ok());

        let group_membership = get_group_membership_result?;
        assert_eq!(group_membership.group_id, GROUP_ID_1);
        assert_eq!(group_membership.group.id, GROUP_ID_1);
        assert_eq!(group_membership.group.name, GROUP_NAME_1);
        assert_eq!(
            group_membership.group.description,
            Some(GROUP_DESCRIPTION_1.to_owned())
        );
        assert_eq!(group_membership.user_id, USER_ID_1);
        assert_eq!(group_membership.role, Role::Admin);

        Ok(())
    }

    #[sqlx::test(fixtures(path = "./../fixtures", scripts("users")))]
    async fn test_get_group_membership_by_user(pool: MySqlPool) -> Result<()> {
        let mut conn = get_connection(&pool)
            .await
            .ok_or(anyhow!("Failed to get connection."))?;

        let get_group_membership_result = get_group_memberships_by_user(&mut conn, USER_ID_1).await;
        assert!(get_group_membership_result.is_ok());

        let group_memberships = get_group_membership_result?;
        let mut group_memberships_iter = group_memberships.iter();

        let first = group_memberships_iter.next();
        assert!(first.is_some());
        let first = first.unwrap();
        assert_eq!(first.group_id, GROUP_ID_1);
        assert_eq!(first.group.id, GROUP_ID_1);
        assert_eq!(first.group.name, GROUP_NAME_1);
        assert_eq!(
            first.group.description,
            Some(GROUP_DESCRIPTION_1.to_owned())
        );
        assert_eq!(first.user_id, USER_ID_1);
        assert_eq!(first.role, Role::Admin);

        let second = group_memberships_iter.next();
        assert!(second.is_some());
        let second = second.unwrap();
        assert_eq!(second.group_id, GROUP_ID_2);
        assert_eq!(second.group.id, GROUP_ID_2);
        assert_eq!(second.group.name, GROUP_NAME_2);
        assert_eq!(
            second.group.description,
            Some(GROUP_DESCRIPTION_2.to_owned())
        );
        assert_eq!(second.user_id, USER_ID_1);
        assert_eq!(second.role, Role::Admin);

        let third = group_memberships_iter.next();
        assert!(third.is_none());

        Ok(())
    }

    #[sqlx::test(fixtures(path = "./../fixtures", scripts("users")))]
    async fn test_delete_group_membership(pool: MySqlPool) -> Result<()> {
        let mut conn = get_connection(&pool)
            .await
            .ok_or(anyhow!("Failed to get connection."))?;

        let group_result = delete_group_membership(&mut conn, &1).await;
        assert!(group_result.is_ok());

        let query_result = group_result?;
        assert_eq!(query_result.rows_affected(), 1);

        Ok(())
    }

    #[sqlx::test]
    async fn test_create_restaurant(pool: MySqlPool) -> Result<()> {
        let new_restaurant = Restaurant {
            id: RESTAURANT_ID.to_owned(),
            cuisine: "test_cuisine".to_owned(),
            ..Default::default()
        };

        let mut conn = get_connection(&pool)
            .await
            .ok_or(anyhow!("Failed to get connection."))?;
        let create_restaurant_result = create_restaurant(&mut conn, &new_restaurant).await;
        assert!(create_restaurant_result.is_ok());

        let restaurant = create_restaurant_result?;
        assert_eq!(restaurant.id, new_restaurant.id);
        assert_eq!(restaurant.cuisine, new_restaurant.cuisine);

        Ok(())
    }

    #[sqlx::test(fixtures(path = "./../fixtures", scripts("restaurants")))]
    async fn test_get_restaurants(pool: MySqlPool) -> Result<()> {
        let mut conn = get_connection(&pool)
            .await
            .ok_or(anyhow!("Failed to get connection."))?;

        let restaurants_result = get_restaurants(&mut conn).await;
        assert!(restaurants_result.is_ok());

        let restaurants = restaurants_result?;
        assert!(!restaurants.is_empty());

        Ok(())
    }

    #[sqlx::test(fixtures(path = "./../fixtures", scripts("restaurants")))]
    async fn test_get_restaurant(pool: MySqlPool) -> Result<()> {
        let mut conn = get_connection(&pool)
            .await
            .ok_or(anyhow!("Failed to get connection."))?;

        let restaurant_result = get_restaurant(&mut conn, RESTAURANT_ID).await;
        assert!(restaurant_result.is_ok());

        let restaurant = restaurant_result.unwrap();
        assert_eq!(restaurant.id, RESTAURANT_ID);
        assert_eq!(restaurant.cuisine, "test_cuisine");

        Ok(())
    }

    #[sqlx::test(fixtures(path = "./../fixtures", scripts("restaurants")))]
    async fn test_update_restuarant(pool: MySqlPool) -> Result<()> {
        let mut conn = get_connection(&pool)
            .await
            .ok_or(anyhow!("Failed to get connection."))?;

        let restaurant = Restaurant {
            id: "new_restaurant".to_owned(),
            cuisine: "new_cuisine".to_owned(),
            ..Default::default()
        };

        let update_restaurant_result =
            update_restaurant(&mut conn, RESTAURANT_ID, &restaurant).await;
        assert!(update_restaurant_result.is_ok());

        let query_result = update_restaurant_result?;
        assert_eq!(query_result.rows_affected(), 1);

        Ok(())
    }

    #[sqlx::test(fixtures(path = "./../fixtures", scripts("restaurants")))]
    async fn test_delete_restaurant(pool: MySqlPool) -> Result<()> {
        let mut conn = get_connection(&pool)
            .await
            .ok_or(anyhow!("Failed to get connection."))?;

        let delete_restaurant_result = delete_restaurant(&mut conn, RESTAURANT_ID).await;
        assert!(delete_restaurant_result.is_ok());

        let query_result = delete_restaurant_result?;
        assert_eq!(query_result.rows_affected(), 1);

        Ok(())
    }

    #[sqlx::test(fixtures(
        path = "./../fixtures",
        scripts("users", "restaurants", "ratings_incomplete")
    ))]
    async fn test_is_restaurant_rating_complete(pool: MySqlPool) -> Result<()> {
        let mut is_restaurant_rating_complete_result =
            is_restaurant_rating_complete(&pool, None, RESTAURANT_ID, GROUP_ID_1).await;
        assert!(is_restaurant_rating_complete_result.is_ok());

        let mut is_complete = is_restaurant_rating_complete_result?;
        assert!(!is_complete);

        let new_rating = NewRating {
            group_id: GROUP_ID_1.to_owned(),
            restaurant_id: RESTAURANT_ID.to_owned(),
            user_id: USER_ID_2.to_owned(),
            username: USER_USERNAME_2.to_owned(),
            score: 8.0,
        };

        let mut conn = get_connection(&pool)
            .await
            .ok_or(anyhow!("Failed to get connection."))?;
        let create_rating_result = create_rating(&mut conn, &new_rating).await;
        assert!(create_rating_result.is_ok());

        is_restaurant_rating_complete_result =
            is_restaurant_rating_complete(&pool, None, RESTAURANT_ID, GROUP_ID_1).await;
        assert!(is_restaurant_rating_complete_result.is_ok());

        is_complete = is_restaurant_rating_complete_result?;
        assert!(is_complete);

        Ok(())
    }

    #[sqlx::test(fixtures(
        path = "./../fixtures",
        scripts("users", "restaurants", "ratings_complete")
    ))]
    async fn test_get_avg_rating(pool: MySqlPool) -> Result<()> {
        let avg_rating_result = get_avg_rating(&pool, RESTAURANT_ID, GROUP_ID_1).await;
        assert!(avg_rating_result.is_ok());

        let avg_rating_option = avg_rating_result?;
        assert!(avg_rating_option.is_some());

        let avg_rating = avg_rating_option.unwrap();
        assert_eq!(avg_rating, 9.0);

        Ok(())
    }

    #[sqlx::test(fixtures(
        path = "./../fixtures",
        scripts("users", "restaurants", "ratings_complete")
    ))]
    async fn test_get_restaurants_with_avg_rating(pool: MySqlPool) -> Result<()> {
        let mut conn = get_connection(&pool)
            .await
            .ok_or(anyhow!("Failed to get connection."))?;

        let restaurants_with_avg_rating_result =
            get_restaurants_with_avg_rating(&mut conn, GROUP_ID_1).await;
        assert!(restaurants_with_avg_rating_result.is_ok());

        let restaurants_with_avg_rating = restaurants_with_avg_rating_result?;
        assert!(!restaurants_with_avg_rating.is_empty());

        let test_restaurant_option = restaurants_with_avg_rating.first();
        assert!(test_restaurant_option.is_some());

        let (test_restaurant, avg_rating) = test_restaurant_option.unwrap();
        assert_eq!(test_restaurant.id, RESTAURANT_ID);
        assert_eq!(*avg_rating, 9.0);

        Ok(())
    }

    #[sqlx::test(fixtures(
        path = "./../fixtures",
        scripts("users", "restaurants", "ratings_incomplete")
    ))]
    async fn test_create_rating(pool: MySqlPool) -> Result<()> {
        let mut conn = get_connection(&pool)
            .await
            .ok_or(anyhow!("Failed to get connection."))?;

        let new_rating = NewRating {
            group_id: GROUP_ID_1.to_owned(),
            restaurant_id: RESTAURANT_ID.to_owned(),
            user_id: USER_ID_2.to_owned(),
            username: USER_USERNAME_2.to_owned(),
            score: 8.0,
        };

        let (current_year, current_period, _) = Period::current_period_info()?;

        let create_rating_result = create_rating(&mut conn, &new_rating).await;
        assert!(create_rating_result.is_ok());

        let rating = create_rating_result?;
        assert_eq!(rating.group_id, new_rating.group_id);
        assert_eq!(rating.restaurant_id, new_rating.restaurant_id);
        assert_eq!(rating.user_id, new_rating.user_id);
        assert_eq!(rating.username, new_rating.username);
        assert_eq!(rating.score, new_rating.score);
        assert_eq!(rating.created_at.year(), current_year);
        assert_eq!(rating.period, current_period);

        Ok(())
    }

    #[sqlx::test(fixtures(
        path = "./../fixtures",
        scripts("users", "restaurants", "ratings_complete")
    ))]
    async fn test_get_ratings_by_user_and_group(pool: MySqlPool) -> Result<()> {
        let mut conn = get_connection(&pool)
            .await
            .ok_or(anyhow!("Failed to get connection."))?;

        let ratings_by_user_result =
            get_ratings_by_user_and_group(&mut conn, USER_ID_1, GROUP_ID_1).await;
        assert!(ratings_by_user_result.is_ok());

        let ratings_by_user = ratings_by_user_result?;
        let current_period_ratings = ratings_by_user.current_period_ratings;
        let historical_ratings = ratings_by_user.historical_ratings;

        let (current_year, current_period, _) = Period::current_period_info()?;
        assert_eq!(ratings_by_user.current_year, current_year);
        assert_eq!(ratings_by_user.current_period, current_period);

        assert!(!current_period_ratings.is_empty());
        assert_eq!(current_period_ratings.len(), 1);

        assert!(!historical_ratings.is_empty());
        assert_eq!(historical_ratings.len(), 1);

        let rating = current_period_ratings.first().unwrap();
        assert_eq!(rating.user_id, USER_ID_1);
        assert_eq!(rating.group_id, GROUP_ID_1);
        assert_eq!(rating.color, Some(USER_COLOR_1.to_owned()));

        Ok(())
    }

    #[sqlx::test(fixtures(
        path = "./../fixtures",
        scripts("users", "restaurants", "ratings_complete")
    ))]
    async fn test_get_ratings_by_restaurant(pool: MySqlPool) -> Result<()> {
        let mut conn = get_connection(&pool)
            .await
            .ok_or(anyhow!("Failed to get connection."))?;

        let ratings_by_restaurant_result =
            get_ratings_by_restaurant(&mut conn, RESTAURANT_ID, GROUP_ID_1).await;
        assert!(ratings_by_restaurant_result.is_ok());

        let ratings_by_restaurant = ratings_by_restaurant_result?;
        let current_period_ratings = ratings_by_restaurant.current_period_ratings;
        let historical_ratings = ratings_by_restaurant.historical_ratings;

        let (current_year, current_period, _) = Period::current_period_info()?;
        assert_eq!(ratings_by_restaurant.current_year, current_year);
        assert_eq!(ratings_by_restaurant.current_period, current_period);

        assert!(!current_period_ratings.is_empty());
        assert_eq!(current_period_ratings.len(), 2);

        assert!(!historical_ratings.is_empty());
        assert_eq!(historical_ratings.len(), 1);

        for rating in current_period_ratings {
            assert_eq!(rating.restaurant_id, RESTAURANT_ID);
            assert_eq!(rating.group_id, GROUP_ID_1);
            assert!(rating.color.is_some());
        }

        Ok(())
    }

    #[sqlx::test(fixtures(
        path = "./../fixtures",
        scripts("users", "restaurants", "ratings_complete")
    ))]
    async fn test_get_ratings_by_restaurant_per_period(pool: MySqlPool) -> Result<()> {
        let mut conn = get_connection(&pool)
            .await
            .ok_or(anyhow!("Failed to get connection."))?;

        let (current_year, current_period, _) = Period::current_period_info()?;

        let ratings_by_restaurant_result = get_ratings_by_restaurant_per_period(
            &mut conn,
            GROUP_ID_1,
            RESTAURANT_ID,
            current_year,
            &current_period,
        )
        .await;
        assert!(ratings_by_restaurant_result.is_ok());

        let ratings_by_restaurant = ratings_by_restaurant_result?;
        assert!(!ratings_by_restaurant.is_empty());
        assert_eq!(ratings_by_restaurant.len(), 2);

        for rating in ratings_by_restaurant {
            assert_eq!(rating.group_id, GROUP_ID_1);
            assert_eq!(rating.restaurant_id, RESTAURANT_ID);
            assert_eq!(rating.created_at.year(), current_year);
            assert_eq!(rating.period, current_period);
            assert!(rating.color.is_some());
        }

        Ok(())
    }

    #[sqlx::test(fixtures(
        path = "./../fixtures",
        scripts("users", "restaurants", "ratings_incomplete")
    ))]
    async fn test_get_rating(pool: MySqlPool) -> Result<()> {
        let mut conn = get_connection(&pool)
            .await
            .ok_or(anyhow!("Failed to get connection."))?;

        let get_rating_result =
            get_rating_by_restaurant(&mut conn, USER_ID_1, GROUP_ID_1, RESTAURANT_ID).await;
        assert!(get_rating_result.is_ok());

        let rating = get_rating_result?;
        assert_eq!(rating.group_id, GROUP_ID_1);
        assert_eq!(rating.user_id, USER_ID_1);
        assert_eq!(rating.restaurant_id, RESTAURANT_ID);
        assert_eq!(rating.color, Some(USER_COLOR_1.to_owned()));

        Ok(())
    }

    #[sqlx::test(fixtures(
        path = "./../fixtures",
        scripts("users", "restaurants", "ratings_incomplete")
    ))]
    async fn test_is_restaurant_rated_by_user(pool: MySqlPool) -> Result<()> {
        let mut conn = get_connection(&pool)
            .await
            .ok_or(anyhow!("Failed to get connection."))?;

        let mut is_restaurant_rated_by_user_result =
            is_restaurant_rated_by_user(&mut conn, RESTAURANT_ID, USER_ID_1, GROUP_ID_1).await;
        assert!(is_restaurant_rated_by_user_result.is_ok());

        let is_restaurant_rated_by_user1 = is_restaurant_rated_by_user_result?;
        assert!(is_restaurant_rated_by_user1);

        is_restaurant_rated_by_user_result =
            is_restaurant_rated_by_user(&mut conn, RESTAURANT_ID, USER_ID_2, GROUP_ID_1).await;
        assert!(is_restaurant_rated_by_user_result.is_ok());

        let is_restaurant_rated_by_user2 = is_restaurant_rated_by_user_result?;
        assert!(!is_restaurant_rated_by_user2);

        Ok(())
    }

    #[sqlx::test(fixtures(
        path = "./../fixtures",
        scripts("users", "restaurants", "ratings_complete")
    ))]
    async fn test_update_rating(pool: MySqlPool) -> Result<()> {
        let mut conn = get_connection(&pool)
            .await
            .ok_or(anyhow!("Failed to get connection."))?;

        let new_rating = NewRating {
            group_id: GROUP_ID_1.to_owned(),
            restaurant_id: RESTAURANT_ID.to_owned(),
            user_id: USER_ID_2.to_owned(),
            username: USER_USERNAME_2.to_owned(),
            score: 9.0,
        };

        let (current_year, current_period, _) = Period::current_period_info()?;

        let update_rating_result = update_rating(&mut conn, &new_rating, USER_ID_2).await;
        assert!(update_rating_result.is_ok());

        let updated_rating = update_rating_result?;
        assert_eq!(updated_rating.group_id, new_rating.group_id);
        assert_eq!(updated_rating.restaurant_id, new_rating.restaurant_id);
        assert_eq!(updated_rating.user_id, new_rating.user_id);
        assert_eq!(updated_rating.username, new_rating.username);
        assert_eq!(updated_rating.score, new_rating.score);
        assert_eq!(updated_rating.updated_at.year(), current_year);
        assert_eq!(updated_rating.period, current_period);

        Ok(())
    }

    #[sqlx::test(fixtures(
        path = "./../fixtures",
        scripts("users", "restaurants", "ratings_complete")
    ))]
    async fn test_delete_rating(pool: MySqlPool) -> Result<()> {
        let mut conn = get_connection(&pool)
            .await
            .ok_or(anyhow!("Failed to get connection."))?;

        let delete_rating_result = delete_rating(&mut conn, 1, USER_ID_1, GROUP_ID_1).await;
        assert!(delete_rating_result.is_ok());

        let query_result = delete_rating_result?;
        assert_eq!(query_result.rows_affected(), 1);

        Ok(())
    }
}
