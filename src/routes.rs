use std::collections::HashMap;

use actix_web::{delete, get, post, put, web, HttpRequest, HttpResponse};
use sqlx::MySqlPool;
use uuid::Uuid;

use crate::{auth, db_models::*, db_util, models::*};

#[get("/health")]
async fn health_route() -> HttpResponse {
    HttpResponse::Ok().json(ApiResponse::success("ok"))
}

#[post("/register")]
async fn register_user_route(
    pool: web::Data<MySqlPool>,
    req: HttpRequest,
    mut new_user: web::Json<NewUser>,
) -> HttpResponse {
    if let Err(err) = auth::validate_ip(&req) {
        return err;
    }

    new_user.0.id = Uuid::new_v4().to_string();
    let username = new_user.0.username.clone();

    let hashed_password = match auth::generate_password_hash(new_user.0.password.clone()) {
        Ok(password) => password,
        Err(error) => {
            return HttpResponse::InternalServerError()
                .json(ApiResponse::<()>::error(error.to_string()));
        }
    };
    new_user.0.password = hashed_password;

    let mut conn = db_util::get_connection(&pool).await.unwrap();
    let result = db_util::create_user(&mut conn, &new_user.0).await;

    match result {
        Ok(db_user) => {
            let token = auth::generate_token(&req, db_user.id.clone(), username);
            HttpResponse::Created().json(ApiResponse::success(User {
                id: db_user.id,
                username: db_user.username,
                password: db_user.password,
                color: db_user.color,
                token: token.clone(),
                ratings: db_user.ratings,
                group_memberships: db_user.group_memberships,
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
    if let Err(err) = auth::validate_ip(&req) {
        return err;
    }

    let username = credentials.0.username.clone();
    let password = credentials.0.password.clone();

    let mut conn = db_util::get_connection(&pool).await.unwrap();
    let result = db_util::get_user_by_credentials(&mut conn, &credentials.0.username.clone()).await;

    match result {
        Ok(users_result) => match users_result {
            Some(mut user) => {
                let is_valid_password = auth::validate_password(&user.password, &password);
                if is_valid_password {
                    let token = auth::generate_token(&req, user.id.clone(), username);
                    user.token.clone_from(&token);
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
    if let Err(err) = auth::validate_ip(&req) {
        return err;
    }

    let _user_claims = match auth::validate_token(&req) {
        Ok(claims) => claims,
        Err(err) => return err,
    };

    let result = db_util::get_users(&pool).await;
    match result {
        Ok(users) => HttpResponse::Ok().json(ApiResponse::success(users)),
        Err(error) => {
            HttpResponse::InternalServerError().json(ApiResponse::<()>::error(error.to_string()))
        }
    }
}

#[get("/users/{user_id}/oidc-links")]
async fn get_user_oidc_links_route(
    pool: web::Data<MySqlPool>,
    req: HttpRequest,
    path: web::Path<String>,
) -> HttpResponse {
    if let Err(err) = auth::validate_ip(&req) {
        return err;
    }
    let user_claims = match auth::validate_token(&req) {
        Ok(claims) => claims,
        Err(err) => return err,
    };
    let user_id = path.into_inner();
    if user_claims.id != user_id {
        return HttpResponse::Forbidden().json(ApiResponse::<()>::error("Unauthorized".into()));
    }
    let mut conn = match db_util::get_connection(&pool).await {
        Some(conn) => conn,
        None => {
            return HttpResponse::InternalServerError()
                .json(ApiResponse::<()>::error("DB Error".into()));
        }
    };
    match db_util::get_oidc_links_for_user(&mut conn, &user_id).await {
        Ok(links) => HttpResponse::Ok().json(ApiResponse::success(links)),
        Err(e) => HttpResponse::InternalServerError().json(ApiResponse::<()>::error(e.to_string())),
    }
}

#[delete("/users/{user_id}/oidc-links/{provider}")]
async fn unlink_oidc_route(
    pool: web::Data<MySqlPool>,
    req: HttpRequest,
    path: web::Path<(String, String)>,
) -> HttpResponse {
    if let Err(err) = auth::validate_ip(&req) {
        return err;
    }
    let user_claims = match auth::validate_token(&req) {
        Ok(claims) => claims,
        Err(err) => return err,
    };
    let (user_id, provider) = path.into_inner();
    if user_claims.id != user_id {
        return HttpResponse::Forbidden().json(ApiResponse::<()>::error("Unauthorized".into()));
    }
    let mut conn = match db_util::get_connection(&pool).await {
        Some(conn) => conn,
        None => {
            return HttpResponse::InternalServerError()
                .json(ApiResponse::<()>::error("DB Error".into()));
        }
    };
    match db_util::unlink_oidc(&mut conn, &user_id, &provider).await {
        Ok(_) => HttpResponse::Ok().json(ApiResponse::success(true)),
        Err(e) => HttpResponse::InternalServerError().json(ApiResponse::<()>::error(e.to_string())),
    }
}

#[put("/users/{id}")]
async fn update_user_route(
    pool: web::Data<MySqlPool>,
    req: HttpRequest,
    id: web::Path<String>,
    user: web::Json<NewUser>,
) -> HttpResponse {
    if let Err(err) = auth::validate_ip(&req) {
        return err;
    }

    let user_claims = match auth::validate_token(&req) {
        Ok(claims) => claims,
        Err(err) => return err,
    };
    let user_id = id.into_inner();
    if user_claims.id != user_id {
        return HttpResponse::Forbidden().json(ApiResponse::<()>::error("Unauthorized".into()));
    }

    let mut conn = db_util::get_connection(&pool).await.unwrap();
    let result = db_util::update_user(&mut conn, &user_id, &user).await;
    match result {
        Ok(updated_user) => HttpResponse::Ok().json(ApiResponse::success(updated_user)),
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
    if let Err(err) = auth::validate_ip(&req) {
        return err;
    }

    let user_claims = match auth::validate_token(&req) {
        Ok(claims) => claims,
        Err(err) => return err,
    };
    let user_id = id.into_inner();
    if user_claims.id != user_id {
        return HttpResponse::Forbidden().json(ApiResponse::<()>::error("Unauthorized".into()));
    }

    let mut conn = db_util::get_connection(&pool).await.unwrap();
    let result = db_util::delete_user(&mut conn, &user_id).await;
    match result {
        Ok(rows) => HttpResponse::Ok().json(ApiResponse::success(rows.last_insert_id())),
        Err(error) => {
            HttpResponse::InternalServerError().json(ApiResponse::<()>::error(error.to_string()))
        }
    }
}

#[post("/subscribe")]
async fn push_subscribe_route(
    pool: web::Data<MySqlPool>,
    req: HttpRequest,
    new_push_subscription: web::Json<NewPushSubscription>,
) -> HttpResponse {
    if let Err(err) = auth::validate_ip(&req) {
        return err;
    }

    let _user_claims = match auth::validate_token(&req) {
        Ok(claims) => claims,
        Err(err) => return err,
    };

    let mut conn = db_util::get_connection(&pool).await.unwrap();
    let result = db_util::create_push_subscription(
        &mut conn,
        &new_push_subscription.user_id,
        &new_push_subscription.subscription_info,
    )
    .await;
    match result {
        // TODO: Should we actually return this to the client?
        Ok(push_subscription) => HttpResponse::Ok().json(ApiResponse::success(push_subscription)),
        Err(error) => {
            HttpResponse::InternalServerError().json(ApiResponse::<()>::error(error.to_string()))
        }
    }
}

#[get("/groups/{user_id}")]
async fn get_group_memberships_by_user_route(
    pool: web::Data<MySqlPool>,
    req: HttpRequest,
    user_id: web::Path<String>,
) -> HttpResponse {
    if let Err(err) = auth::validate_ip(&req) {
        return err;
    }

    let _user_claims = match auth::validate_token(&req) {
        Ok(claims) => claims,
        Err(err) => return err,
    };

    let mut conn = db_util::get_connection(&pool).await.unwrap();
    let result = db_util::get_group_memberships_by_user(&mut conn, &user_id).await;
    match result {
        Ok(group_memberships) => HttpResponse::Ok().json(ApiResponse::success(group_memberships)),
        Err(error) => {
            HttpResponse::InternalServerError().json(ApiResponse::<()>::error(error.to_string()))
        }
    }
}

#[post("/groups")]
async fn create_group_route(
    pool: web::Data<MySqlPool>,
    req: HttpRequest,
    group: web::Json<NewGroup>,
) -> HttpResponse {
    if let Err(err) = auth::validate_ip(&req) {
        return err;
    }

    let _user_claims = match auth::validate_token(&req) {
        Ok(claims) => claims,
        Err(err) => return err,
    };

    let mut conn = db_util::get_connection(&pool).await.unwrap();
    let result = db_util::create_group(&mut conn, &group.0).await;
    match result {
        Ok(group_membership) => HttpResponse::Ok().json(ApiResponse::success(group_membership)),
        Err(error) => {
            HttpResponse::InternalServerError().json(ApiResponse::<()>::error(error.to_string()))
        }
    }
}

#[post("/groups/join")]
async fn join_group_route(
    pool: web::Data<MySqlPool>,
    req: HttpRequest,
    group_membership: web::Json<NewGroupMembership>,
) -> HttpResponse {
    if let Err(err) = auth::validate_ip(&req) {
        return err;
    }

    let _user_claims = match auth::validate_token(&req) {
        Ok(claims) => claims,
        Err(err) => return err,
    };

    let mut conn = db_util::get_connection(&pool).await.unwrap();
    let result = db_util::create_group_membership(&mut conn, &group_membership.0).await;
    match result {
        Ok(group_membership) => HttpResponse::Ok().json(ApiResponse::success(group_membership)),
        Err(error) => {
            HttpResponse::InternalServerError().json(ApiResponse::<()>::error(error.to_string()))
        }
    }
}

#[put("/restaurants/{id}")]
async fn update_restaurant_route(
    pool: web::Data<MySqlPool>,
    req: HttpRequest,
    id: web::Path<i32>,
    restaurant: web::Json<Restaurant>,
) -> HttpResponse {
    if let Err(err) = auth::validate_ip(&req) {
        return err;
    }

    let user_claims = match auth::validate_token(&req) {
        Ok(claims) => claims,
        Err(err) => return err,
    };

    let mut conn = db_util::get_connection(&pool).await.unwrap();

    let group_membership = db_util::get_group_memberships_by_user(&mut conn, &user_claims.id)
        .await
        .unwrap_or_default()
        .into_iter()
        .find(|gm| gm.group_id == restaurant.group_id);

    if let Some(membership) = group_membership {
        if !matches!(membership.role, Role::Admin) {
            return HttpResponse::Forbidden().json(ApiResponse::<()>::error(
                "Only admins can update restaurants".to_string(),
            ));
        }
    } else {
        return HttpResponse::Forbidden().json(ApiResponse::<()>::error(
            "User is not a member of this group".to_string(),
        ));
    }

    let result = db_util::update_restaurant(&mut conn, id.into_inner(), &restaurant.0).await;
    match result {
        Ok(query_result) => {
            HttpResponse::Ok().json(ApiResponse::success(query_result.rows_affected()))
        }
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
    if let Err(err) = auth::validate_ip(&req) {
        return err;
    }

    let _user_claims = match auth::validate_token(&req) {
        Ok(claims) => claims,
        Err(err) => return err,
    };

    let mut conn = db_util::get_connection(&pool).await.unwrap();
    let result = db_util::create_restaurant(&mut conn, &restaurant.0).await;
    match result {
        Ok(restaurant) => HttpResponse::Ok().json(ApiResponse::success(restaurant)),
        Err(error) => {
            HttpResponse::InternalServerError().json(ApiResponse::<()>::error(error.to_string()))
        }
    }
}

#[get("/restaurants")]
async fn get_restaurants_route(
    pool: web::Data<MySqlPool>,
    req: HttpRequest,
    query_params: web::Query<HashMap<String, String>>,
) -> HttpResponse {
    if let Err(err) = auth::validate_ip(&req) {
        return err;
    }

    let user_claims = match auth::validate_token(&req) {
        Ok(claims) => claims,
        Err(err) => return err,
    };

    let group_id = match query_params.get("group_id") {
        Some(group_id) => group_id,
        None => {
            return HttpResponse::BadRequest()
                .json(ApiResponse::<()>::error("group_id is missing".to_string()));
        }
    };

    let mut conn = db_util::get_connection(&pool).await.unwrap();

    let group_membership = db_util::get_group_memberships_by_user(&mut conn, &user_claims.id)
        .await
        .unwrap_or_default()
        .into_iter()
        .find(|gm| gm.group_id == *group_id);

    if group_membership.is_none() {
        return HttpResponse::Forbidden().json(ApiResponse::<()>::error(
            "User is not a member of this group".to_string(),
        ));
    }

    let result = db_util::get_restaurants(&mut conn, group_id).await;
    match result {
        Ok(restaurants) => HttpResponse::Ok().json(ApiResponse::success(restaurants)),
        Err(error) => {
            HttpResponse::InternalServerError().json(ApiResponse::<()>::error(error.to_string()))
        }
    }
}

#[get("/restaurants/{id}")]
async fn get_restaurant_route(
    pool: web::Data<MySqlPool>,
    req: HttpRequest,
    path: web::Path<i32>,
) -> HttpResponse {
    if let Err(err) = auth::validate_ip(&req) {
        return err;
    }

    let restaurant_id = path.into_inner();

    let mut conn = db_util::get_connection(&pool).await.unwrap();
    let result = db_util::get_restaurant(&mut conn, restaurant_id).await;
    match result {
        Ok(restaurant) => HttpResponse::Ok().json(ApiResponse::success(restaurant)),
        Err(error) => {
            HttpResponse::InternalServerError().json(ApiResponse::<()>::error(error.to_string()))
        }
    }
}

#[get("/restaurants_with_avg_rating")]
async fn get_restaurants_with_avg_rating_route(
    pool: web::Data<MySqlPool>,
    req: HttpRequest,
    query_params: web::Query<HashMap<String, String>>,
) -> HttpResponse {
    if let Err(err) = auth::validate_ip(&req) {
        return err;
    }

    let _user_claims = match auth::validate_token(&req) {
        Ok(claims) => claims,
        Err(err) => return err,
    };

    let group_id = match query_params.get("group_id") {
        Some(group_id) => group_id,
        None => {
            return HttpResponse::InternalServerError().json(ApiResponse::<()>::error(
                "group_id was not provided".to_string(),
            ));
        }
    };

    let mut conn = db_util::get_connection(&pool).await.unwrap();
    let result = db_util::get_restaurants_with_avg_rating(&mut conn, group_id).await;
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
    id: web::Path<i32>,
    query_params: web::Query<HashMap<String, String>>,
) -> HttpResponse {
    if let Err(err) = auth::validate_ip(&req) {
        return err;
    }

    let _user_claims = match auth::validate_token(&req) {
        Ok(claims) => claims,
        Err(err) => return err,
    };

    let group_id = match query_params.get("group_id") {
        Some(group_id) => group_id,
        None => {
            return HttpResponse::InternalServerError().json(ApiResponse::<()>::error(
                "group_id was not provided".to_string(),
            ));
        }
    };

    let mut conn = db_util::get_connection(&pool).await.unwrap();
    let result = db_util::get_ratings_by_restaurant(&mut conn, id.into_inner(), group_id).await;
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
    params: web::Path<(i32, i32, Period)>,
    query_params: web::Query<HashMap<String, String>>,
) -> HttpResponse {
    if let Err(err) = auth::validate_ip(&req) {
        return err;
    }

    let _user_claims = match auth::validate_token(&req) {
        Ok(claims) => claims,
        Err(err) => return err,
    };

    let (restaurant_id, year, period) = params.into_inner();
    let group_id = match query_params.get("group_id") {
        Some(group_id) => group_id,
        None => {
            return HttpResponse::InternalServerError().json(ApiResponse::<()>::error(
                "group_id was not provided".to_string(),
            ));
        }
    };

    let mut conn = db_util::get_connection(&pool).await.unwrap();
    let result = db_util::get_ratings_by_restaurant_per_period(
        &mut conn,
        group_id,
        restaurant_id,
        year,
        &period,
    )
    .await;
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
    push_client: web::Data<PushClient>,
    req: HttpRequest,
    id: web::Path<i32>,
    query_params: web::Query<HashMap<String, String>>,
) -> HttpResponse {
    if let Err(err) = auth::validate_ip(&req) {
        return err;
    }

    let _user_claims = match auth::validate_token(&req) {
        Ok(claims) => claims,
        Err(err) => return err,
    };

    let group_id = match query_params.get("group_id") {
        Some(group_id) => group_id,
        None => {
            return HttpResponse::InternalServerError().json(ApiResponse::<()>::error(
                "group_id was not provided".to_string(),
            ));
        }
    };

    let result = db_util::is_restaurant_rating_complete(
        &pool,
        Some(&push_client),
        id.into_inner(),
        group_id,
    )
    .await;
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
    id: web::Path<i32>,
    query_params: web::Query<HashMap<String, String>>,
) -> HttpResponse {
    if let Err(err) = auth::validate_ip(&req) {
        return err;
    }

    let user_claims = match auth::validate_token(&req) {
        Ok(claims) => claims,
        Err(err) => return err,
    };

    let group_id = match query_params.get("group_id") {
        Some(group_id) => group_id,
        None => {
            return HttpResponse::BadRequest()
                .json(ApiResponse::<()>::error("group_id is missing".to_string()));
        }
    };

    let mut conn = db_util::get_connection(&pool).await.unwrap();

    let group_membership = db_util::get_group_memberships_by_user(&mut conn, &user_claims.id)
        .await
        .unwrap_or_default()
        .into_iter()
        .find(|gm| gm.group_id == *group_id);

    if let Some(membership) = group_membership {
        if !matches!(membership.role, Role::Admin) {
            return HttpResponse::Forbidden().json(ApiResponse::<()>::error(
                "Only admins can delete restaurants".to_string(),
            ));
        }
    } else {
        return HttpResponse::Forbidden().json(ApiResponse::<()>::error(
            "User is not a member of this group".to_string(),
        ));
    }

    let result = db_util::delete_restaurant(&mut conn, id.into_inner(), group_id).await;
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
    if let Err(err) = auth::validate_ip(&req) {
        return err;
    }

    let user_claims = match auth::validate_token(&req) {
        Ok(claims) => claims,
        Err(err) => return err,
    };
    let user_id = user_id.into_inner();
    if user_claims.id != user_id {
        return HttpResponse::Forbidden().json(ApiResponse::<()>::error("Unauthorized".into()));
    }

    let mut conn = db_util::get_connection(&pool).await.unwrap();

    let mut new_rating = rating.into_inner();
    new_rating.user_id = user_claims.id.clone();

    let rated = db_util::is_restaurant_rated_by_user(
        &mut conn,
        new_rating.restaurant_id,
        &new_rating.user_id,
        &new_rating.group_id,
    )
    .await
    .unwrap_or_default();

    let result = match rated {
        false => db_util::create_rating(&mut conn, &new_rating).await,
        true => db_util::update_rating(&mut conn, &new_rating, &user_id).await,
    };
    match result {
        Ok(rating) => HttpResponse::Ok().json(ApiResponse::success(rating)),
        Err(error) => {
            HttpResponse::InternalServerError().json(ApiResponse::<()>::error(error.to_string()))
        }
    }
}

// #[get("/users/{user_id}/ratings")]
// async fn get_ratings_route(
//     pool: web::Data<MySqlPool>,
//     req: HttpRequest,
//     user_id: web::Path<String>,
// ) -> HttpResponse {
//     if let Err(err) = auth::validate_ip(&req) {
//         return err;
//     }
//
//     let _user_claims = match auth::validate_token(&req) {
//         Ok(claims) => claims,
//         Err(err) => return err,
//     };
//
//     let mut conn = db_util::get_connection(&pool).await.unwrap();
//     let result = db_util::get_ratings_by_user(&mut conn, &user_id).await;
//     match result {
//         Ok(ratings) => HttpResponse::Ok().json(ApiResponse::success(ratings)),
//         Err(error) => {
//             HttpResponse::InternalServerError().json(ApiResponse::<()>::error(error.to_string()))
//         }
//     }
// }

#[get("/users/{user_id}/ratings")]
async fn get_ratings_by_user_and_group_route(
    pool: web::Data<MySqlPool>,
    req: HttpRequest,
    user_id: web::Path<String>,
    query_params: web::Query<HashMap<String, String>>,
) -> HttpResponse {
    if let Err(err) = auth::validate_ip(&req) {
        return err;
    }

    let _user_claims = match auth::validate_token(&req) {
        Ok(claims) => claims,
        Err(err) => return err,
    };

    let group_id = match query_params.get("group_id") {
        Some(group_id) => group_id,
        None => {
            return HttpResponse::InternalServerError().json(ApiResponse::<()>::error(
                "group_id was not provided".to_string(),
            ));
        }
    };

    let mut conn = db_util::get_connection(&pool).await.unwrap();
    let result = db_util::get_ratings_by_user_and_group(&mut conn, &user_id, group_id).await;
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
    params: web::Path<(String, i32)>,
    query_params: web::Query<HashMap<String, String>>,
) -> HttpResponse {
    if let Err(err) = auth::validate_ip(&req) {
        return err;
    }

    let _user_claims = match auth::validate_token(&req) {
        Ok(claims) => claims,
        Err(err) => return err,
    };

    let (user_id, restaurant_id): (String, i32) = params.into_inner();
    let group_id = match query_params.get("group_id") {
        Some(group_id) => group_id,
        None => {
            return HttpResponse::InternalServerError().json(ApiResponse::<()>::error(
                "group_id was not provided".to_string(),
            ));
        }
    };

    let mut conn = db_util::get_connection(&pool).await.unwrap();
    let result =
        db_util::get_rating_by_restaurant(&mut conn, &user_id, group_id, restaurant_id).await;
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
    if let Err(err) = auth::validate_ip(&req) {
        return err;
    }

    let _user_claims = match auth::validate_token(&req) {
        Ok(claims) => claims,
        Err(err) => return err,
    };

    let mut conn = db_util::get_connection(&pool).await.unwrap();
    let result = db_util::update_rating(&mut conn, &rating.0, &user_id).await;
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
    query_params: web::Query<HashMap<String, String>>,
) -> HttpResponse {
    if let Err(err) = auth::validate_ip(&req) {
        return err;
    }

    let _user_claims = match auth::validate_token(&req) {
        Ok(claims) => claims,
        Err(err) => return err,
    };

    let (user_id, rating_id) = params.into_inner();
    let group_id = match query_params.get("group_id") {
        Some(group_id) => group_id,
        None => {
            return HttpResponse::InternalServerError().json(ApiResponse::<()>::error(
                "group_id was not provided".to_string(),
            ));
        }
    };

    let mut conn = db_util::get_connection(&pool).await.unwrap();
    let result = db_util::delete_rating(&mut conn, rating_id, &user_id, group_id).await;
    match result {
        Ok(rows) => HttpResponse::Ok().json(ApiResponse::success(rows.last_insert_id())),
        Err(error) => {
            HttpResponse::InternalServerError().json(ApiResponse::<()>::error(error.to_string()))
        }
    }
}
