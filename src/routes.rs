use actix_web::{delete, get, post, put, web, HttpRequest, HttpResponse};
use sqlx::MySqlPool;
use uuid::Uuid;

use crate::{auth::*, db_models::*, db_util::*, models::*};

#[post("/register")]
async fn register_user_route(
    pool: web::Data<MySqlPool>,
    req: HttpRequest,
    mut new_user: web::Json<NewUser>,
) -> HttpResponse {
    if let Err(err) = validate_ip(&req) {
        return err;
    }

    new_user.0.id = Uuid::new_v4().to_string();
    let username = new_user.0.username.clone();

    let hashed_password = match generate_password_hash(new_user.0.password.clone()) {
        Ok(password) => password,
        Err(error) => {
            return HttpResponse::InternalServerError()
                .json(ApiResponse::<()>::error(error.to_string()))
        }
    };
    new_user.0.password = hashed_password;

    let mut conn = get_connection(&pool).await.unwrap();
    let result = create_user(&mut conn, &new_user.0).await;

    match result {
        Ok(db_user) => {
            let token = generate_token(&req, db_user.id.clone(), username);
            HttpResponse::Created().json(ApiResponse::success(User {
                id: db_user.id,
                username: db_user.username,
                password: db_user.password,
                token: token.clone(),
                ratings: db_user.ratings,
            }))
        }
        Err(error) => {
            HttpResponse::InternalServerError().json(ApiResponse::<()>::error(error.to_string()))
        }
    }
}

#[post("/login")]
async fn login_user_route(
    pool: web::Data<MySqlPool>,
    req: HttpRequest,
    credentials: web::Json<NewUser>,
) -> HttpResponse {
    if let Err(err) = validate_ip(&req) {
        return err;
    }

    let username = credentials.0.username.clone();
    let password = credentials.0.password.clone();

    let mut conn = get_connection(&pool).await.unwrap();
    let result = get_user_by_credentials(&mut conn, &credentials.0.username.clone()).await;

    match result {
        Ok(users_result) => match users_result {
            Some(mut user) => {
                let is_valid_password = validate_password(&user.password, &password);
                if is_valid_password {
                    let token = generate_token(&req, user.id.clone(), username);
                    user.token = token.clone();
                    HttpResponse::Ok().json(ApiResponse::success(user))
                } else {
                    HttpResponse::Unauthorized()
                        .json(ApiResponse::<()>::error("Invalid credentials".to_string()))
                }
            }
            None => HttpResponse::InternalServerError()
                .json(ApiResponse::<()>::error("User not found".to_string())),
        },
        Err(error) => {
            HttpResponse::InternalServerError().json(ApiResponse::<()>::error(error.to_string()))
        }
    }
}

#[get("/users")]
async fn get_users_route(pool: web::Data<MySqlPool>, req: HttpRequest) -> HttpResponse {
    if let Err(err) = validate_ip(&req) {
        return err;
    }

    if let Err(err) = validate_token(&req) {
        return err;
    }

    let result = get_users(&pool).await;
    match result {
        Ok(users) => HttpResponse::Ok().json(ApiResponse::success(users)),
        Err(error) => {
            HttpResponse::InternalServerError().json(ApiResponse::<()>::error(error.to_string()))
        }
    }
}

#[delete("/users/{id}")]
async fn delete_user_route(
    pool: web::Data<MySqlPool>,
    req: HttpRequest,
    id: web::Path<String>,
) -> HttpResponse {
    if let Err(err) = validate_ip(&req) {
        return err;
    }

    if let Err(err) = validate_token(&req) {
        return err;
    }

    let mut conn = get_connection(&pool).await.unwrap();
    let result = delete_user(&mut conn, &id).await;
    match result {
        Ok(rows) => HttpResponse::Ok().json(ApiResponse::success(rows.last_insert_id())),
        Err(error) => {
            HttpResponse::InternalServerError().json(ApiResponse::<()>::error(error.to_string()))
        }
    }
}

#[post("/restaurants")]
async fn create_restaurant_route(
    pool: web::Data<MySqlPool>,
    req: HttpRequest,
    restaurant: web::Json<Restaurant>,
) -> HttpResponse {
    if let Err(err) = validate_ip(&req) {
        return err;
    }

    if let Err(err) = validate_token(&req) {
        return err;
    }

    let mut conn = get_connection(&pool).await.unwrap();
    let result = create_restaurant(&mut conn, &restaurant.0).await;
    match result {
        Ok(restaurant) => HttpResponse::Ok().json(ApiResponse::success(restaurant)),
        Err(error) => {
            HttpResponse::InternalServerError().json(ApiResponse::<()>::error(error.to_string()))
        }
    }
}

#[get("/restaurants")]
async fn get_restaurants_route(pool: web::Data<MySqlPool>, req: HttpRequest) -> HttpResponse {
    if let Err(err) = validate_ip(&req) {
        return err;
    }

    // if let Err(err) = validate_token(&req) {
    //     return err;
    // }

    let mut conn = get_connection(&pool).await.unwrap();
    let result = get_restaurants(&mut conn).await;
    match result {
        Ok(restaurants) => HttpResponse::Ok().json(ApiResponse::success(restaurants)),
        Err(error) => {
            HttpResponse::InternalServerError().json(ApiResponse::<()>::error(error.to_string()))
        }
    }
}

#[get("/restaurants_with_avg_rating")]
async fn get_restaurants_with_avg_rating_route(
    pool: web::Data<MySqlPool>,
    req: HttpRequest,
) -> HttpResponse {
    if let Err(err) = validate_ip(&req) {
        return err;
    }

    if let Err(err) = validate_token(&req) {
        return err;
    }

    let mut conn = get_connection(&pool).await.unwrap();
    let result = get_restaurants_with_avg_rating(&mut conn).await;
    match result {
        Ok(restaurants_with_avg) => {
            HttpResponse::Ok().json(ApiResponse::success(restaurants_with_avg))
        }
        Err(error) => {
            HttpResponse::InternalServerError().json(ApiResponse::<()>::error(error.to_string()))
        }
    }
}

#[get("/restaurants/{id}/ratings")]
async fn get_restaurant_ratings_route(
    pool: web::Data<MySqlPool>,
    req: HttpRequest,
    id: web::Path<String>,
) -> HttpResponse {
    if let Err(err) = validate_ip(&req) {
        return err;
    }

    if let Err(err) = validate_token(&req) {
        return err;
    }

    let mut conn = get_connection(&pool).await.unwrap();
    let result = get_ratings_by_restaurant(&mut conn, &id).await;
    match result {
        Ok(restaurant_ratings) => HttpResponse::Ok().json(ApiResponse::success(restaurant_ratings)),
        Err(error) => {
            HttpResponse::InternalServerError().json(ApiResponse::<()>::error(error.to_string()))
        }
    }
}

#[get("/restaurants/{id}/ratings/{year}/{period}")]
async fn get_restaurant_ratings_per_period_route(
    pool: web::Data<MySqlPool>,
    req: HttpRequest,
    params: web::Path<(String, i32, Period)>,
) -> HttpResponse {
    if let Err(err) = validate_ip(&req) {
        return err;
    }

    if let Err(err) = validate_token(&req) {
        return err;
    }

    let (restaurant_id, year, period) = params.into_inner();

    let mut conn = get_connection(&pool).await.unwrap();
    let result =
        get_ratings_by_restaurant_per_period(&mut conn, &restaurant_id, year, period).await;
    match result {
        Ok(restaurant_ratings) => HttpResponse::Ok().json(ApiResponse::success(restaurant_ratings)),
        Err(error) => {
            HttpResponse::InternalServerError().json(ApiResponse::<()>::error(error.to_string()))
        }
    }
}

#[get("/restaurants/{id}/is_rating_complete")]
async fn is_restaurant_rating_complete_route(
    pool: web::Data<MySqlPool>,
    req: HttpRequest,
    id: web::Path<String>,
) -> HttpResponse {
    if let Err(err) = validate_ip(&req) {
        return err;
    }

    if let Err(err) = validate_token(&req) {
        return err;
    }

    let mut conn = get_connection(&pool).await.unwrap();
    let result = is_restaurant_rating_complete(&mut conn, &id).await;
    match result {
        Ok(is_rating_complete) => HttpResponse::Ok().json(ApiResponse::success(is_rating_complete)),
        Err(error) => {
            HttpResponse::InternalServerError().json(ApiResponse::<()>::error(error.to_string()))
        }
    }
}

#[delete("/restaurants/{id}")]
async fn delete_restaurant_route(
    pool: web::Data<MySqlPool>,
    req: HttpRequest,
    id: web::Path<String>,
) -> HttpResponse {
    if let Err(err) = validate_ip(&req) {
        return err;
    }

    if let Err(err) = validate_token(&req) {
        return err;
    }

    let mut conn = get_connection(&pool).await.unwrap();
    let result = delete_restaurant(&mut conn, &id).await;
    match result {
        Ok(rows) => HttpResponse::Ok().json(ApiResponse::success(rows.last_insert_id())),
        Err(error) => {
            HttpResponse::InternalServerError().json(ApiResponse::<()>::error(error.to_string()))
        }
    }
}

#[post("/users/{user_id}/ratings")]
async fn rate_restaurant_route(
    pool: web::Data<MySqlPool>,
    req: HttpRequest,
    user_id: web::Path<String>,
    rating: web::Json<NewRating>,
) -> HttpResponse {
    if let Err(err) = validate_ip(&req) {
        return err;
    }

    if let Err(err) = validate_token(&req) {
        return err;
    }

    let mut conn = get_connection(&pool).await.unwrap();

    let rated = is_restaurant_rated_by_user(&mut conn, &rating.0.restaurant_id, &user_id)
        .await
        .unwrap_or_default();

    let result = match rated {
        false => create_rating(&mut conn, &rating.0).await,
        true => update_rating(&mut conn, &rating.0, &user_id).await,
    };
    match result {
        Ok(rating) => HttpResponse::Ok().json(ApiResponse::success(rating)),
        Err(error) => {
            HttpResponse::InternalServerError().json(ApiResponse::<()>::error(error.to_string()))
        }
    }
}

#[get("/users/{user_id}/ratings")]
async fn get_ratings_route(
    pool: web::Data<MySqlPool>,
    req: HttpRequest,
    user_id: web::Path<String>,
) -> HttpResponse {
    if let Err(err) = validate_ip(&req) {
        return err;
    }

    if let Err(err) = validate_token(&req) {
        return err;
    }

    let mut conn = get_connection(&pool).await.unwrap();
    let result = get_ratings_by_user(&mut conn, &user_id).await;
    match result {
        Ok(ratings) => HttpResponse::Ok().json(ApiResponse::success(ratings)),
        Err(error) => {
            HttpResponse::InternalServerError().json(ApiResponse::<()>::error(error.to_string()))
        }
    }
}

#[get("/users/{user_id}/ratings/{restaurant_id}")]
async fn get_rating_route(
    pool: web::Data<MySqlPool>,
    req: HttpRequest,
    params: web::Path<(String, String)>,
) -> HttpResponse {
    if let Err(err) = validate_ip(&req) {
        return err;
    }

    if let Err(err) = validate_token(&req) {
        return err;
    }

    let (user_id, restaurant_id) = params.into_inner();

    let mut conn = get_connection(&pool).await.unwrap();
    let result = get_rating(&mut conn, &user_id, &restaurant_id).await;
    match result {
        Ok(rating) => HttpResponse::Ok().json(ApiResponse::success(rating)),
        Err(error) => {
            HttpResponse::InternalServerError().json(ApiResponse::<()>::error(error.to_string()))
        }
    }
}

#[put("/users/{user_id}/ratings")]
async fn update_rating_route(
    pool: web::Data<MySqlPool>,
    req: HttpRequest,
    user_id: web::Path<String>,
    rating: web::Json<NewRating>,
) -> HttpResponse {
    if let Err(err) = validate_ip(&req) {
        return err;
    }

    if let Err(err) = validate_token(&req) {
        return err;
    }

    let mut conn = get_connection(&pool).await.unwrap();
    let result = update_rating(&mut conn, &rating.0, &user_id).await;
    match result {
        Ok(updated_rating) => HttpResponse::Ok().json(ApiResponse::success(updated_rating)),
        Err(error) => {
            HttpResponse::InternalServerError().json(ApiResponse::<()>::error(error.to_string()))
        }
    }
}

#[delete("/users/{user_id}/ratings/{rating_id}")]
async fn delete_rating_route(
    pool: web::Data<MySqlPool>,
    req: HttpRequest,
    params: web::Path<(String, i32)>,
) -> HttpResponse {
    if let Err(err) = validate_ip(&req) {
        return err;
    }

    if let Err(err) = validate_token(&req) {
        return err;
    }

    let (user_id, rating_id) = params.into_inner();

    let mut conn = get_connection(&pool).await.unwrap();
    let result = delete_rating(&mut conn, rating_id, &user_id).await;
    match result {
        Ok(rows) => HttpResponse::Ok().json(ApiResponse::success(rows.last_insert_id())),
        Err(error) => {
            HttpResponse::InternalServerError().json(ApiResponse::<()>::error(error.to_string()))
        }
    }
}
