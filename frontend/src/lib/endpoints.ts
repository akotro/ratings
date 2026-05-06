import { env } from '$env/dynamic/public';

export const API_BASE_URL = env.PUBLIC_API_BASE_URL;
export const VAPID_PUBLIC_KEY = env.PUBLIC_VAPID_PUBLIC_KEY;

export const LOGIN_ENDPOINT = `${API_BASE_URL}/auth/login`;
export const REGISTER_ENDPOINT = `${API_BASE_URL}/auth/register`;

export const OIDC_PROVIDER_NAME = env.PUBLIC_OIDC_PROVIDER_NAME;
export const OIDC_PROVIDER_ICON_URL: string | undefined = env.PUBLIC_OIDC_PROVIDER_ICON_URL;
export const OIDC_LOGIN_ENDPOINT = `${API_BASE_URL}/auth/oidc/login`;
export const OIDC_CALLBACK_ENDPOINT = `${API_BASE_URL}/auth/oidc/callback`;
export const OIDC_LINK_ENDPOINT = `${API_BASE_URL}/auth/oidc/link`;

export function UPDATE_USER_ENDPOINT(userId: string) {
  return `${API_BASE_URL}/users/${userId}`;
}

export function GET_GROUP_MEMBERSHIPS_ENDPOINT(userId: string) {
  return `${API_BASE_URL}/groups/${userId}`;
}
export const CREATE_GROUP_ENDPOINT = `${API_BASE_URL}/groups`;
export const JOIN_GROUP_ENDPOINT = `${API_BASE_URL}/groups/join`;
export function SHARE_GROUP_ENDPOINT(groupId: string) {
  return `${window.location.origin}/groups/join/${groupId}`;
}

export function RESTAURANTS_ENDPOINT(groupId: string) {
  return `${API_BASE_URL}/restaurants?group_id=${groupId}`;
}
export function RESTAURANTS_WITH_AVG_RATING_ENDPOINT(groupId: string) {
  return `${API_BASE_URL}/restaurants_with_avg_rating?group_id=${groupId}`;
}
export const CREATE_RESTAURANT_ENDPOINT = `${API_BASE_URL}/restaurants`;
export function UPDATE_RESTAURANT_ENDPOINT(restaurantId: number) {
  return `${API_BASE_URL}/restaurants/${restaurantId}`;
}
export function DELETE_RESTAURANT_ENDPOINT(restaurantId: number) {
  return `${API_BASE_URL}/restaurants/${restaurantId}`;
}
export function RESTAURANT_RATING_COMPLETE_ENDPOINT(restaurantId: number, groupId: string) {
  return `${API_BASE_URL}/restaurants/${restaurantId}/is_rating_complete?group_id=${groupId}`;
}
export function GET_RESTAURANT_RATINGS_ENDPOINT(restaurantId: number, groupId: string) {
  return `${API_BASE_URL}/restaurants/${restaurantId}/ratings?group_id=${groupId}`;
}

export function GET_RATING_ENDPOINT(userId: string, restaurantId: number, groupId: string) {
  return `${API_BASE_URL}/users/${userId}/ratings/${restaurantId}?group_id=${groupId}`;
}
export function GET_RATINGS_ENDPOINT(userId: string, groupId: string) {
  return `${API_BASE_URL}/users/${userId}/ratings?group_id=${groupId}`;
}
export function GET_USER_OIDC_LINKS_ENDPOINT(userId: string) {
  return `${API_BASE_URL}/users/${userId}/oidc-links`;
}
export function UNLINK_OIDC_ENDPOINT(userId: string) {
  return `${API_BASE_URL}/users/${userId}/oidc-links/${encodeURIComponent(OIDC_PROVIDER_NAME)}`;
}
export function RATE_ENDPOINT(userId: string) {
  return `${API_BASE_URL}/users/${userId}/ratings`;
}

export const PUSH_SUBSCRIBE_ENDPOINT = `${API_BASE_URL}/push/subscribe`;
