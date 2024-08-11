<script lang="ts">
  import { goto } from '$app/navigation';
  import { page } from '$app/stores';
  import { deleteCookies, readTokenCookie, setUserCookies } from '$lib/auth';
  import { UPDATE_USER_ENDPOINT } from '$lib/endpoints';
  import Loading from '$lib/loading.svelte';
  import { user } from '$lib/store';
  import axios from 'axios';
  import { onMount } from 'svelte';
  import ColorPicker from 'svelte-awesome-color-picker';

  let newUsername = '';
  let hex = '';

  let updateLoading = false;
  let updateFailed = false;
  let checkingAuth = true;

  onMount(() => {
    const token = readTokenCookie();
    if (token) {
      checkingAuth = false;

      user.subscribe((value) => {
        if (value) {
          newUsername = value.username;
          hex = value.color;
        }
      });
    } else {
      checkingAuth = false;
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
          // NOTE: logout?
          logout();
          // $user = data.data;
          // setUserCookies(data.data.token, data.data.color);
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
</script>

<div class="flex flex-col items-center justify-center">
  <h1 class="text-center text-6xl my-4">Profile</h1>
  {#if checkingAuth}
    <!-- <div class="flex items-center justify-center"> -->
    <!--   <Loading /> -->
    <!-- </div> -->
  {:else if $user && $user.token.length > 0}
    <div class="flex flex-col items-center justify-center px-4">
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
            <button class="variant-filled-surface" on:click={resetUsername}>Reset</button>
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
              <button class="variant-filled-surface" on:click={resetColor}>Reset</button>
            </div>
          </div>
        </label>
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
