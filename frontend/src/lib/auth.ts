import { jwtDecode } from 'jwt-decode';

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

export function getUserFromToken(token: string) {
  const decodedToken = jwtDecode(token) as { id: string; username: number; exp: number };
  if (decodedToken && decodedToken.exp > Date.now() / 1000) {
    // console.log('token has not expired');
    const userId: string = decodedToken.id;
    const username: string = decodedToken.username.toLocaleString();

    return { id: userId, username: username, password: '', token: token };
  } else {
    // console.log('token has expired');
    deleteTokenCookie();
    return null;
  }
}
