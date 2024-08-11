<script lang="ts">
  // Most of your app wide CSS should be put in this file
  import '../app.postcss';
  import {
    AppShell,
    AppBar,
    Drawer,
    getDrawerStore,
    type DrawerSettings,
    initializeStores,
    storePopup,
    type PopupSettings,
    popup,
    Toast
  } from '@skeletonlabs/skeleton';
  import { computePosition, autoUpdate, offset, shift, flip, arrow } from '@floating-ui/dom';
  import { user } from '$lib/store';
  import { deleteCookies, getUserFromToken, readTokenCookie } from '$lib/auth';
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import Navigation from '$lib/navigation.svelte';

  storePopup.set({ computePosition, autoUpdate, offset, shift, flip, arrow });
  initializeStores();

  onMount(() => {
    let token = readTokenCookie();
    if (token) {
      const userFromToken = getUserFromToken(token);
      if (userFromToken) {
        $user = userFromToken;
      } else {
        // Token is invalid or expired, handle accordingly (e.g., clear user)
        $user = null;
      }
    } else {
      // No token found, user is not logged in
      $user = null;
    }
  });

  function logout() {
    $user = null;
    deleteCookies();
    goto('/');
  }

  const drawerStore = getDrawerStore();
  function drawerOpen(): void {
    const drawerSettings: DrawerSettings = {
      bgBackdrop: 'bg-gradient-to-tr from-blue-500/50 via-purple-500/50 to-blue-500/50',
      width: 'w-[140px] md:w-[480px]',
      rounded: 'rounded-xl'
    };

    drawerStore.open(drawerSettings);
  }

  const userPopup: PopupSettings = {
    event: 'click',
    target: 'userPopup',
    placement: 'bottom'
  };
</script>

<Drawer>
  <div class="flex flex-col items-center">
    <a class="p-4 my-4" href="/">
      <strong class="text-xl uppercase">Ratings</strong>
    </a>
    <hr />
    <Navigation />
  </div>
</Drawer>

<Toast />

<!-- App Shell -->
<AppShell slotSidebarLeft="bg-surface-500/5 w-0 lg:w-32">
  <svelte:fragment slot="header">
    <!-- App Bar -->
    <AppBar>
      <svelte:fragment slot="lead">
        <div class="flex items-center">
          <button class="lg:hidden btn btn-sm mr-4" on:click={drawerOpen}>
            <span>
              <svg viewBox="0 0 100 80" class="fill-token w-4 h-4">
                <rect width="100" height="20" />
                <rect y="30" width="100" height="20" />
                <rect y="60" width="100" height="20" />
              </svg>
            </span>
          </button>
          <a href="/">
            <strong class="text-xl uppercase">Ratings</strong>
          </a>
        </div>
      </svelte:fragment>
      <svelte:fragment slot="trail">
        {#if $user && $user.token.length > 0}
          <p class="badge btn-sm bg-violet-500">{$user.groupMembership?.group.name || '___'}</p>
          <div class="relative inline-block">
            <button
              class="badge btn-sm variant-filled-secondary cursor-pointer"
              use:popup={userPopup}
            >
              {$user.username}
            </button>
            <div class="card p-4 shadow-xl" data-popup="userPopup">
              <div class="flex flex-col items-center">
                <a class="btn btn-sm variant-filled-secondary w-full text-center" href="/profile">
                  Profile
                </a>
                <br />
                <button
                  class="btn btn-sm variant-filled-primary w-full text-center"
                  on:click={logout}
                >
                  Logout
                </button>
                <div class="arrow variant-filled-surface" />
              </div>
            </div>
          </div>
        {/if}
      </svelte:fragment>
    </AppBar>
  </svelte:fragment>

  <svelte:fragment slot="sidebarLeft">
    <Navigation />
  </svelte:fragment>
  <!-- Page Route Content -->
  <slot />
</AppShell>
