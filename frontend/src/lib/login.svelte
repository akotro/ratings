<script lang="ts">
  import { createEventDispatcher, onMount } from 'svelte';
  import { user } from './store';
  import { LOGIN_ENDPOINT, REGISTER_ENDPOINT } from './endpoints';
  import Loading from './loading.svelte';
  import axios from 'axios';
  import { setTokenCookie, deleteTokenCookie, readTokenCookie } from './auth';

  let username = '';
  let password = '';
  let confirmPassword = '';
  let loginLoading = false;
  let loginFailed = false;
  let passwordsMatchError = false;
  let registration = false;

  // TODO: verify token with backend
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
    passwordsMatchError = false;

    if (registration) {
      if (password !== confirmPassword) {
        passwordsMatchError = true;
        return;
      }

      try {
        loginLoading = true;
        const response = await axios.post(REGISTER_ENDPOINT, { id: '', username, password });
        const data = response.data;
        if (data.success && data.data) {
          $user = data.data;
          dispatch('login', data.data);
          setTokenCookie(data.data.token);
          username = '';
          password = '';
          confirmPassword = '';
        } else {
          loginFailed = true;
        }
      } catch (error) {
        loginFailed = true;
      }
    } else {
      try {
        loginLoading = true;
        const response = await axios.post(LOGIN_ENDPOINT, { id: '', username, password });
        const data = response.data;
        if (data.success && data.data) {
          $user = data.data;
          dispatch('login', data.data);
          setTokenCookie(data.data.token);
          username = '';
          password = '';
        } else {
          loginFailed = true;
        }
      } catch (error) {
        loginFailed = true;
      }
    }
    loginLoading = false;
  }

  function logout() {
    $user = null;
    deleteTokenCookie();
  }

  function toggleRegistration() {
    registration = !registration;
    passwordsMatchError = false; // Reset this when toggling registration to ensure it starts clean
  }
</script>

{#if checkingAuth}
  <!-- Auth checking UI -->
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
    {#if registration}
      <label class="label">
        <span class="text-xl">Confirm Password</span>
        <input class="input" type="password" bind:value={confirmPassword} />
      </label>
    {/if}
    {#if passwordsMatchError}
      <p class="text-red-500">Passwords do not match. Please try again.</p>
    {/if}
    <br />
    {#if loginLoading}
      <button class="btn btn-lg variant-filled-surface" disabled>
        <Loading />
        {registration ? 'Registering' : 'Loading'}
      </button>
    {:else}
      <button class="btn btn-lg variant-filled-surface" type="submit">
        {registration ? 'Register' : 'Login'}
      </button>
    {/if}
    {#if loginFailed}
      <p class="text-red-500">Operation failed. Please try again.</p>
    {/if}
    <br />
    <br />
    <a href="./" on:click={toggleRegistration} class="cursor-pointer text-blue-500 mt-4">
      {#if registration}
        Already have an account? Login
      {:else}
        Don't have an account? Register
      {/if}
    </a>
  </form>
{/if}
