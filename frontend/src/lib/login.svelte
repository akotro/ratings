<script lang="ts">
  import { createEventDispatcher, onMount } from 'svelte';
  import { user } from './store';
  import { LOGIN_ENDPOINT, REGISTER_ENDPOINT } from './endpoints';
  import Loading from './loading.svelte';
  import axios from 'axios';
  import { setTokenCookie, readTokenCookie, deleteCookies } from './auth';
  import Groups from './groups.svelte';
  import ColorPicker from 'svelte-awesome-color-picker';

  export let showRegister = true;
  export let loginRedirectUrl: string | null = null;

  let username = '';
  let password = '';
  let confirmPassword = '';
  let hex = '';
  let loginLoading = false;
  let loginFailed = false;
  let fieldValidationError = false;
  let passwordsMatchError = false;
  let registration = false;

  // TODO: verify token with backend
  let checkingAuth = true;
  onMount(() => {
    registration = false;
    hex = '';
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
    fieldValidationError = false;
    passwordsMatchError = false;

    if (!username || !password || (registration && (!confirmPassword || !hex))) {
      fieldValidationError = true;
      return;
    }

    if (registration) {
      if (password !== confirmPassword) {
        passwordsMatchError = true;
        return;
      }

      try {
        loginLoading = true;
        const response = await axios.post(REGISTER_ENDPOINT, {
          id: '',
          username,
          password,
          color: hex
        });
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
        const response = await axios.post(LOGIN_ENDPOINT, {
          id: '',
          username,
          password,
          color: ''
        });
        const data = response.data;
        if (data.success && data.data) {
          $user = data.data;
          dispatch('login', data.data);
          setTokenCookie(data.data.token);
          username = '';
          password = '';

          if (loginRedirectUrl) {
            window.location.href = loginRedirectUrl;
          }
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
    deleteCookies();
  }

  function toggleRegistration() {
    registration = !registration;
    fieldValidationError = false;
    passwordsMatchError = false;
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
      {#if $user.groupMembership}
        <a class="btn btn-lg variant-ghost-surface my-2 w-full" href="/groups"> Groups </a>
        <a class="btn btn-lg variant-ghost-surface my-2 w-full" href="/restaurants">
          Restaurants
        </a>
        <a class="btn btn-lg variant-ghost-surface my-2 w-full" href="/ratings"> Ratings </a>
      {:else}
        <p class="py-2 h4 text-white text-center">Select a group!</p>
        <Groups />
      {/if}

      <br />
      <br />
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
      <br />
      <label class="label flex items-center space-x-2">
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
          <ColorPicker bind:hex isAlpha={false} position="responsive" />
        </div>
        <input class="input flex-1" type="text" bind:value={hex} readonly />
      </label>
    {/if}
    {#if fieldValidationError}
      <p class="text-red-500">All fields are required. Please fill them out.</p>
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
    {#if showRegister}
      <a href="./" on:click={toggleRegistration} class="cursor-pointer text-blue-500 mt-4">
        {#if registration}
          Already have an account? Login
        {:else}
          Don't have an account? Register
        {/if}
      </a>
    {/if}
  </form>
{/if}
