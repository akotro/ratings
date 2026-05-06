<script lang="ts">
  import { browser } from '$app/environment';
  import { goto } from '$app/navigation';
  import { readTokenCookie, getUserFromToken } from '$lib/auth';
  import Loading from '$lib/loading.svelte';
  import Login from '$lib/login.svelte';
  import { user } from '$lib/store';
  import { onMount } from 'svelte';

  let checkingAuth = true;
  let loginRedirectUrl: string | null = null;

  onMount(async () => {
    const token = readTokenCookie();
    if (token) {
      const userData = getUserFromToken(token);
      if (userData) {
        $user = userData;
      }
    }

    const urlParams = new URLSearchParams(window.location.search);

    if (urlParams.has('redirect')) {
      const redirectUrlToSet = urlParams.get('redirect') || '';
      if (redirectUrlToSet.startsWith('/')) {
        loginRedirectUrl = redirectUrlToSet;
      }
    }

    const error = urlParams.get('error');
    if (error) {
      console.error('OIDC Error:', error);
      window.history.replaceState({}, document.title, window.location.pathname);
      checkingAuth = false;
      return;
    }

    if (urlParams.get('oidc_success') === 'true') {
      window.history.replaceState({}, document.title, window.location.pathname);
    }

    checkingAuth = false;
  });

  $: if (browser && $user && !checkingAuth) {
    goto(loginRedirectUrl || '/');
  }
</script>

<div class="flex flex-col items-center justify-center">
  {#if checkingAuth}
    <div class="animate-pulse">Verifying authentication...</div>
    <Loading />
  {:else}
    <div class="space-y-10 text-center flex flex-col items-center">
      <h1 class="text-center text-6xl my-4">Login</h1>
      <Login showRegister={false} {loginRedirectUrl} />
    </div>
  {/if}
</div>
