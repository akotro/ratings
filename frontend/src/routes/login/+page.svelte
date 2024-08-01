<script lang="ts">
  import { goto } from '$app/navigation';
  import { readTokenCookie } from '$lib/auth';
  import Login from '$lib/login.svelte';
  import { user } from '$lib/store';
  import { onMount } from 'svelte';

  let checkingAuth = true;
  let loginRedirectUrl: string | null = null;

  onMount(() => {
    const token = readTokenCookie();
    if (token) {
      checkingAuth = false;
    } else {
      checkingAuth = false;
    }

    const urlParams = new URLSearchParams(window.location.search);
    if (urlParams.has('redirect')) {
      const redirectUrlToSet = urlParams.get('redirect') || '';
      if (redirectUrlToSet.startsWith('/')) {
        loginRedirectUrl = redirectUrlToSet;
      }
    }
  });
</script>

<div class="flex flex-col items-center justify-center">
  {#if checkingAuth}
    <!-- <div class="flex items-center justify-center my-12"> -->
    <!--   <Loading /> -->
    <!-- </div> -->
  {:else if $user && $user.token.length > 0}
    {goto('/')}
  {:else}
    <div class="space-y-10 text-center flex flex-col items-center">
      <h1 class="text-center text-6xl my-4">Login</h1>
      <Login showRegister={false} {loginRedirectUrl} />
    </div>
  {/if}
</div>
