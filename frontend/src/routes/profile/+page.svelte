<script lang="ts">
  import { goto } from '$app/navigation';
  import { page } from '$app/stores';
  import { deleteCookies, getUserFromToken, readTokenCookie } from '$lib/auth';
  import {
    UPDATE_USER_ENDPOINT,
    OIDC_LOGIN_ENDPOINT,
    OIDC_LINK_ENDPOINT,
    GET_USER_OIDC_LINKS_ENDPOINT,
    UNLINK_OIDC_ENDPOINT,
    OIDC_PROVIDER_NAME,
    OIDC_PROVIDER_ICON_URL
  } from '$lib/endpoints';
  import Loading from '$lib/loading.svelte';
  import { NOTIFICATION_TOAST_DISMISSED, setupNotifications } from '$lib/notifications';
  import { user } from '$lib/store';
  import axios from 'axios';
  import { onMount } from 'svelte';
  import ColorPicker from 'svelte-awesome-color-picker';

  let newUsername = '';
  let hex = '';

  let updateLoading = false;
  let updateFailed = false;
  let checkingAuth = true;

  let notificationDismissed: string | null = null;
  let showNotificationSettings = false;
  let oidcProvider = '';
  let showOidcLinkSection = false;
  let oidcLinks: { provider: string; subject: string; created_at?: string }[] = [];

  onMount(async () => {
    const token = readTokenCookie();
    if (token) {
      checkingAuth = false;

      const currentUser = getUserFromToken(token);
      if (currentUser) {
        $user = currentUser;
      }

      user.subscribe((value) => {
        if (value) {
          newUsername = value.username;
          hex = value.color;
        }
      });

      notificationDismissed = localStorage.getItem(NOTIFICATION_TOAST_DISMISSED);
      if (notificationDismissed === 'true' && window.Notification.permission !== 'granted') {
        showNotificationSettings = true;
      } else {
        showNotificationSettings = false;
      }

      if (currentUser) {
        try {
          const response = await axios.get(GET_USER_OIDC_LINKS_ENDPOINT(currentUser.id), {
            headers: { Authorization: `Bearer ${currentUser.token}` }
          });
          if (response.data.success) {
            oidcLinks = response.data.data || [];
          }
        } catch (err) {
          console.error('Failed to fetch OIDC links');
        }
      }

      const urlParams = new URLSearchParams(window.location.search);
      if (urlParams.get('oidc_pending') === 'true') {
        const urlProvider = urlParams.get('provider');
        const urlSubject = urlParams.get('subject');

        if (urlProvider && urlSubject) {
          sessionStorage.setItem('oidc_pending_link_provider', urlProvider);
          sessionStorage.setItem('oidc_pending_link_subject', urlSubject);

          window.history.replaceState({}, document.title, window.location.pathname);
        }
      }

      const pendingProvider = sessionStorage.getItem('oidc_pending_link_provider');
      const pendingSubject = sessionStorage.getItem('oidc_pending_link_subject');

      if (pendingProvider && pendingSubject) {
        const isAlreadyLinked = oidcLinks.some((link) => link.provider === pendingProvider);
        if (isAlreadyLinked) {
          sessionStorage.removeItem('oidc_pending_link_provider');
          sessionStorage.removeItem('oidc_pending_link_subject');
          showOidcLinkSection = false;
        } else {
          showOidcLinkSection = true;
          oidcProvider = pendingProvider;
        }
      }
    } else {
      checkingAuth = false;
      showNotificationSettings = false;
    }
  });

  function resetUsername() {
    if ($user) {
      newUsername = $user.username;
    }
  }

  function resetColor() {
    if ($user) {
      hex = $user.color;
    }
  }

  async function updateUser() {
    updateLoading = true;
    updateFailed = false;

    try {
      if ($user && $user.token.length > 0) {
        const response = await axios.put(
          UPDATE_USER_ENDPOINT($user.id),
          {
            id: $user.id,
            username: newUsername,
            password: '',
            color: hex
          },
          {
            headers: {
              'Content-Type': 'application/json',
              Authorization: 'Bearer ' + $user.token
            }
          }
        );
        const data = response.data;
        if (data && data.success) {
          logout();
        } else {
          updateFailed = true;
        }
      }
    } catch (error) {
      updateFailed = true;
    }

    updateLoading = false;
  }

  function logout() {
    $user = null;
    deleteCookies();
    goto('/');
  }

  function startOidcLogin() {
    window.location.href = OIDC_LOGIN_ENDPOINT;
  }

  async function linkOidcAccount() {
    const provider = sessionStorage.getItem('oidc_pending_link_provider');
    const subject = sessionStorage.getItem('oidc_pending_link_subject');

    if (!provider || !subject || !$user) return;

    try {
      const response = await axios.post(
        OIDC_LINK_ENDPOINT,
        { provider, subject },
        {
          headers: { Authorization: `Bearer ${$user.token}` }
        }
      );

      if (response.data.success) {
        showOidcLinkSection = false;
        sessionStorage.removeItem('oidc_pending_link_provider');
        sessionStorage.removeItem('oidc_pending_link_subject');
        oidcLinks = [...oidcLinks, { provider, subject }];
      }
    } catch (err) {
      console.error('Failed to link OIDC account', err);
    }
  }

  function cancelOidcLink() {
    showOidcLinkSection = false;
    sessionStorage.removeItem('oidc_pending_link_provider');
    sessionStorage.removeItem('oidc_pending_link_subject');
  }

  async function unlinkOidc(provider: string) {
    if (!$user) return;
    try {
      await axios.delete(UNLINK_OIDC_ENDPOINT($user.id), {
        headers: { Authorization: `Bearer ${$user.token}` }
      });

      oidcLinks = oidcLinks.filter((l) => l.provider !== provider);

      if (sessionStorage.getItem('oidc_pending_link_provider') === provider) {
        sessionStorage.removeItem('oidc_pending_link_provider');
        sessionStorage.removeItem('oidc_pending_link_subject');
        showOidcLinkSection = false;
      }
    } catch (err) {
      console.error('Failed to unlink OIDC account');
    }
  }
</script>

<div class="flex flex-col items-center justify-center">
  <h1 class="text-center text-6xl my-4">Profile</h1>
  {#if checkingAuth}
    <div class="animate-pulse">Verifying authentication...</div>
    <Loading />
  {:else if $user && $user.token.length > 0}
    <div class="flex flex-col items-center justify-center px-4 gap-2">
      <div class="card p-2 w-full max-w-md">
        <label for="username" class="label block text-center">
          <span class="text-xl">Username</span>
          <div class="input-group input-group-divider grid-cols-[auto_1fr_auto]">
            <input
              id="username"
              class="input w-full"
              title="Input (text)"
              type="text"
              placeholder="input text"
              bind:value={newUsername}
            />
            <button class="variant-filled-error" on:click={resetUsername}>Reset</button>
          </div>
        </label>
      </div>

      <br />

      <div class="card p-2 w-full max-w-md">
        <label for="color" class="label block text-center">
          <span class="text-xl">Color</span>
          <div class="flex items-center space-x-2">
            <style>
              .darkColorPicker {
                --cp-bg-color: #4e3c8b;
                --cp-border-color: #15171f;
                --cp-text-color: #dfe0e2;
                --cp-input-color: #212432;
                --cp-button-hover-color: #8a95ca;
              }
            </style>
            <div class="rounded-full variant-outline flex-1 darkColorPicker">
              <ColorPicker bind:hex isAlpha={false} position="responsive" label="" />
            </div>
            <div class="input-group input-group-divider grid-cols-[auto_1fr_auto] w-full">
              <input id="color" class="input w-full" type="text" bind:value={hex} readonly />
              <button class="variant-filled-error" on:click={resetColor}>Reset</button>
            </div>
          </div>
        </label>
      </div>

      <br />

      <div class="card p-4 w-full max-w-md">
        <h3 class="text-xl mb-4 text-center">Connected Accounts</h3>

        {#if oidcLinks.length > 0}
          <div class="mb-4 space-y-2">
            {#each oidcLinks as link}
              <div class="flex items-center justify-between p-3 variant-ghost rounded">
                <div class="flex items-center">
                  {#if OIDC_PROVIDER_ICON_URL}
                    <img
                      class="mr-2"
                      width="28"
                      height="28"
                      src={OIDC_PROVIDER_ICON_URL}
                      alt={OIDC_PROVIDER_NAME}
                    />
                  {/if}
                  <span class="text-sm font-medium"> {link.provider}</span>
                </div>
                <button
                  class="btn btn-sm variant-filled-error"
                  on:click={() => unlinkOidc(link.provider)}
                >
                  Unlink
                </button>
              </div>
            {/each}
          </div>
        {:else if !showOidcLinkSection}
          <p class="text-sm text-gray-400 mb-4 text-center">No external accounts linked yet.</p>
        {/if}

        {#if showOidcLinkSection}
          <hr class="opacity-50 my-4" />
          <div class="variant-soft-warning p-4 rounded text-center">
            <div class="flex items-center text-sm mb-3">
              <span>Link pending for:</span>
              {#if OIDC_PROVIDER_ICON_URL}
                <img
                  class="mx-2"
                  width="28"
                  height="28"
                  src={OIDC_PROVIDER_ICON_URL}
                  alt={OIDC_PROVIDER_NAME}
                />
              {/if}
              <span><strong>{oidcProvider}</strong></span>
            </div>
            <div class="flex justify-center gap-2">
              <button class="btn btn-sm variant-filled-primary" on:click={linkOidcAccount}>
                Confirm Link
              </button>
              <button class="btn btn-sm variant-ghost" on:click={cancelOidcLink}> Cancel </button>
            </div>
          </div>
        {:else if oidcLinks.length <= 0}
          <button class="btn variant-filled-secondary w-full mt-2" on:click={startOidcLogin}>
            Link
            {#if OIDC_PROVIDER_ICON_URL}
              <img
                class="mx-2"
                width="28"
                height="28"
                src={OIDC_PROVIDER_ICON_URL}
                alt={OIDC_PROVIDER_NAME}
              />
              {OIDC_PROVIDER_NAME} Account
            {/if}
          </button>
        {/if}
      </div>

      <br />

      {#if updateLoading}
        <button on:click={updateUser} class="btn btn-lg variant-filled-secondary">
          <Loading />
          Update Profile
        </button>
      {:else}
        <button on:click={updateUser} class="btn btn-lg variant-filled-secondary">
          Update Profile
        </button>
      {/if}
      {#if updateFailed}
        <p class="text-red-500">Operation failed. Please try again.</p>
      {/if}

      <br />

      {#if showNotificationSettings}
        <h2 class="text-center text-4xl my-4">Notification Settings</h2>

        <div class="card p-2 w-full max-w-md">
          <label class="label block text-center">
            <div class="grid-cols-[auto_1fr_auto]">
              <button
                class="btn variant-filled-surface"
                on:click={async () => {
                  if ($user && $user.token.length > 0) {
                    const status = await window.Notification.requestPermission();
                    if (status === 'granted') {
                      await setupNotifications($user, $user.token);
                    }
                  }
                }}>Allow Notifications</button
              >
            </div>
          </label>
        </div>
      {/if}
    </div>
  {:else if $user == null || $user.token == null}
    <h1 class="p-6 text-8xl text-white text-center">
      Please <a
        href={$page ? `/login?redirect=${$page.route.id}` : '/'}
        class="hover:underline dark:text-blue-500">Login</a
      >
    </h1>
  {/if}
</div>
