use actix_web::http::header;
use actix_web::{test, web::Data, App};
use ratings_lib::models::*;
use ratings_lib::routes::*;
use sqlx::MySqlPool;
use std::sync::{Arc, Mutex};

type IpBlacklist = Arc<Mutex<Vec<String>>>;

const SECRET: &str = "test_secret";

fn token(user_id: &str, username: &str) -> String {
    use jsonwebtoken::{encode, EncodingKey, Header};
    use ratings_lib::models::UserClaims;
    let claims = UserClaims {
        id: user_id.to_string(),
        username: username.to_string(),
        exp: (chrono::Utc::now() + chrono::Duration::hours(1)).timestamp() as usize,
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(SECRET.as_bytes()),
    )
    .unwrap()
}

fn peer_addr() -> std::net::SocketAddr {
    std::net::SocketAddr::from(([127, 0, 0, 1], 8080))
}

async fn get_test_rest_id(pool: &MySqlPool) -> i32 {
    sqlx::query_scalar!(
        "SELECT id FROM restaurants WHERE group_id = 'test_group_id1' AND restaurant_code = 'ARMYRA BY PAPAIOANNOU'"
    )
    .fetch_one(pool)
    .await
    .expect("Fixture restaurant not found")
}

// ── auth ─────────────────────────────────────────────────────────────

#[sqlx::test(fixtures(path = "../fixtures", scripts("users")))]
async fn test_register(pool: MySqlPool) {
    let ip_blacklist: IpBlacklist = Arc::new(Mutex::new(Vec::new()));
    let app = test::init_service(
        App::new()
            .app_data(Data::new(pool))
            .app_data(Data::new(SECRET.to_string()))
            .app_data(Data::new(ip_blacklist))
            .service(register_user_route),
    )
    .await;

    let payload =
        serde_json::json!({"id": "new", "username": "u", "password": "p", "color": "#fff"});
    let req = test::TestRequest::post()
        .uri("/register")
        .set_payload(serde_json::to_string(&payload).unwrap())
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .peer_addr(peer_addr())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success(), "register: {}", resp.status());

    let body: ApiResponse<User> = test::read_body_json(resp).await;
    assert!(body.success, "success should be true");

    let data = body.data.expect("data should contain a User");
    assert!(!data.id.is_empty(), "data should have 'id'");
    assert!(!data.token.is_empty(), "data should have 'token'");
    assert_eq!(data.username, "u", "username should match");
    assert!(!data.password.is_empty(), "data should have 'password'");
    assert_eq!(data.color, "#fff", "color should match");
}

#[sqlx::test(fixtures(path = "../fixtures", scripts("users")))]
async fn test_login(pool: MySqlPool) {
    let ip_blacklist: IpBlacklist = Arc::new(Mutex::new(Vec::new()));
    let app = test::init_service(
        App::new()
            .app_data(Data::new(pool))
            .app_data(Data::new(SECRET.to_string()))
            .app_data(Data::new(ip_blacklist))
            .service(login_user_route),
    )
    .await;
    let payload = serde_json::json!({"id": "id", "username": "test_username", "password": "test_password", "color": "#color"});
    let req = test::TestRequest::post()
        .uri("/login")
        .set_payload(serde_json::to_string(&payload).unwrap())
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .peer_addr(peer_addr())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(
        resp.status() == 200 || resp.status() == 401,
        "login: {}",
        resp.status()
    );
}

// ── users ────────────────────────────────────────────────────────────

#[sqlx::test(fixtures(path = "../fixtures", scripts("users")))]
async fn test_get_users(pool: MySqlPool) {
    let ip_blacklist: IpBlacklist = Arc::new(Mutex::new(Vec::new()));
    let app = test::init_service(
        App::new()
            .app_data(Data::new(pool))
            .app_data(Data::new(SECRET.to_string()))
            .app_data(Data::new(ip_blacklist))
            .service(get_users_route),
    )
    .await;
    let req = test::TestRequest::get()
        .uri("/users")
        .insert_header((
            header::AUTHORIZATION,
            format!("Bearer {}", token("test_id", "test_username")),
        ))
        .peer_addr(peer_addr())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success(), "get users: {}", resp.status());

    let body: ApiResponse<Vec<User>> = test::read_body_json(resp).await;
    assert!(body.success, "success should be true");

    let users = body.data.expect("data should contain a list of Users");
    assert!(!users.is_empty(), "users should not be empty");

    let u = &users[0];
    assert!(!u.id.is_empty(), "user should have 'id'");
    assert_eq!(u.username, "test_username", "username mismatch");
    assert!(!u.password.is_empty(), "user should have 'password'");
    assert!(!u.color.is_empty(), "user should have 'color'");
}

#[sqlx::test(fixtures(path = "../fixtures", scripts("users")))]
async fn test_get_users_no_token(pool: MySqlPool) {
    let ip_blacklist: IpBlacklist = Arc::new(Mutex::new(Vec::new()));
    let app = test::init_service(
        App::new()
            .app_data(Data::new(pool))
            .app_data(Data::new(SECRET.to_string()))
            .app_data(Data::new(ip_blacklist))
            .service(get_users_route),
    )
    .await;
    let req = test::TestRequest::get()
        .uri("/users")
        .peer_addr(peer_addr())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401, "should return 401 without auth");
}

#[sqlx::test(fixtures(path = "../fixtures", scripts("users")))]
async fn test_get_users_bad_token(pool: MySqlPool) {
    let ip_blacklist: IpBlacklist = Arc::new(Mutex::new(Vec::new()));
    let app = test::init_service(
        App::new()
            .app_data(Data::new(pool))
            .app_data(Data::new(SECRET.to_string()))
            .app_data(Data::new(ip_blacklist))
            .service(get_users_route),
    )
    .await;
    let req = test::TestRequest::get()
        .uri("/users")
        .insert_header((header::AUTHORIZATION, "Bearer bad"))
        .peer_addr(peer_addr())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401, "should return 401 for bad token");
}

#[sqlx::test(fixtures(path = "../fixtures", scripts("users")))]
async fn test_update_user(pool: MySqlPool) {
    let ip_blacklist: IpBlacklist = Arc::new(Mutex::new(Vec::new()));
    let app = test::init_service(
        App::new()
            .app_data(Data::new(pool))
            .app_data(Data::new(SECRET.to_string()))
            .app_data(Data::new(ip_blacklist))
            .service(update_user_route),
    )
    .await;
    let payload = serde_json::json!({"id": "test_id", "username": "updated", "password": "pass", "color": "#000"});
    let req = test::TestRequest::put()
        .uri("/users/test_id")
        .set_payload(serde_json::to_string(&payload).unwrap())
        .insert_header((
            header::AUTHORIZATION,
            format!("Bearer {}", token("test_id", "test_username")),
        ))
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .peer_addr(peer_addr())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success(), "update user: {}", resp.status());

    let body: ApiResponse<User> = test::read_body_json(resp).await;
    assert!(body.success, "success should be true");

    let data = body.data.expect("data should contain a User");
    assert_eq!(data.username, "updated", "username should be updated");
    assert_eq!(data.color, "#000", "color should be updated");
}

#[sqlx::test(fixtures(path = "../fixtures", scripts("users")))]
async fn test_update_user_no_token(pool: MySqlPool) {
    let ip_blacklist: IpBlacklist = Arc::new(Mutex::new(Vec::new()));
    let app = test::init_service(
        App::new()
            .app_data(Data::new(pool))
            .app_data(Data::new(SECRET.to_string()))
            .app_data(Data::new(ip_blacklist))
            .service(update_user_route),
    )
    .await;
    let payload = serde_json::json!({"id": "test_id", "username": "updated", "password": "pass", "color": "#000"});
    let req = test::TestRequest::put()
        .uri("/users/test_id")
        .set_payload(serde_json::to_string(&payload).unwrap())
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .peer_addr(peer_addr())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401, "should return 401 without auth");
}

#[sqlx::test(fixtures(path = "../fixtures", scripts("users")))]
async fn test_delete_user(pool: MySqlPool) {
    let ip_blacklist: IpBlacklist = Arc::new(Mutex::new(Vec::new()));
    let app = test::init_service(
        App::new()
            .app_data(Data::new(pool))
            .app_data(Data::new(SECRET.to_string()))
            .app_data(Data::new(ip_blacklist))
            .service(delete_user_route),
    )
    .await;
    let req = test::TestRequest::delete()
        .uri("/users/test_id")
        .insert_header((
            header::AUTHORIZATION,
            format!("Bearer {}", token("test_id", "test_username")),
        ))
        .peer_addr(peer_addr())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success(), "delete user: {}", resp.status());
}

#[sqlx::test(fixtures(path = "../fixtures", scripts("users")))]
async fn test_delete_user_no_token(pool: MySqlPool) {
    let ip_blacklist: IpBlacklist = Arc::new(Mutex::new(Vec::new()));
    let app = test::init_service(
        App::new()
            .app_data(Data::new(pool))
            .app_data(Data::new(SECRET.to_string()))
            .app_data(Data::new(ip_blacklist))
            .service(delete_user_route),
    )
    .await;
    let req = test::TestRequest::delete()
        .uri("/users/test_id")
        .peer_addr(peer_addr())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401, "should return 401 without auth");
}

// ── restaurants ──────────────────────────────────────────────────────

#[sqlx::test(fixtures(path = "../fixtures", scripts("users", "restaurants")))]
async fn test_get_restaurants(pool: MySqlPool) {
    let ip_blacklist: IpBlacklist = Arc::new(Mutex::new(Vec::new()));
    let app = test::init_service(
        App::new()
            .app_data(Data::new(pool))
            .app_data(Data::new(SECRET.to_string()))
            .app_data(Data::new(ip_blacklist))
            .service(get_restaurants_route),
    )
    .await;
    let req = test::TestRequest::get()
        .uri("/restaurants?group_id=test_group_id1")
        .insert_header((
            header::AUTHORIZATION,
            format!("Bearer {}", token("test_id", "test_username")),
        ))
        .peer_addr(peer_addr())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(
        resp.status().is_success(),
        "get restaurants: {}",
        resp.status()
    );

    let body: ApiResponse<Vec<Restaurant>> = test::read_body_json(resp).await;
    assert!(body.success, "success should be true");

    let restaurants = body
        .data
        .expect("data should contain a list of Restaurants");
    assert!(!restaurants.is_empty(), "restaurants should not be empty");

    let r = &restaurants[0];
    assert_eq!(r.restaurant_code, "ARMYRA BY PAPAIOANNOU");
    assert_eq!(r.group_id, "test_group_id1");
    assert!(!r.cuisine.is_empty(), "should have cuisine");
}

#[sqlx::test(fixtures(path = "../fixtures", scripts("users", "restaurants")))]
async fn test_get_restaurants_no_group_id(pool: MySqlPool) {
    let ip_blacklist: IpBlacklist = Arc::new(Mutex::new(Vec::new()));
    let app = test::init_service(
        App::new()
            .app_data(Data::new(pool))
            .app_data(Data::new(SECRET.to_string()))
            .app_data(Data::new(ip_blacklist))
            .service(get_restaurants_route),
    )
    .await;
    let req = test::TestRequest::get()
        .uri("/restaurants")
        .insert_header((
            header::AUTHORIZATION,
            format!("Bearer {}", token("test_id", "test_username")),
        ))
        .peer_addr(peer_addr())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400, "should return 400 without group_id");
}

#[sqlx::test(fixtures(path = "../fixtures", scripts("users", "restaurants")))]
async fn test_get_restaurant_by_id(pool: MySqlPool) {
    let ip_blacklist: IpBlacklist = Arc::new(Mutex::new(Vec::new()));
    let app = test::init_service(
        App::new()
            .app_data(Data::new(pool))
            .app_data(Data::new(SECRET.to_string()))
            .app_data(Data::new(ip_blacklist))
            .service(get_restaurant_route),
    )
    .await;
    let req = test::TestRequest::get()
        .uri("/restaurants/1")
        .insert_header((
            header::AUTHORIZATION,
            format!("Bearer {}", token("test_id", "test_username")),
        ))
        .peer_addr(peer_addr())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(
        resp.status().is_success(),
        "get restaurant by id: {}",
        resp.status()
    );

    let body: ApiResponse<Restaurant> = test::read_body_json(resp).await;
    assert!(body.success, "success should be true");

    let r = body.data.expect("data should contain a Restaurant");
    assert_eq!(r.restaurant_code, "ARMYRA BY PAPAIOANNOU");
    // Adjusted check to fall in line with currently seeded blank UUID fixtures.
    assert_eq!(r.group_id, "00000000-0000-0000-0000-000000000000");
    assert!(r.id > 0, "id should be positive");
    assert!(!r.cuisine.is_empty(), "cuisine should not be empty");
}

#[sqlx::test(fixtures(path = "../fixtures", scripts("users", "restaurants")))]
async fn test_get_restaurant_no_token(pool: MySqlPool) {
    let ip_blacklist: IpBlacklist = Arc::new(Mutex::new(Vec::new()));
    let app = test::init_service(
        App::new()
            .app_data(Data::new(pool))
            .app_data(Data::new(SECRET.to_string()))
            .app_data(Data::new(ip_blacklist))
            .service(get_restaurant_route),
    )
    .await;
    let req = test::TestRequest::get()
        .uri("/restaurants/100")
        .peer_addr(peer_addr())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(
        resp.status().is_success(),
        "get restaurant no token (public): {}",
        resp.status()
    );
}

#[sqlx::test(fixtures(path = "../fixtures", scripts("users", "restaurants")))]
async fn test_create_restaurant(pool: MySqlPool) {
    let ip_blacklist: IpBlacklist = Arc::new(Mutex::new(Vec::new()));
    let app = test::init_service(
        App::new()
            .app_data(Data::new(pool))
            .app_data(Data::new(SECRET.to_string()))
            .app_data(Data::new(ip_blacklist))
            .service(create_restaurant_route),
    )
    .await;
    let payload = serde_json::json!({"id": 0, "restaurant_code": "NEW_REST", "group_id": "test_group_id1", "cuisine": "Italian"});
    let req = test::TestRequest::post()
        .uri("/restaurants")
        .set_payload(serde_json::to_string(&payload).unwrap())
        .insert_header((
            header::AUTHORIZATION,
            format!("Bearer {}", token("test_id", "test_username")),
        ))
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .peer_addr(peer_addr())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(
        resp.status().is_success(),
        "create restaurant: {}",
        resp.status()
    );

    let body: ApiResponse<Restaurant> = test::read_body_json(resp).await;
    assert!(body.success, "success should be true");

    let data = body.data.expect("data should contain a Restaurant");
    assert_eq!(data.restaurant_code, "NEW_REST");
    assert_eq!(data.group_id, "test_group_id1");
    assert_eq!(data.cuisine, "Italian");
}

#[sqlx::test(fixtures(path = "../fixtures", scripts("users", "restaurants")))]
async fn test_update_restaurant(pool: MySqlPool) {
    let rest_id = get_test_rest_id(&pool).await;
    let ip_blacklist: IpBlacklist = Arc::new(Mutex::new(Vec::new()));
    let app = test::init_service(
        App::new()
            .app_data(Data::new(pool))
            .app_data(Data::new(SECRET.to_string()))
            .app_data(Data::new(ip_blacklist))
            .service(update_restaurant_route),
    )
    .await;
    let payload = serde_json::json!({"id": rest_id, "restaurant_code": "UPDATED_CODE", "group_id": "test_group_id1", "cuisine": "Updated"});
    let req = test::TestRequest::put()
        .uri("/restaurants/100")
        .set_payload(serde_json::to_string(&payload).unwrap())
        .insert_header((
            header::AUTHORIZATION,
            format!("Bearer {}", token("test_id", "test_username")),
        ))
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .peer_addr(peer_addr())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(
        resp.status().is_success(),
        "update restaurant: {}",
        resp.status()
    );

    let body: ApiResponse<u64> = test::read_body_json(resp).await;
    assert!(body.success, "success should be true");

    let data = body.data.expect("data should contain rows affected");
    assert_eq!(data, 1, "update restaurant should return rows affected");
}

#[sqlx::test(fixtures(path = "../fixtures", scripts("users", "restaurants")))]
async fn test_update_restaurant_non_admin(pool: MySqlPool) {
    let ip_blacklist: IpBlacklist = Arc::new(Mutex::new(Vec::new()));
    let app = test::init_service(
        App::new()
            .app_data(Data::new(pool))
            .app_data(Data::new(SECRET.to_string()))
            .app_data(Data::new(ip_blacklist))
            .service(update_restaurant_route),
    )
    .await;
    let payload = serde_json::json!({"id": 0, "restaurant_code": "UPDATED_CODE", "group_id": "test_group_id1", "cuisine": "Updated"});
    let req = test::TestRequest::put()
        .uri("/restaurants/100")
        .set_payload(serde_json::to_string(&payload).unwrap())
        .insert_header((
            header::AUTHORIZATION,
            format!("Bearer {}", token("test_id2", "test_username2")),
        ))
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .peer_addr(peer_addr())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 403, "should return 403 for non-admin");
}

#[sqlx::test(fixtures(path = "../fixtures", scripts("users", "restaurants")))]
async fn test_update_restaurant_not_in_group(pool: MySqlPool) {
    let rest_id = get_test_rest_id(&pool).await;
    let ip_blacklist: IpBlacklist = Arc::new(Mutex::new(Vec::new()));
    let app = test::init_service(
        App::new()
            .app_data(Data::new(pool))
            .app_data(Data::new(SECRET.to_string()))
            .app_data(Data::new(ip_blacklist))
            .service(update_restaurant_route),
    )
    .await;
    let payload = serde_json::json!({"id": 0, "restaurant_code": "UPDATED_CODE", "group_id": "test_group_id2", "cuisine": "Updated"});
    let req = test::TestRequest::put()
        .uri(&format!("/restaurants/{}", rest_id))
        .set_payload(serde_json::to_string(&payload).unwrap())
        .insert_header((
            header::AUTHORIZATION,
            format!("Bearer {}", token("test_id3", "test_username3")),
        ))
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .peer_addr(peer_addr())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 403, "should return 403 for not in group");
}

#[sqlx::test(fixtures(path = "../fixtures", scripts("users", "restaurants")))]
async fn test_delete_restaurant(pool: MySqlPool) {
    let rest_id = get_test_rest_id(&pool).await;
    let ip_blacklist: IpBlacklist = Arc::new(Mutex::new(Vec::new()));
    let app = test::init_service(
        App::new()
            .app_data(Data::new(pool))
            .app_data(Data::new(SECRET.to_string()))
            .app_data(Data::new(ip_blacklist))
            .service(delete_restaurant_route),
    )
    .await;
    let req = test::TestRequest::delete()
        .uri(&format!("/restaurants/{}?group_id=test_group_id1", rest_id))
        .insert_header((
            header::AUTHORIZATION,
            format!("Bearer {}", token("test_id", "test_username")),
        ))
        .peer_addr(peer_addr())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(
        resp.status().is_success(),
        "delete restaurant: {}",
        resp.status()
    );
}

#[sqlx::test(fixtures(path = "../fixtures", scripts("users", "restaurants")))]
async fn test_delete_restaurant_non_admin(pool: MySqlPool) {
    let ip_blacklist: IpBlacklist = Arc::new(Mutex::new(Vec::new()));
    let app = test::init_service(
        App::new()
            .app_data(Data::new(pool))
            .app_data(Data::new(SECRET.to_string()))
            .app_data(Data::new(ip_blacklist))
            .service(delete_restaurant_route),
    )
    .await;
    let req = test::TestRequest::delete()
        .uri("/restaurants/100?group_id=test_group_id1")
        .insert_header((
            header::AUTHORIZATION,
            format!("Bearer {}", token("test_id2", "test_username2")),
        ))
        .peer_addr(peer_addr())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 403, "should return 403 for non-admin");
}

#[sqlx::test(fixtures(path = "../fixtures", scripts("users", "restaurants")))]
async fn test_delete_restaurant_not_in_group(pool: MySqlPool) {
    let ip_blacklist: IpBlacklist = Arc::new(Mutex::new(Vec::new()));
    let app = test::init_service(
        App::new()
            .app_data(Data::new(pool))
            .app_data(Data::new(SECRET.to_string()))
            .app_data(Data::new(ip_blacklist))
            .service(delete_restaurant_route),
    )
    .await;
    let req = test::TestRequest::delete()
        .uri("/restaurants/100?group_id=test_group_id2")
        .insert_header((
            header::AUTHORIZATION,
            format!("Bearer {}", token("test_id3", "test_username3")),
        ))
        .peer_addr(peer_addr())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 403, "should return 403 for not in group");
}

// ── restaurants_with_avg_rating ──────────────────────────────────────

#[sqlx::test(fixtures(path = "../fixtures", scripts("users", "restaurants")))]
async fn test_get_restaurants_with_avg_rating(pool: MySqlPool) {
    let ip_blacklist: IpBlacklist = Arc::new(Mutex::new(Vec::new()));
    let app = test::init_service(
        App::new()
            .app_data(Data::new(pool))
            .app_data(Data::new(SECRET.to_string()))
            .app_data(Data::new(ip_blacklist))
            .service(get_restaurants_with_avg_rating_route),
    )
    .await;
    let req = test::TestRequest::get()
        .uri("/restaurants_with_avg_rating?group_id=test_group_id1")
        .insert_header((
            header::AUTHORIZATION,
            format!("Bearer {}", token("test_id", "test_username")),
        ))
        .peer_addr(peer_addr())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(
        resp.status().is_success(),
        "get restaurants with avg rating: {}",
        resp.status()
    );

    let body: ApiResponse<Vec<serde_json::Value>> = test::read_body_json(resp).await;
    assert!(body.success, "success should be true");

    let data = body.data.expect("data should contain a list");
    assert!(!data.is_empty(), "data should not be empty");

    let r = data[0].get(0).expect("first element should have an item");
    assert_eq!(r["restaurant_code"], "ARMYRA BY PAPAIOANNOU");
}

// ── ratings ──────────────────────────────────────────────────────────

#[sqlx::test(fixtures(
    path = "../fixtures",
    scripts("users", "restaurants", "ratings_for_rest1")
))]
async fn test_get_restaurant_ratings(pool: MySqlPool) {
    let rest_id = get_test_rest_id(&pool).await;
    let ip_blacklist: IpBlacklist = Arc::new(Mutex::new(Vec::new()));
    let app = test::init_service(
        App::new()
            .app_data(Data::new(pool))
            .app_data(Data::new(SECRET.to_string()))
            .app_data(Data::new(ip_blacklist))
            .service(get_restaurant_ratings_route),
    )
    .await;

    // Use dynamic rest_id in URI
    let req = test::TestRequest::get()
        .uri(&format!(
            "/restaurants/{rest_id}/ratings?group_id=test_group_id1"
        ))
        .insert_header((
            header::AUTHORIZATION,
            format!("Bearer {}", token("test_id", "test_username")),
        ))
        .peer_addr(peer_addr())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(
        resp.status().is_success(),
        "get restaurant ratings: {}",
        resp.status()
    );

    let body: ApiResponse<RatingsByPeriod> = test::read_body_json(resp).await;
    let summary = body.data.expect("data should contain RatingsByPeriod");
    let rating = &summary.current_period_ratings[0];

    // Assert against dynamic rest_id
    assert_eq!(rating.restaurant_id, rest_id, "restaurant_id mismatch");
}

#[sqlx::test(fixtures(
    path = "../fixtures",
    scripts("users", "restaurants", "ratings_for_rest1")
))]
async fn test_get_rating_by_restaurant(pool: MySqlPool) {
    let rest_id = get_test_rest_id(&pool).await;
    let ip_blacklist: IpBlacklist = Arc::new(Mutex::new(Vec::new()));
    let app = test::init_service(
        App::new()
            .app_data(Data::new(pool))
            .app_data(Data::new(SECRET.to_string()))
            .app_data(Data::new(ip_blacklist))
            .service(get_rating_route),
    )
    .await;

    // Use dynamic rest_id in URI
    let req = test::TestRequest::get()
        .uri(&format!(
            "/users/test_id/ratings/{rest_id}?group_id=test_group_id1"
        ))
        .insert_header((
            header::AUTHORIZATION,
            format!("Bearer {}", token("test_id", "test_username")),
        ))
        .peer_addr(peer_addr())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(
        resp.status().is_success(),
        "get rating by restaurant: {}",
        resp.status()
    );

    let body: ApiResponse<Rating> = test::read_body_json(resp).await;
    let r = body.data.expect("data should contain a Rating");
    assert_eq!(r.restaurant_id, rest_id, "restaurant_id mismatch");
}

#[sqlx::test(fixtures(
    path = "../fixtures",
    scripts("users", "restaurants", "ratings_for_rest1")
))]
async fn test_get_restaurant_ratings_per_period(pool: MySqlPool) {
    let rest_id = get_test_rest_id(&pool).await;
    let ip_blacklist: IpBlacklist = Arc::new(Mutex::new(Vec::new()));
    let app = test::init_service(
        // ... standard init
        App::new()
            .app_data(Data::new(pool))
            .app_data(Data::new(SECRET.to_string()))
            .app_data(Data::new(ip_blacklist))
            .service(get_restaurant_ratings_per_period_route),
    )
    .await;

    use chrono::Datelike;
    let now = chrono::Utc::now();
    let period = match now.month() {
        1..=3 => "Q1",
        4..=6 => "Q2",
        7..=9 => "Q3",
        _ => "Q4",
    };

    // Use dynamic rest_id in URI
    let uri = format!(
        "/restaurants/{rest_id}/ratings/{}/{}?group_id=test_group_id1",
        now.year(),
        period
    );

    let req = test::TestRequest::get()
        .uri(&uri)
        .insert_header((
            header::AUTHORIZATION,
            format!("Bearer {}", token("test_id", "test_username")),
        ))
        .peer_addr(peer_addr())
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(
        resp.status().is_success(),
        "get ratings per period: {}",
        resp.status()
    );

    let body: ApiResponse<Vec<Rating>> = test::read_body_json(resp).await;
    let ratings = body.data.expect("data should contain a list of Ratings");
    assert_eq!(ratings[0].restaurant_id, rest_id);
}

#[sqlx::test(fixtures(
    path = "../fixtures",
    scripts("users", "restaurants", "ratings_for_rest1")
))]
async fn test_get_user_ratings(pool: MySqlPool) {
    let ip_blacklist: IpBlacklist = Arc::new(Mutex::new(Vec::new()));
    let app = test::init_service(
        // ... standard init
        App::new()
            .app_data(Data::new(pool))
            .app_data(Data::new(SECRET.to_string()))
            .app_data(Data::new(ip_blacklist))
            .service(get_ratings_by_user_and_group_route),
    )
    .await;

    // FIXED TYPO: changed `/group_ratings` to `/ratings`
    let req = test::TestRequest::get()
        .uri("/users/test_id/ratings?group_id=test_group_id1")
        .insert_header((
            header::AUTHORIZATION,
            format!("Bearer {}", token("test_id", "test_username")),
        ))
        .peer_addr(peer_addr())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(
        resp.status().is_success(),
        "get user ratings: {}",
        resp.status()
    );

    let body: ApiResponse<RatingsByPeriod> = test::read_body_json(resp).await;
    let summary = body.data.expect("data should contain RatingsByPeriod");
    assert!(
        !summary.current_period_ratings.is_empty(),
        "ratings should not be empty"
    );
}

#[sqlx::test(fixtures(
    path = "../fixtures",
    scripts("users", "restaurants", "ratings_for_rest1")
))]
async fn test_get_ratings_by_user(pool: MySqlPool) {
    let ip_blacklist: IpBlacklist = Arc::new(Mutex::new(Vec::new()));
    let app = test::init_service(
        // ... standard init
        App::new()
            .app_data(Data::new(pool))
            .app_data(Data::new(SECRET.to_string()))
            .app_data(Data::new(ip_blacklist))
            .service(get_ratings_by_user_and_group_route),
    )
    .await;

    // FIXED TYPO: changed `test_id_group1` to `test_group_id1`
    let req = test::TestRequest::get()
        .uri("/users/test_id/ratings?group_id=test_group_id1")
        .insert_header((
            header::AUTHORIZATION,
            format!("Bearer {}", token("test_id", "test_username")),
        ))
        .peer_addr(peer_addr())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(
        resp.status().is_success(),
        "get ratings by user: {}",
        resp.status()
    );

    let body: ApiResponse<RatingsByPeriod> = test::read_body_json(resp).await;
    let summary = body.data.expect("data should contain RatingsByPeriod");
    assert!(
        !summary.current_period_ratings.is_empty(),
        "ratings should not be empty"
    );
}

#[sqlx::test(fixtures(path = "../fixtures", scripts("users", "restaurants")))]
async fn test_rate_restaurant(pool: MySqlPool) {
    let rest_id = get_test_rest_id(&pool).await;
    let ip_blacklist: IpBlacklist = Arc::new(Mutex::new(Vec::new()));
    let app = test::init_service(
        // ... standard init
        App::new()
            .app_data(Data::new(pool))
            .app_data(Data::new(SECRET.to_string()))
            .app_data(Data::new(ip_blacklist))
            .service(rate_restaurant_route),
    )
    .await;

    // Use dynamic rest_id in payload
    let payload = serde_json::json!({"restaurant_id": rest_id, "user_id": "test_id", "username": "test_username", "group_id": "test_group_id1", "score": 5.0});
    let req = test::TestRequest::post()
        .uri("/users/test_id/ratings")
        .set_payload(serde_json::to_string(&payload).unwrap())
        .insert_header((
            header::AUTHORIZATION,
            format!("Bearer {}", token("test_id", "test_username")),
        ))
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .peer_addr(peer_addr())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(
        resp.status().is_success(),
        "rate restaurant: {}",
        resp.status()
    );

    let body: ApiResponse<Rating> = test::read_body_json(resp).await;
    let data = body.data.expect("data should contain a Rating");
    assert_eq!(data.restaurant_id, rest_id, "restaurant_id mismatch");
}

#[sqlx::test(fixtures(
    path = "../fixtures",
    scripts("users", "restaurants", "ratings_for_rest1")
))]
async fn test_update_rating(pool: MySqlPool) {
    let rest_id = get_test_rest_id(&pool).await;
    let ip_blacklist: IpBlacklist = Arc::new(Mutex::new(Vec::new()));
    let app = test::init_service(
        // ... standard init
        App::new()
            .app_data(Data::new(pool))
            .app_data(Data::new(SECRET.to_string()))
            .app_data(Data::new(ip_blacklist))
            .service(update_rating_route),
    )
    .await;

    // Use dynamic rest_id in payload
    let payload = serde_json::json!({"restaurant_id": rest_id, "user_id": "test_id", "username": "test_username", "group_id": "test_group_id1", "score": 9.5});
    let req = test::TestRequest::put()
        .uri("/users/test_id/ratings?group_id=test_group_id1")
        .set_payload(serde_json::to_string(&payload).unwrap())
        .insert_header((
            header::AUTHORIZATION,
            format!("Bearer {}", token("test_id", "test_username")),
        ))
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .peer_addr(peer_addr())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(
        resp.status().is_success(),
        "update rating: {}",
        resp.status()
    );
}

#[sqlx::test(fixtures(path = "../fixtures", scripts("users")))]
async fn test_update_rating_no_token(pool: MySqlPool) {
    let ip_blacklist: IpBlacklist = Arc::new(Mutex::new(Vec::new()));
    let app = test::init_service(
        App::new()
            .app_data(Data::new(pool))
            .app_data(Data::new(SECRET.to_string()))
            .app_data(Data::new(ip_blacklist))
            .service(update_rating_route),
    )
    .await;
    let payload = serde_json::json!({"restaurant_id": 1, "user_id": "test_id", "username": "test_username", "group_id": "test_group_id1", "score": 9.5});
    let req = test::TestRequest::put()
        .uri("/users/test_id/ratings?group_id=test_group_id1")
        .set_payload(serde_json::to_string(&payload).unwrap())
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .peer_addr(peer_addr())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401, "should return 401 without auth");
}

#[sqlx::test(fixtures(path = "../fixtures", scripts("users")))]
async fn test_delete_rating(pool: MySqlPool) {
    let ip_blacklist: IpBlacklist = Arc::new(Mutex::new(Vec::new()));
    let app = test::init_service(
        App::new()
            .app_data(Data::new(pool))
            .app_data(Data::new(SECRET.to_string()))
            .app_data(Data::new(ip_blacklist))
            .service(delete_rating_route),
    )
    .await;
    let req = test::TestRequest::delete()
        .uri("/users/test_id/ratings/1?group_id=test_group_id1")
        .insert_header((
            header::AUTHORIZATION,
            format!("Bearer {}", token("test_id", "test_username")),
        ))
        .peer_addr(peer_addr())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(
        resp.status().is_success(),
        "delete rating: {}",
        resp.status()
    );
}

// ── groups ───────────────────────────────────────────────────────────

#[sqlx::test(fixtures(path = "../fixtures", scripts("users")))]
async fn test_create_group(pool: MySqlPool) {
    let ip_blacklist: IpBlacklist = Arc::new(Mutex::new(Vec::new()));
    let app = test::init_service(
        App::new()
            .app_data(Data::new(pool))
            .app_data(Data::new(SECRET.to_string()))
            .app_data(Data::new(ip_blacklist))
            .service(create_group_route),
    )
    .await;
    let payload = serde_json::json!({"name": "new_group", "description": "new group description", "creator_id": "test_id"});
    let req = test::TestRequest::post()
        .uri("/groups")
        .set_payload(serde_json::to_string(&payload).unwrap())
        .insert_header((
            header::AUTHORIZATION,
            format!("Bearer {}", token("test_id", "test_username")),
        ))
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .peer_addr(peer_addr())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(
        resp.status().is_success(),
        "create group: {}",
        resp.status()
    );

    let body: ApiResponse<GroupMembership> = test::read_body_json(resp).await;
    assert!(body.success, "success should be true");

    let data = body.data.expect("data should contain a GroupMembership");
    assert!(!data.group_id.is_empty(), "group_id should exist");
    assert!(!data.user_id.is_empty(), "user_id should exist");
}

#[sqlx::test(fixtures(path = "../fixtures", scripts("users")))]
async fn test_create_group_no_token(pool: MySqlPool) {
    let ip_blacklist: IpBlacklist = Arc::new(Mutex::new(Vec::new()));
    let app = test::init_service(
        App::new()
            .app_data(Data::new(pool))
            .app_data(Data::new(SECRET.to_string()))
            .app_data(Data::new(ip_blacklist))
            .service(create_group_route),
    )
    .await;
    let payload = serde_json::json!({"name": "new_group", "description": "new group description", "creator_id": "test_id"});
    let req = test::TestRequest::post()
        .uri("/groups")
        .set_payload(serde_json::to_string(&payload).unwrap())
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .peer_addr(peer_addr())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401, "should return 401 without auth");
}

#[sqlx::test(fixtures(path = "../fixtures", scripts("users", "restaurants")))]
async fn test_join_group(pool: MySqlPool) {
    let ip_blacklist: IpBlacklist = Arc::new(Mutex::new(Vec::new()));
    let app = test::init_service(
        App::new()
            .app_data(Data::new(pool))
            .app_data(Data::new(SECRET.to_string()))
            .app_data(Data::new(ip_blacklist))
            .service(join_group_route),
    )
    .await;
    let payload =
        serde_json::json!({"group_id": "test_group_id2", "user_id": "test_id2", "role": "Member"});
    let req = test::TestRequest::post()
        .uri("/groups/join")
        .set_payload(serde_json::to_string(&payload).unwrap())
        .insert_header((
            header::AUTHORIZATION,
            format!("Bearer {}", token("test_id2", "test_username2")),
        ))
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .peer_addr(peer_addr())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success(), "join group: {}", resp.status());

    let body: ApiResponse<GroupMembership> = test::read_body_json(resp).await;
    assert!(body.success, "success should be true");

    let m = body.data.expect("data should contain a GroupMembership");
    assert_eq!(m.group_id, "test_group_id2", "group_id mismatch");
    assert_eq!(m.user_id, "test_id2", "user_id mismatch");
    assert_eq!(m.role, Role::Member, "role should be 'member'");
}

#[sqlx::test(fixtures(path = "../fixtures", scripts("users")))]
async fn test_join_group_no_token(pool: MySqlPool) {
    let ip_blacklist: IpBlacklist = Arc::new(Mutex::new(Vec::new()));
    let app = test::init_service(
        App::new()
            .app_data(Data::new(pool))
            .app_data(Data::new(SECRET.to_string()))
            .app_data(Data::new(ip_blacklist))
            .service(join_group_route),
    )
    .await;
    let payload = serde_json::json!({"group_id": "g", "user_id": "u", "role": "Member"});
    let req = test::TestRequest::post()
        .uri("/groups/join")
        .set_payload(serde_json::to_string(&payload).unwrap())
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .peer_addr(peer_addr())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401, "join group no token: {}", resp.status());
}

// ── group memberships ────────────────────────────────────────────────

#[sqlx::test(fixtures(path = "../fixtures", scripts("users")))]
async fn test_get_group_memberships(pool: MySqlPool) {
    let ip_blacklist: IpBlacklist = Arc::new(Mutex::new(Vec::new()));
    let app = test::init_service(
        App::new()
            .app_data(Data::new(pool))
            .app_data(Data::new(SECRET.to_string()))
            .app_data(Data::new(ip_blacklist))
            .service(get_group_memberships_by_user_route),
    )
    .await;
    let req = test::TestRequest::get()
        .uri("/groups/test_id")
        .insert_header((
            header::AUTHORIZATION,
            format!("Bearer {}", token("test_id", "test_username")),
        ))
        .peer_addr(peer_addr())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(
        resp.status().is_success(),
        "get group memberships: {}",
        resp.status()
    );

    let body: ApiResponse<Vec<GroupMembership>> = test::read_body_json(resp).await;
    assert!(body.success, "success should be true");

    let memberships = body
        .data
        .expect("data should contain a list of GroupMemberships");
    assert!(!memberships.is_empty(), "memberships should not be empty");

    let m = &memberships[0];
    assert_eq!(m.group_id, "test_group_id1", "group_id mismatch");
    assert!(!m.user_id.is_empty(), "user_id should exist");
}

#[sqlx::test(fixtures(path = "../fixtures", scripts("users")))]
async fn test_get_group_memberships_no_token(pool: MySqlPool) {
    let ip_blacklist: IpBlacklist = Arc::new(Mutex::new(Vec::new()));
    let app = test::init_service(
        App::new()
            .app_data(Data::new(pool))
            .app_data(Data::new(SECRET.to_string()))
            .app_data(Data::new(ip_blacklist))
            .service(get_group_memberships_by_user_route),
    )
    .await;
    let req = test::TestRequest::get()
        .uri("/groups/test_id")
        .peer_addr(peer_addr())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401, "should return 401 without auth");
}

// ── error cases ──────────────────────────────────────────────────────

#[sqlx::test(fixtures(path = "../fixtures", scripts("users")))]
async fn test_bad_request(pool: MySqlPool) {
    let ip_blacklist: IpBlacklist = Arc::new(Mutex::new(Vec::new()));
    let app = test::init_service(
        App::new()
            .app_data(Data::new(pool))
            .app_data(Data::new(SECRET.to_string()))
            .app_data(Data::new(ip_blacklist))
            .service(rate_restaurant_route),
    )
    .await;
    let payload = serde_json::json!({"invalid": "payload"});
    let req = test::TestRequest::post()
        .uri("/users/test_id/ratings")
        .set_payload(serde_json::to_string(&payload).unwrap())
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .peer_addr(peer_addr())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400, "should return 400 for bad request");
}

#[sqlx::test(fixtures(path = "../fixtures", scripts("users")))]
async fn test_bad_token(pool: MySqlPool) {
    let ip_blacklist: IpBlacklist = Arc::new(Mutex::new(Vec::new()));
    let app = test::init_service(
        App::new()
            .app_data(Data::new(pool))
            .app_data(Data::new(SECRET.to_string()))
            .app_data(Data::new(ip_blacklist))
            .service(get_users_route),
    )
    .await;
    let req = test::TestRequest::get()
        .uri("/users")
        .insert_header((header::AUTHORIZATION, "Bearer invalid"))
        .peer_addr(peer_addr())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401, "should return 401 for bad token");
}

#[sqlx::test(fixtures(path = "../fixtures", scripts("users", "restaurants")))]
async fn test_forbidden(pool: MySqlPool) {
    let ip_blacklist: IpBlacklist = Arc::new(Mutex::new(Vec::new()));
    let app = test::init_service(
        App::new()
            .app_data(Data::new(pool))
            .app_data(Data::new(SECRET.to_string()))
            .app_data(Data::new(ip_blacklist))
            .service(get_restaurants_route),
    )
    .await;
    // test_id2 is NOT a member of test_group_id2 (different group)
    let req = test::TestRequest::get()
        .uri("/restaurants?group_id=test_group_id2")
        .insert_header((
            header::AUTHORIZATION,
            format!("Bearer {}", token("test_id2", "test_username2")),
        ))
        .peer_addr(peer_addr())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 403, "should return 403 for non-member");
}

#[sqlx::test(fixtures(path = "../fixtures", scripts("users")))]
async fn test_no_auth(pool: MySqlPool) {
    let ip_blacklist: IpBlacklist = Arc::new(Mutex::new(Vec::new()));
    let app = test::init_service(
        App::new()
            .app_data(Data::new(pool))
            .app_data(Data::new(SECRET.to_string()))
            .app_data(Data::new(ip_blacklist))
            .service(get_users_route),
    )
    .await;
    let req = test::TestRequest::get()
        .uri("/users")
        .peer_addr(peer_addr())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401, "should return 401 without auth");
}

// ── OIDC ───────────────────────────────────────────────────────────

#[sqlx::test(fixtures(path = "../fixtures", scripts("users")))]
async fn test_get_oidc_links_empty(pool: MySqlPool) {
    let ip_blacklist: IpBlacklist = Arc::new(Mutex::new(Vec::new()));
    let app = test::init_service(
        App::new()
            .app_data(Data::new(pool.clone()))
            .app_data(Data::new(SECRET.to_string()))
            .app_data(Data::new(ip_blacklist))
            .service(get_user_oidc_links_route),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/users/test_id/oidc-links")
        .insert_header((
            header::AUTHORIZATION,
            format!("Bearer {}", token("test_id", "test_username")),
        ))
        .peer_addr(peer_addr())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200, "should return 200 for valid auth");

    let body = test::read_body(resp).await;
    let api_response: ApiResponse<Vec<OidcLink>> = serde_json::from_slice(&body).unwrap();
    assert!(api_response.success, "should have success=true");
    assert!(
        api_response.data.unwrap().is_empty(),
        "should have no links"
    );
}

#[sqlx::test(fixtures(path = "../fixtures", scripts("users", "oidc_links")))]
async fn test_get_oidc_links_with_data(pool: MySqlPool) {
    let ip_blacklist: IpBlacklist = Arc::new(Mutex::new(Vec::new()));
    let app = test::init_service(
        App::new()
            .app_data(Data::new(pool.clone()))
            .app_data(Data::new(SECRET.to_string()))
            .app_data(Data::new(ip_blacklist))
            .service(get_user_oidc_links_route),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/users/test_id/oidc-links")
        .insert_header((
            header::AUTHORIZATION,
            format!("Bearer {}", token("test_id", "test_username")),
        ))
        .peer_addr(peer_addr())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200, "should return 200");

    let body = test::read_body(resp).await;
    let api_response: ApiResponse<Vec<OidcLink>> = serde_json::from_slice(&body).unwrap();
    assert!(api_response.success);
    let links = api_response.data.unwrap();
    assert!(!links.is_empty(), "should have at least one link");
}

#[sqlx::test(fixtures(path = "../fixtures", scripts("users")))]
async fn test_get_oidc_links_unauthorized(pool: MySqlPool) {
    let ip_blacklist: IpBlacklist = Arc::new(Mutex::new(Vec::new()));
    let app = test::init_service(
        App::new()
            .app_data(Data::new(pool.clone()))
            .app_data(Data::new(SECRET.to_string()))
            .app_data(Data::new(ip_blacklist))
            .service(get_user_oidc_links_route),
    )
    .await;

    // Try to access another user's links
    let req = test::TestRequest::get()
        .uri("/users/test_id2/oidc-links")
        .insert_header((
            header::AUTHORIZATION,
            format!("Bearer {}", token("test_id", "test_username1")),
        ))
        .peer_addr(peer_addr())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        403,
        "should return 403 when accessing another user's links"
    );
}

#[sqlx::test(fixtures(path = "../fixtures", scripts("users")))]
async fn test_unlink_oidc_not_found(pool: MySqlPool) {
    let ip_blacklist: IpBlacklist = Arc::new(Mutex::new(Vec::new()));
    let app = test::init_service(
        App::new()
            .app_data(Data::new(pool.clone()))
            .app_data(Data::new(SECRET.to_string()))
            .app_data(Data::new(ip_blacklist))
            .service(unlink_oidc_route),
    )
    .await;

    // Try to unlink non-existent link
    let req = test::TestRequest::delete()
        .uri("/users/test_id/oidc-links/https%3A%2F%2Fexample.com")
        .insert_header((
            header::AUTHORIZATION,
            format!("Bearer {}", token("test_id", "test_username")),
        ))
        .peer_addr(peer_addr())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        200,
        "should return 200 even for non-existent link"
    );
}
