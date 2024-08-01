import { env } from '$env/dynamic/public';

export const API_BASE_URL = env.PUBLIC_API_BASE_URL;

export const LOGIN_ENDPOINT = `${API_BASE_URL}/auth/login`;
export const REGISTER_ENDPOINT = `${API_BASE_URL}/auth/register`;

export function GET_GROUP_MEMBERSHIPS_ENDPOINT(userId: string) {
  return `${API_BASE_URL}/groups/${userId}`;
}
export const CREATE_GROUP_ENDPOINT = `${API_BASE_URL}/groups`;
export const JOIN_GROUP_ENDPOINT = `${API_BASE_URL}/groups/join`;
export function SHARE_GROUP_ENDPOINT(groupId: string) {
  return `${window.location.origin}/groups/join/${groupId}`;
}

export const RESTAURANTS_ENDPOINT = `${API_BASE_URL}/restaurants`;
export function RESTAURANTS_WITH_AVG_RATING_ENDPOINT(groupId: string) {
  return `${API_BASE_URL}/restaurants_with_avg_rating?group_id=${groupId}`;
}
export function RESTAURANT_RATING_COMPLETE_ENDPOINT(restaurantId: string, groupId: string) {
  return `${API_BASE_URL}/restaurants/${restaurantId}/is_rating_complete?group_id=${groupId}`;
}
export function GET_RESTAURANT_RATINGS_ENDPOINT(restaurantId: string, groupId: string) {
  return `${API_BASE_URL}/restaurants/${restaurantId}/ratings?group_id=${groupId}`;
}

export function GET_RATING_ENDPOINT(userId: string, restaurantId: string, groupId: string) {
  return `${API_BASE_URL}/users/${userId}/ratings/${restaurantId}?group_id=${groupId}`;
}
export function GET_RATINGS_ENDPOINT(userId: string, groupId: string) {
  return `${API_BASE_URL}/users/${userId}/ratings?group_id=${groupId}`;
}
export function RATE_ENDPOINT(userId: string) {
  return `${API_BASE_URL}/users/${userId}/ratings`;
}
