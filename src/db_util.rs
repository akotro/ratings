#![allow(dead_code)]

use std::{cmp::Ordering, env};

use anyhow::{anyhow, Error, Ok, Result};
use dotenvy::dotenv;
use sqlx::{
    migrate,
    mysql::{MySqlPoolOptions, MySqlQueryResult},
    pool::PoolConnection,
    query, query_as, Acquire, MySql, MySqlConnection, MySqlExecutor, MySqlPool, Row,
};

use crate::{db_models::*, models::*};

// NOTE:(akotro) Database

const LOCAL_DB: &str = "DATABASE_URL";

pub async fn init_database() -> Result<MySqlPool, Error> {
    dotenv().ok();
    let database_url = env::var(LOCAL_DB).expect("DATABASE_URL must be set");
    let pool = MySqlPoolOptions::new().connect(&database_url).await?;

    migrate!().run(&pool).await?;

    Ok(pool)
}

pub async fn get_connection(pool: &MySqlPool) -> Option<PoolConnection<MySql>> {
    pool.acquire().await.ok()
}

// NOTE:(akotro) Users

pub async fn get_users(pool: &MySqlPool) -> Result<Vec<User>> {
    let db_users = sqlx::query_as!(DbUser, "SELECT id, username, password FROM users")
        .fetch_all(pool)
        .await?;

    let tasks: Vec<_> = db_users
        .into_iter()
        .map(|db_user| {
            let pool = pool.clone();
            async move {
                // TODO: Better way to do this?
                let mut conn = get_connection(&pool)
                    .await
                    .ok_or(anyhow!("Failed to get connection."))?;
                let ratings = get_ratings_by_user(&mut conn, &db_user.id)
                    .await
                    .unwrap_or_default();
                Ok(User {
                    id: db_user.id,
                    token: String::new(),
                    username: db_user.username,
                    password: db_user.password,
                    ratings,
                })
            }
        })
        .collect();

    let results: Result<Vec<_>, _> = futures::future::join_all(tasks).await.into_iter().collect();

    results
}

pub async fn create_user(conn: &mut MySqlConnection, new_user: &NewUser) -> Result<User> {
    let existing_user = get_user_by_credentials(conn, &new_user.username).await;
    if existing_user.is_ok_and(|u| u.is_some()) {
        return Err(anyhow!(
            "User already exists with username: {}",
            new_user.username
        ));
    }

    let query = query_as!(
        User,
        "INSERT INTO users (id, username, password) VALUES (?, ?, ?)",
        new_user.id,
        new_user.username,
        new_user.password
    );
    let result = query.execute(conn).await?;

    if result.rows_affected() == 1 {
        let last_insert_id = result.last_insert_id();

        Ok(User {
            id: last_insert_id.to_string(),
            username: new_user.username.clone(),
            password: new_user.password.clone(),
            token: String::new(),
            ratings: Vec::<Rating>::new(),
        })
    } else {
        Err(anyhow::anyhow!("Failed to create user."))
    }
}

pub async fn get_user_by_credentials(
    conn: &mut MySqlConnection,
    username: &str,
) -> Result<Option<User>> {
    let mut tx = Acquire::begin(conn).await?;

    let query = query_as!(
        DbUser,
        "SELECT id, username, password FROM users WHERE username = ?",
        username
    );
    let db_user = query.fetch_optional(&mut *tx).await?;

    match db_user {
        Some(db_user) => {
            let ratings = get_ratings_by_user(&mut tx, &db_user.id).await?;
            Ok(Some(User {
                id: db_user.id,
                token: String::new(),
                username: db_user.username,
                password: db_user.password,
                ratings,
            }))
        }
        None => Err(anyhow!("User not found")),
    }
}

pub async fn update_user(
    conn: &mut MySqlConnection,
    user_id: &str,
    user: &User,
) -> Result<MySqlQueryResult> {
    let result = query!(
        "UPDATE users
         SET username = ?, password = ?
         WHERE id = ?;",
        user.username,
        user.password,
        user_id
    )
    .execute(conn)
    .await?;

    Ok(result)
}

pub async fn delete_user(conn: &mut MySqlConnection, user_id: &str) -> Result<MySqlQueryResult> {
    let result = query!("DELETE FROM users WHERE id = ?", user_id)
        .execute(conn)
        .await?;

    Ok(result)
}

// NOTE:(akotro) Restaurants

pub async fn create_restaurant(
    conn: &mut MySqlConnection,
    restaurant: &Restaurant,
) -> Result<Restaurant> {
    let query = query_as!(
        Restaurant,
        "INSERT INTO restaurants (id, cuisine) VALUES (?, ?)",
        restaurant.id,
        restaurant.cuisine
    );
    let result = query.execute(conn).await?;

    if result.rows_affected() == 1 {
        let last_insert_id = result.last_insert_id();

        Ok(Restaurant {
            id: last_insert_id.to_string(),
            cuisine: restaurant.cuisine.clone(),
            menu: Vec::<MenuItem>::new(),
        })
    } else {
        Err(anyhow::anyhow!("Failed to create restaurant."))
    }
}

pub async fn get_restaurants(conn: &mut MySqlConnection) -> Result<Vec<Restaurant>> {
    let query = query!("SELECT id, cuisine FROM restaurants");
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

pub async fn is_restaurant_rating_complete(
    conn: &mut MySqlConnection,
    restaurant_id: &str,
) -> Result<bool> {
    let result = sqlx::query_scalar!(
        "
        SELECT
            (SELECT COUNT(*) FROM users) =
            (SELECT COUNT(*) FROM ratings WHERE restaurant_id = ?) AS is_complete
        ",
        restaurant_id
    )
    .fetch_one(conn)
    .await?;

    let is_complete = match result {
        Some(is_complete) => is_complete == 1,
        None => return Err(anyhow!("Failed to check if restaurant rating is complete.")),
    };

    Ok(is_complete)
}

pub async fn delete_restaurant(
    conn: &mut MySqlConnection,
    restaurant_id: &str,
) -> Result<MySqlQueryResult> {
    let result = query!("DELETE FROM restaurants WHERE id = ?", restaurant_id)
        .execute(conn)
        .await?;

    Ok(result)
}

pub async fn get_avg_rating(
    conn: &mut MySqlConnection,
    restaurant_id: &str,
) -> Result<Option<f64>> {
    if is_restaurant_rating_complete(conn, restaurant_id).await? {
        let avg_rating: Option<f64> = sqlx::query_scalar!(
            "SELECT AVG(score) FROM ratings WHERE restaurant_id = ?",
            restaurant_id
        )
        .fetch_one(conn)
        .await?;

        Ok(avg_rating)
    } else {
        Ok(None)
    }
}

pub async fn get_restaurants_with_avg_rating(
    conn: &mut MySqlConnection,
) -> Result<Vec<(Restaurant, f64)>> {
    let mut tx = Acquire::begin(conn).await?;

    let db_restaurants_result =
        sqlx::query_as!(DbRestaurant, "SELECT id, cuisine FROM restaurants")
            .fetch_all(&mut *tx)
            .await;

    if let Err(e) = db_restaurants_result {
        tx.rollback().await?;
        return Err(e.into());
    }

    let mut results = Vec::new();

    for db_restaurant in db_restaurants_result? {
        let menu_result = get_menu_items(&mut tx, &db_restaurant.id).await;
        if menu_result.is_err() {
            tx.rollback().await?;
            return Err(menu_result.err().unwrap());
        }
        let menu = menu_result.unwrap_or_default();

        let avg_rating_result = get_avg_rating(&mut tx, &db_restaurant.id).await;
        if avg_rating_result.is_err() {
            tx.rollback().await?;
            return Err(avg_rating_result.err().unwrap());
        }
        let avg_rating = avg_rating_result.unwrap_or(None).unwrap_or(0.0);

        results.push((
            Restaurant {
                id: db_restaurant.id.clone(),
                cuisine: db_restaurant.cuisine.clone(),
                menu,
            },
            avg_rating,
        ));
    }

    results.sort_by(|a, b| {
        if a.1 == 0.0 && b.1 == 0.0 {
            a.0.id.cmp(&b.0.id)
        } else {
            b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal)
        }
    });

    tx.commit().await?;

    Ok(results)
}

// NOTE:(akotro) Menu Items

pub async fn create_menu_item(
    conn: &mut MySqlConnection,
    menu_item: &MenuItem, // Assuming you have a MenuItem struct defined
) -> Result<MenuItem> {
    let mut tx = conn.begin().await?;

    let menu_item_result = query("INSERT INTO menu_items (name, price) VALUES (?, ?)")
        .bind(&menu_item.name)
        .bind(menu_item.price)
        .execute(&mut *tx)
        .await?;

    let last_insert_id = menu_item_result.last_insert_id();

    query("INSERT INTO restaurant_menu_items (restaurant_id, menu_item_id) VALUES (?, ?)")
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
            restaurant_id: restaurant_id.to_string(),
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
    let result = query!("DELETE FROM menu_items WHERE id = ?", menu_item_id)
        .execute(conn)
        .await?;

    Ok(result)
}

// NOTE:(akotro) Ratings

pub async fn create_rating(conn: &mut MySqlConnection, rating: &NewRating) -> Result<Rating> {
    let query = query_as!(
        NewRating,
        "INSERT INTO ratings (restaurant_id, user_id, username, score) VALUES (?, ?, ?, ?)",
        rating.restaurant_id,
        rating.user_id,
        rating.username,
        rating.score
    );
    let result = query.execute(conn).await?;

    if result.rows_affected() == 1 {
        let last_insert_id = result.last_insert_id();

        Ok(Rating {
            id: last_insert_id as i32,
            restaurant_id: rating.restaurant_id.clone(),
            user_id: rating.user_id.clone(),
            username: rating.username.clone(),
            score: rating.score,
        })
    } else {
        Err(anyhow::anyhow!("Failed to create rating."))
    }
}

pub async fn get_ratings_by_user(conn: &mut MySqlConnection, user_id: &str) -> Result<Vec<Rating>> {
    let query = query_as!(
        Rating,
        "SELECT id, restaurant_id, user_id, score, username
         FROM ratings
         WHERE user_id = ?",
        user_id
    );
    let ratings = query.fetch_all(conn).await?;

    Ok(ratings)
}

pub async fn get_ratings_by_restaurant(
    conn: &mut MySqlConnection,
    restaurant_id: &str,
) -> Result<Vec<Rating>> {
    let query = query_as!(
        Rating,
        "SELECT id, restaurant_id, user_id, score, username
         FROM ratings
         WHERE restaurant_id = ?",
        restaurant_id
    );
    let ratings = query.fetch_all(conn).await?;

    Ok(ratings)
}

pub async fn get_rating(
    conn: &mut MySqlConnection,
    user_id: &str,
    restaurant_id: &str,
) -> Result<Rating> {
    let query = query_as!(
        Rating,
        "SELECT id, restaurant_id, user_id, score, username
         FROM ratings
         WHERE user_id = ? AND restaurant_id = ?",
        user_id,
        restaurant_id
    );
    let rating = query.fetch_optional(conn).await?;

    match rating {
        Some(rating) => Ok(rating),
        None => Err(anyhow!("Rating not found")),
    }
}

pub async fn is_restaurant_rated_by_user(
    conn: &mut MySqlConnection,
    restaurant_id: &str,
    user_id: &str,
) -> Result<bool> {
    let query = query_as!(
        Rating,
        "SELECT id, restaurant_id, user_id, score, username
         FROM ratings
         WHERE restaurant_id = ? AND user_id = ?",
        restaurant_id,
        user_id
    );
    let ratings = query.fetch_all(conn).await?;

    Ok(!ratings.is_empty())
}

pub async fn is_restaurant_rated_by_all_users(
    conn: &mut MySqlConnection,
    restaurant_id: &str,
) -> Result<bool> {
    let mut tx = conn.begin().await?;

    let db_users = query_as!(DbUser, "SELECT id, username, password FROM users")
        .fetch_all(&mut *tx)
        .await?;
    let ratings = get_ratings_by_restaurant(&mut tx, restaurant_id).await?;

    Ok(db_users.len() == ratings.len())
}

pub async fn update_rating(
    conn: &mut MySqlConnection,
    rating: &NewRating,
    user_id: &str,
) -> Result<Rating> {
    let result = query!(
        "UPDATE ratings
         SET score = ?, username = ?
         WHERE user_id = ? and restaurant_id = ?",
        rating.score,
        rating.username,
        user_id,
        rating.restaurant_id
    )
    .execute(conn)
    .await?;

    Ok(Rating {
        id: result.last_insert_id() as i32,
        restaurant_id: rating.restaurant_id.clone(),
        user_id: user_id.to_string(),
        username: rating.username.clone(),
        score: rating.score,
    })
}

pub async fn delete_rating(
    conn: &mut MySqlConnection,
    rating_id: i32,
    user_id: &str,
) -> Result<MySqlQueryResult> {
    let result = query!(
        "DELETE FROM ratings WHERE id = ? AND user_id = ?",
        rating_id,
        user_id
    )
    .execute(conn)
    .await?;

    Ok(result)
}

// NOTE:(akotro) Ips

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
        let result = query!(
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
    let db_ips = query_as!(Ip, "SELECT ip_address FROM ip_blacklist")
        .fetch_all(conn)
        .await?;

    Ok(db_ips)
}

pub async fn delete_ip(conn: &mut MySqlConnection, ip: &str) -> Result<MySqlQueryResult> {
    let result = query!("DELETE FROM ip_blacklist WHERE ip_address = ?", ip)
        .execute(conn)
        .await?;

    Ok(result)
}
