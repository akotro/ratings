<script lang="ts">
  import { createEventDispatcher, onMount } from 'svelte';
  import { user } from './store';
  import { LOGIN_ENDPOINT } from './endpoints';
  import Loading from './loading.svelte';
  import axios from 'axios';
  import { setTokenCookie, deleteTokenCookie, readTokenCookie } from './auth';

  let username = '';
  let password = '';
  let loginLoading = false;
  let loginFailed = false;

  let checkingAuth = true;
  onMount(() => {
    const token = readTokenCookie();
    if (token) {
      checkingAuth = false;
    } else {
      checkingAuth = false;
    }
  });

  const dispatch = createEventDispatcher();

  async function login() {
    loginFailed = false;
    loginLoading = true;
    try {
      const response = await axios.post(LOGIN_ENDPOINT, {
        id: '',
        username,
        password
      });

      const data = response.data;
      if (data.success && data.data) {
        loginFailed = false;
        $user = data.data;
        dispatch('login', data.data);
        setTokenCookie(data.data.token);

        username = '';
        password = '';
      } else {
        loginFailed = true;
      }
    } catch (error) {
      // console.log('Login error: ' + error);
      loginFailed = true;
    }

    loginLoading = false;
  }

  function logout() {
    $user = null;
    deleteTokenCookie();
  }
</script>

{#if checkingAuth}
  <div class="flex items-center justify-center my-12">
    <Loading />
  </div>
{:else if $user && $user.token.length > 0}
  <div class="flex flex-col items-center">
    <h3 class="p-6 h3 text-white text-center">
      Welcome {$user.username}!
    </h3>

    <div class="flex flex-col items-center justify-center">
      <a class="btn btn-lg variant-ghost-surface my-2 w-full" href="/restaurants"> Restaurants </a>
      <a class="btn btn-lg variant-ghost-surface my-2 w-full" href="/restaurants/ratings">
        Ratings
      </a>
      <button class="btn btn-lg variant-filled-primary my-2 w-full" on:click={logout}>Logout</button
      >
    </div>
  </div>
{:else}
  <form on:submit|preventDefault={login}>
    <label class="label">
      <span class="text-xl">Username</span>
      <input class="input" type="text" bind:value={username} />
    </label>
    <label class="label">
      <span class="text-xl">Password</span>
      <input class="input" type="password" bind:value={password} />
    </label>
    <br />
    {#if loginLoading}
      <button class="btn btn-lg variant-filled-surface">
        <Loading size="6" />Loading
      </button>
    {:else}
      <button class="btn btn-lg variant-filled-surface" type="submit"> Login </button>
    {/if}
    {#if loginFailed}
      <p class="text-red-500">Login failed. Please try again.</p>
    {/if}
  </form>
{/if}
