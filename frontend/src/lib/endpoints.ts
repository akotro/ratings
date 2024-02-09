import { env } from '$env/dynamic/public';

export const API_BASE_URL = env.PUBLIC_API_BASE_URL;

export const LOGIN_ENDPOINT = `${API_BASE_URL}/auth/login`;
export const RESTAURANTS_ENDPOINT = `${API_BASE_URL}/restaurants`;
export const RESTAURANTS_WITH_AVG_RATING_ENDPOINT = `${API_BASE_URL}/restaurants_with_avg_rating`;
export function RESTAURANT_RATING_COMPLETE_ENDPOINT(restaurantId: string) {
  return `${API_BASE_URL}/restaurants/${restaurantId}/is_rating_complete`;
}
export function GET_RESTAURANT_RATINGS_ENDPOINT(restaurantId: string) {
  return `${API_BASE_URL}/restaurants/${restaurantId}/ratings`;
}
export function GET_RATING_ENDPOINT(userId: string, restaurantId: string) {
  return `${API_BASE_URL}/users/${userId}/ratings/${restaurantId}`;
}
export function GET_RATINGS_ENDPOINT(userId: string) {
  return `${API_BASE_URL}/users/${userId}/ratings`;
}
export function RATE_ENDPOINT(userId: string) {
  return `${API_BASE_URL}/users/${userId}/ratings`;
}
