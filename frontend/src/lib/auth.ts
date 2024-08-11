import { jwtDecode } from 'jwt-decode';
import type { GroupMembership, User } from './models';

export function setUserCookies(token: string, color: string) {
  setTokenCookie(token);
  setColorCookie(color);
}

export function setTokenCookie(value: string) {
  document.cookie = `token=${value}; path=/`;
}

export function readTokenCookie(): string | null {
  const name = 'token=';
  const decodedCookie = decodeURIComponent(document.cookie);
  const cookieArray = decodedCookie.split(';');
  for (let i = 0; i < cookieArray.length; i++) {
    let cookie = cookieArray[i];
    while (cookie.charAt(0) === ' ') {
      cookie = cookie.substring(1);
    }
    if (cookie.indexOf(name) === 0) {
      return cookie.substring(name.length, cookie.length);
    }
  }
  return null;
}

export function deleteTokenCookie() {
  document.cookie = 'token=; path=/;';
}

export function setColorCookie(color: string) {
  document.cookie = `color=${color}; path=/`;
}

export function readColorCookie(): string {
  const name = 'color=';
  const decodedCookie = decodeURIComponent(document.cookie);
  const cookieArray = decodedCookie.split(';');
  for (let i = 0; i < cookieArray.length; i++) {
    let cookie = cookieArray[i];
    while (cookie.charAt(0) === ' ') {
      cookie = cookie.substring(1);
    }
    if (cookie.indexOf(name) === 0) {
      const colorString = cookie.substring(name.length, cookie.length);
      if (!colorString) {
        return '';
      }

      return colorString;
    }
  }

  return '';
}

export function deleteColorCookie() {
  document.cookie = 'color=; path=/;';
}

export function setGroupCookie(group: GroupMembership) {
  const groupString = JSON.stringify(group);
  document.cookie = `group=${groupString}; path=/`;
}

export function readGroupCookie(): GroupMembership | null {
  const name = 'group=';
  const decodedCookie = decodeURIComponent(document.cookie);
  const cookieArray = decodedCookie.split(';');
  for (let i = 0; i < cookieArray.length; i++) {
    let cookie = cookieArray[i];
    while (cookie.charAt(0) === ' ') {
      cookie = cookie.substring(1);
    }
    if (cookie.indexOf(name) === 0) {
      const groupString = cookie.substring(name.length, cookie.length);
      if (!groupString) {
        return null;
      }

      try {
        return JSON.parse(groupString) as GroupMembership;
      } catch (e) {
        console.error('Error parsing group cookie:', e);
        return null;
      }
    }
  }
  return null;
}

export function deleteGroupCookie() {
  document.cookie = 'group=; path=/;';
}

export function deleteCookies() {
  deleteTokenCookie();
  deleteColorCookie();
  deleteGroupCookie();
}

export function getUserFromToken(token: string) {
  const decodedToken = jwtDecode(token) as { id: string; username: number; exp: number };
  if (decodedToken && decodedToken.exp > Date.now() / 1000) {
    const userId: string = decodedToken.id;
    const username: string = decodedToken.username.toLocaleString();
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
  } else {
    deleteCookies();
    return null;
  }
}
