import axios from 'axios';
import { PUSH_SUBSCRIBE_ENDPOINT, VAPID_PUBLIC_KEY } from './endpoints';
import type { NewPushSubscription, User } from './models';

export const NOTIFICATION_TOAST_DISMISSED = 'notificationToastDismissed';

export async function setupNotifications(user: User, token: string) {
  await navigator.serviceWorker
    .getRegistration()
    .then((registration) => {
      if (registration) {
        return registration.pushManager.getSubscription().then(async (subscription) => {
          if (subscription) {
            return subscription;
          }

          if (VAPID_PUBLIC_KEY) {
            return registration.pushManager.subscribe({
              userVisibleOnly: true,
              applicationServerKey: VAPID_PUBLIC_KEY
            });
          }
        });
      }
    })
    .then(async (subscription) => {
      if (subscription) {
        await axios.post(
          PUSH_SUBSCRIBE_ENDPOINT,
          {
            user_id: user.id,
            subscription_info: subscription
          } as NewPushSubscription,
          {
            headers: {
              'Content-Type': 'application/json',
              Authorization: 'Bearer ' + token
            }
          }
        );
      }
    });
}
