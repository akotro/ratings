import { jwtDecode } from 'jwt-decode';
import type { GroupMembership, User } from './models';
import { NOTIFICATION_TOAST_DISMISSED } from './notifications';
import { COOKIE_DOMAIN } from './endpoints';

const domainAttr = COOKIE_DOMAIN ? `; domain=${COOKIE_DOMAIN}` : '';

function getCookieValue(name: string): string | null {
  const match = document.cookie.match(new RegExp('(^| )' + name + '=([^;]+)'));
  if (match) return decodeURIComponent(match[2]);
  return null;
}

function eraseCookie(name: string) {
  document.cookie = `${name}=; path=/${domainAttr}; expires=Thu, 01 Jan 1970 00:00:00 GMT;`;
  if (domainAttr) {
    document.cookie = `${name}=; path=/; expires=Thu, 01 Jan 1970 00:00:00 GMT;`;
  }
}

export function setUserCookies(token: string, color: string) {
  setTokenCookie(token);
  setColorCookie(color);
}

export function setTokenCookie(value: string) {
  document.cookie = `token=${encodeURIComponent(value)}; path=/${domainAttr}`;
}

export function setColorCookie(color: string) {
  document.cookie = `color=${encodeURIComponent(color)}; path=/${domainAttr}`;
}

export function setGroupCookie(group: GroupMembership) {
  const groupString = JSON.stringify(group);
  document.cookie = `group=${encodeURIComponent(groupString)}; path=/${domainAttr}`;
}

export function readTokenCookie(): string | null {
  return getCookieValue('token');
}

export function readColorCookie(): string {
  return getCookieValue('color') || '';
}

export function readGroupCookie(): GroupMembership | null {
  const groupString = getCookieValue('group');
  if (!groupString) return null;

  try {
    return JSON.parse(groupString) as GroupMembership;
  } catch (e) {
    console.error('Error parsing group cookie:', e);
    return null;
  }
}

export function deleteTokenCookie() {
  eraseCookie('token');
}

export function deleteColorCookie() {
  eraseCookie('color');
}

export function deleteGroupCookie() {
  eraseCookie('group');
}

export function deleteCookies() {
  deleteTokenCookie();
  deleteColorCookie();
  deleteGroupCookie();
  localStorage.setItem(NOTIFICATION_TOAST_DISMISSED, 'false');
}

export function getUserFromToken(token: string): User | null {
  try {
    const decodedToken = jwtDecode(token) as { id: string; username: string; exp: number };

    if (decodedToken && decodedToken.exp > Date.now() / 1000) {
      const userId: string = decodedToken.id;
      const username: string = decodedToken.username;
      const color: string = readColorCookie();
      const groupMembership: GroupMembership | null = readGroupCookie();

      return {
        id: userId,
        username: username,
        password: '',
        color: color,
        token: token,
        groupMembership: groupMembership
      } as User;
    }
  } catch (e) {
    console.error('Failed to decode token:', e);
  }

  deleteCookies();
  return null;
}
