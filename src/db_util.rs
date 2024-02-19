#![allow(dead_code)]

use std::{cmp::Ordering, env, result::Result::Ok, str::FromStr};

use anyhow::{anyhow, Error, Result};
use chrono::Utc;
use dotenvy::dotenv;
use sqlx::{
    migrate,
    migrate::Migrator,
    mysql::{MySqlConnectOptions, MySqlQueryResult},
    pool::PoolConnection,
    query, query_as, Acquire, ConnectOptions, MySql, MySqlConnection, MySqlExecutor, MySqlPool,
    Row,
};

use crate::{db_models::*, models::*};

// NOTE:(akotro) Database

const DB_URL: &str = "DATABASE_URL";

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

// NOTE:(akotro) Users

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

    let query = query_as!(
        User,
        "INSERT INTO users (id, username, password) VALUES (?, ?, ?)",
        new_user.id,
        new_user.username,
        new_user.password
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
            token: String::new(),
            ratings: Vec::<Rating>::new(),
        })
    } else {
        Err(anyhow::anyhow!("Failed to create user."))
    }
}
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

    let results: Result<Vec<User>> = futures::future::join_all(tasks).await.into_iter().collect();

    results
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
                Ok(query_result) => query_result,
                Err(err) => {
                    tx.rollback().await?;
                    return Err(anyhow!("User's ratings not found: {err}"));
                }
            };
            tx.commit().await?;
            Ok(Some(User {
                id: db_user.id,
                token: String::new(),
                username: db_user.username,
                password: db_user.password,
                ratings,
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
    let mut tx = conn.begin().await?;

    let query = query_as!(
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

pub async fn get_restaurant(conn: &mut MySqlConnection, restaurant_id: &str) -> Result<Restaurant> {
    let mut tx = Acquire::begin(conn).await?;

    let query = query_as!(
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
    let result = query!(
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
    let result = query!("DELETE FROM restaurants WHERE id = ?", restaurant_id)
        .execute(conn)
        .await?;

    Ok(result)
}

pub async fn is_restaurant_rating_complete(
    conn: &mut MySqlConnection,
    restaurant_id: &str,
) -> Result<bool> {
    let result = sqlx::query_scalar!(
        "SELECT (SELECT COUNT(*) FROM users) = (SELECT COUNT(*) FROM ratings WHERE restaurant_id = ?) AS is_complete",
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
    // TODO: These are too many requests, migrate some of this logic to sql

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
    menu_item: &MenuItem,
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
    let created_at = Utc::now().naive_utc();
    let updated_at = created_at;

    let query = query_as!(
        Rating,
        "INSERT INTO ratings (restaurant_id, user_id, username, score, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?)",
        rating.restaurant_id,
        rating.user_id,
        rating.username,
        rating.score,
        created_at,
        updated_at
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
            created_at,
            updated_at,
        })
    } else {
        Err(anyhow::anyhow!("Failed to create rating."))
    }
}

pub async fn get_ratings_by_user(conn: &mut MySqlConnection, user_id: &str) -> Result<Vec<Rating>> {
    let query = query_as!(
        Rating,
        "SELECT id, restaurant_id, user_id, score, username, created_at, updated_at
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
        "SELECT id, restaurant_id, user_id, score, username, created_at, updated_at
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
        "SELECT id, restaurant_id, user_id, score, username, created_at, updated_at
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
        "SELECT id, restaurant_id, user_id, score, username, created_at, updated_at
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

    let db_users = match query_as!(DbUser, "SELECT id, username, password FROM users")
        .fetch_all(&mut *tx)
        .await
    {
        Ok(db_users) => db_users,
        Err(err) => {
            tx.rollback().await?;
            return Err(anyhow!("Could not get users: {err}"));
        }
    };

    let ratings_result = get_ratings_by_restaurant(&mut tx, restaurant_id).await;
    let ratings = match ratings_result {
        Ok(ratings) => ratings,
        Err(err) => {
            tx.rollback().await?;
            return Err(anyhow!("Could not get ratings: {err}"));
        }
    };

    tx.commit().await?;

    Ok(db_users.len() == ratings.len())
}

pub async fn update_rating(
    conn: &mut MySqlConnection,
    rating: &NewRating,
    user_id: &str,
) -> Result<Rating> {
    let mut tx = conn.begin().await?;

    let updated_at = Utc::now().naive_utc();
    let _ = match query!(
        "UPDATE ratings
         SET score = ?, username = ?, updated_at = ?
         WHERE user_id = ? and restaurant_id = ?",
        rating.score,
        rating.username,
        updated_at,
        user_id,
        rating.restaurant_id
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

    let updated_rating = match get_rating(&mut tx, user_id, &rating.restaurant_id).await {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[sqlx::test]
    async fn test_init_database() {
        let pool_result = init_database().await;
        assert!(pool_result.is_ok());
    }

    #[sqlx::test]
    async fn test_create_user(pool: MySqlPool) -> Result<()> {
        let new_user = NewUser {
            id: "test_id".to_string(),
            username: "test_username".to_string(),
            password: "test_password".to_string(),
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

        Ok(())
    }

    #[sqlx::test(fixtures(path = "./../fixtures", scripts("users")))]
    async fn test_get_users(pool: MySqlPool) -> Result<()> {
        let users_result = get_users(&pool).await;
        assert!(users_result.is_ok());

        let users = users_result?;
        assert!(!users.is_empty());

        let user = users.first().expect("");
        assert_eq!(user.id, "test_id");
        assert_eq!(user.username, "test_username");
        assert_eq!(user.password, "test_password");

        Ok(())
    }

    #[sqlx::test(fixtures(path = "./../fixtures", scripts("users")))]
    async fn test_get_user_by_credentials(pool: MySqlPool) -> Result<()> {
        let mut conn = get_connection(&pool)
            .await
            .ok_or(anyhow!("Failed to get connection."))?;

        let user_result = get_user_by_credentials(&mut conn, "test_username").await;
        assert!(user_result.is_ok());

        let user_option = user_result?;
        assert!(user_option.is_some());

        let user = user_option.expect("");
        assert_eq!(user.id, "test_id");
        assert_eq!(user.username, "test_username");
        assert_eq!(user.password, "test_password");

        Ok(())
    }

    #[sqlx::test(fixtures(path = "./../fixtures", scripts("users")))]
    async fn test_update_user(pool: MySqlPool) -> Result<()> {
        let mut conn = get_connection(&pool)
            .await
            .ok_or(anyhow!("Failed to get connection."))?;

        let user = User {
            id: "test_id".to_string(),
            username: "new_username".to_string(),
            password: "new_password".to_string(),
            ..Default::default()
        };

        let update_user_result = update_user(&mut conn, &user.id, &user).await;
        assert!(update_user_result.is_ok());

        let query_result = update_user_result?;
        assert_eq!(query_result.rows_affected(), 1);

        Ok(())
    }

    #[sqlx::test(fixtures(path = "./../fixtures", scripts("users")))]
    async fn test_delete_user(pool: MySqlPool) -> Result<()> {
        let mut conn = get_connection(&pool)
            .await
            .ok_or(anyhow!("Failed to get connection."))?;

        let delete_user_result = delete_user(&mut conn, "test_id").await;
        assert!(delete_user_result.is_ok());

        let query_result = delete_user_result?;
        assert_eq!(query_result.rows_affected(), 1);

        Ok(())
    }

    #[sqlx::test]
    async fn test_create_restaurant(pool: MySqlPool) -> Result<()> {
        let new_restaurant = Restaurant {
            id: "test_restaurant".to_string(),
            cuisine: "test_cuisine".to_string(),
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

        let restaurant_result = get_restaurant(&mut conn, "test_restaurant").await;
        assert!(restaurant_result.is_ok());

        let restaurant = restaurant_result.expect("");
        assert_eq!(restaurant.id, "test_restaurant");
        assert_eq!(restaurant.cuisine, "test_cuisine");

        Ok(())
    }

    #[sqlx::test(fixtures(path = "./../fixtures", scripts("restaurants")))]
    async fn test_update_restuarant(pool: MySqlPool) -> Result<()> {
        let mut conn = get_connection(&pool)
            .await
            .ok_or(anyhow!("Failed to get connection."))?;

        let restaurant = Restaurant {
            id: "new_restaurant".to_string(),
            cuisine: "new_cuisine".to_string(),
            ..Default::default()
        };

        let update_restaurant_result =
            update_restaurant(&mut conn, "test_restaurant", &restaurant).await;
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

        let delete_restaurant_result = delete_restaurant(&mut conn, "test_restaurant").await;
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
        let mut conn = get_connection(&pool)
            .await
            .ok_or(anyhow!("Failed to get connection."))?;

        let restaurant_id = "test_restaurant";

        let mut is_restaurant_rating_complete_result =
            is_restaurant_rating_complete(&mut conn, restaurant_id).await;
        assert!(is_restaurant_rating_complete_result.is_ok());

        let mut is_complete = is_restaurant_rating_complete_result?;
        assert!(!is_complete);

        let new_rating = NewRating {
            restaurant_id: restaurant_id.to_string(),
            user_id: "test_id2".to_string(),
            username: "test_username2".to_string(),
            score: 8.0,
        };

        let create_rating_result = create_rating(&mut conn, &new_rating).await;
        assert!(create_rating_result.is_ok());

        is_restaurant_rating_complete_result =
            is_restaurant_rating_complete(&mut conn, restaurant_id).await;
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
        let mut conn = get_connection(&pool)
            .await
            .ok_or(anyhow!("Failed to get connection."))?;

        let avg_rating_result = get_avg_rating(&mut conn, "test_restaurant").await;
        assert!(avg_rating_result.is_ok());

        let avg_rating_option = avg_rating_result?;
        assert!(avg_rating_option.is_some());

        let avg_rating = avg_rating_option.expect("");
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

        let restaurants_with_avg_rating_result = get_restaurants_with_avg_rating(&mut conn).await;
        assert!(restaurants_with_avg_rating_result.is_ok());

        let restaurants_with_avg_rating = restaurants_with_avg_rating_result?;
        assert!(!restaurants_with_avg_rating.is_empty());

        let test_restaurant_option = restaurants_with_avg_rating
            .iter()
            .find(|&(r, _)| r.id == "test_restaurant");
        assert!(test_restaurant_option.is_some());

        let (test_restaurant, avg_rating) = test_restaurant_option.expect("");
        assert_eq!(test_restaurant.id, "test_restaurant");
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
            restaurant_id: "test_restaurant".to_string(),
            user_id: "test_id2".to_string(),
            username: "test_username2".to_string(),
            score: 8.0,
        };

        let create_rating_result = create_rating(&mut conn, &new_rating).await;
        assert!(create_rating_result.is_ok());

        let rating = create_rating_result?;
        assert_eq!(rating.restaurant_id, new_rating.restaurant_id);
        assert_eq!(rating.user_id, new_rating.user_id);
        assert_eq!(rating.username, new_rating.username);
        assert_eq!(rating.score, new_rating.score);

        Ok(())
    }

    #[sqlx::test(fixtures(
        path = "./../fixtures",
        scripts("users", "restaurants", "ratings_complete")
    ))]
    async fn test_get_ratings_by_user(pool: MySqlPool) -> Result<()> {
        let mut conn = get_connection(&pool)
            .await
            .ok_or(anyhow!("Failed to get connection."))?;

        let user_id = "test_id";

        let ratings_by_user_result = get_ratings_by_user(&mut conn, user_id).await;
        assert!(ratings_by_user_result.is_ok());

        let ratings_by_user = ratings_by_user_result?;
        assert!(!ratings_by_user.is_empty());
        assert_eq!(ratings_by_user.len(), 1);

        let rating = ratings_by_user.first().expect("");
        assert_eq!(rating.user_id, user_id);

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

        let restaurant_id = "test_restaurant";

        let ratings_by_restaurant_result =
            get_ratings_by_restaurant(&mut conn, restaurant_id).await;
        assert!(ratings_by_restaurant_result.is_ok());

        let ratings_by_restaurant = ratings_by_restaurant_result?;
        assert!(!ratings_by_restaurant.is_empty());
        assert_eq!(ratings_by_restaurant.len(), 2);

        for rating in ratings_by_restaurant {
            assert_eq!(rating.restaurant_id, restaurant_id);
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

        let user_id = "test_id";
        let restaurant_id = "test_restaurant";

        let get_rating_result = get_rating(&mut conn, user_id, restaurant_id).await;
        assert!(get_rating_result.is_ok());

        let rating = get_rating_result?;
        assert_eq!(rating.user_id, user_id);
        assert_eq!(rating.restaurant_id, restaurant_id);

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

        let restaurant_id = "test_restaurant";

        let mut is_restaurant_rated_by_user_result =
            is_restaurant_rated_by_user(&mut conn, restaurant_id, "test_id").await;
        assert!(is_restaurant_rated_by_user_result.is_ok());

        let is_restaurant_rated_by_user1 = is_restaurant_rated_by_user_result?;
        assert!(is_restaurant_rated_by_user1);

        is_restaurant_rated_by_user_result =
            is_restaurant_rated_by_user(&mut conn, restaurant_id, "test_id2").await;
        assert!(is_restaurant_rated_by_user_result.is_ok());

        let is_restaurant_rated_by_user2 = is_restaurant_rated_by_user_result?;
        assert!(!is_restaurant_rated_by_user2);

        Ok(())
    }

    #[sqlx::test(fixtures(
        path = "./../fixtures",
        scripts("users", "restaurants", "ratings_complete")
    ))]
    async fn test_is_restaurant_rated_by_all_users(pool: MySqlPool) -> Result<()> {
        let mut conn = get_connection(&pool)
            .await
            .ok_or(anyhow!("Failed to get connection."))?;

        let restaurant_id = "test_restaurant";

        let is_restaurant_rated_by_all_users_result =
            is_restaurant_rated_by_all_users(&mut conn, restaurant_id).await;
        assert!(is_restaurant_rated_by_all_users_result.is_ok());

        let is_restaurant_rated_by_all_users = is_restaurant_rated_by_all_users_result?;
        assert!(is_restaurant_rated_by_all_users);

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

        let user_id = "test_id2";

        let new_rating = NewRating {
            restaurant_id: "test_restaurant".to_string(),
            user_id: user_id.to_string(),
            username: "test_username2".to_string(),
            score: 9.0,
        };

        let update_rating_result = update_rating(&mut conn, &new_rating, user_id).await;
        assert!(update_rating_result.is_ok());

        let updated_rating = update_rating_result?;
        assert_eq!(updated_rating.restaurant_id, new_rating.restaurant_id);
        assert_eq!(updated_rating.user_id, new_rating.user_id);
        assert_eq!(updated_rating.username, new_rating.username);
        assert_eq!(updated_rating.score, new_rating.score);

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

        let delete_rating_result = delete_rating(&mut conn, 1, "test_id").await;
        assert!(delete_rating_result.is_ok());

        let query_result = delete_rating_result?;
        assert_eq!(query_result.rows_affected(), 1);

        Ok(())
    }
}
