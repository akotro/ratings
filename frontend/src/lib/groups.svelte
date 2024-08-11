<script lang="ts">
  import { user } from '$lib/store';
  import {
    CREATE_GROUP_ENDPOINT,
    JOIN_GROUP_ENDPOINT,
    GET_GROUP_MEMBERSHIPS_ENDPOINT,
    SHARE_GROUP_ENDPOINT
  } from '$lib/endpoints';
  import Loading from './loading.svelte';
  import axios from 'axios';
  import { onMount } from 'svelte';
  import { readTokenCookie, setGroupCookie } from '$lib/auth';
  import { writable } from 'svelte/store';
  import { Role, type GroupMembership } from './models';
  import { goto } from '$app/navigation';
  import { getToastStore, type ToastSettings } from '@skeletonlabs/skeleton';
  import { page } from '$app/stores';

  const toastStore = getToastStore();

  let checkingAuth = true;

  let group_memberships: Array<GroupMembership> = [];

  let createGroupLoading = false;
  let createGroupError = writable<string | null>(null);
  let groupName = '';
  let groupDescription = '';

  let joinGroupLoading = false;
  let joinGroupError = writable<string | null>(null);
  let joinCode = '';

  const currentPage = writable<'main' | 'create' | 'join'>('main');

  onMount(() => {
    const token = readTokenCookie();
    if (token) {
      checkingAuth = false;
    } else {
      checkingAuth = false;
    }
  });

  async function get_group_memberships(userId: string, token: string) {
    const res = await axios.get(GET_GROUP_MEMBERSHIPS_ENDPOINT(userId), {
      headers: {
        'Content-Type': 'application/json',
        Authorization: 'Bearer ' + token
      }
    });
    var data = await res.data;
    if (data.success && data.data) {
      group_memberships = data.data;
    } else {
      throw new Error('Failed getting groups');
    }
  }

  async function createGroup() {
    createGroupError.set(null);

    try {
      createGroupLoading = true;

      if ($user && $user.token.length > 0) {
        const response = await axios.post(
          CREATE_GROUP_ENDPOINT,
          { name: groupName, description: groupDescription, creator_id: $user.id },
          {
            headers: {
              'Content-Type': 'application/json',
              Authorization: 'Bearer ' + $user.token
            }
          }
        );
        const data = response.data;

        if (data.success && data.data) {
          group_memberships.push(data.data);
          currentPage.set('main');
        } else {
          createGroupError.set('Failed to create group');
        }
      }
    } catch (error) {
      console.log(error);
      createGroupError.set('Failed to create group');
    }

    createGroupLoading = false;
  }

  async function joinGroup() {
    joinGroupError.set(null);

    try {
      joinGroupLoading = true;

      if ($user && $user.token.length > 0) {
        const response = await axios.post(
          JOIN_GROUP_ENDPOINT,
          {
            group_id: joinCode,
            user_id: $user.id,
            role: Role.Member
          },
          {
            headers: {
              'Content-Type': 'application/json',
              Authorization: 'Bearer ' + $user.token
            }
          }
        );
        const data = response.data;

        if (data.success && data.data) {
          group_memberships.push(data.data);
          currentPage.set('main');
        } else {
          joinGroupError.set('Failed to join group. It most likely does not exist!');
        }
      }
    } catch (error) {
      console.log(error);
      joinGroupError.set('Failed to join group. It most likely does not exist!');
    }

    joinGroupLoading = false;
  }

  function setGroup(groupMembership: GroupMembership) {
    if ($user) {
      $user.groupMembership = groupMembership;
      setGroupCookie(groupMembership);
      goto('/');
    }
  }

  function copyToClipboard(groupId: string) {
    const shareLink = SHARE_GROUP_ENDPOINT(groupId);
    navigator.clipboard
      .writeText(shareLink)
      .then(() => {
        const notification: ToastSettings = {
          message: 'Join link copied to clipboard!',
          timeout: 3000,
          background: 'variant-filled-surface'
        };
        toastStore.trigger(notification);
      })
      .catch((err) => {
        console.error('Failed to copy: ', err);
      });
  }

  $: adminGroups = group_memberships?.filter((gm) => gm.role === Role.Admin) || [];
  $: memberGroups = group_memberships?.filter((gm) => gm.role !== Role.Admin) || [];
</script>

{#if $currentPage === 'main'}
  <div class="flex flex-col items-center justify-center">
    {#if checkingAuth}
      <!-- <div class="flex items-center justify-center my-12"> -->
      <!--   <Loading /> -->
      <!-- </div> -->
    {:else if $user && $user.token.length > 0}
      {#await get_group_memberships($user.id, $user.token)}
        <div class="flex items-center justify-center">
          <Loading />
        </div>
      {:then}
        {#if group_memberships?.length > 0}
          <div>
            {#if adminGroups?.length > 0}
              <h3 class="text-center text-3xl my-4">Your Groups</h3>
              <nav class="list-nav">
                <ul class="grid grid-cols-1 gap-4">
                  {#each adminGroups as group_membership}
                    <li class="flex items-center justify-between">
                      <button
                        class="flex-grow space-x-2 btn btn-md variant-outline"
                        on:click|preventDefault={() => setGroup(group_membership)}
                      >
                        <span class="badge bg-violet-500"> 👥 </span>
                        <span class="group-name flex-auto">{group_membership.group.name}</span>
                      </button>
                      <button
                        class="btn btn-md variant-outline"
                        on:click|preventDefault={() => copyToClipboard(group_membership.group_id)}
                        >Share</button
                      >
                    </li>
                  {/each}
                </ul>
              </nav>
            {/if}

            {#if memberGroups?.length > 0}
              <h3 class="text-center text-3xl my-4">Joined Groups</h3>
              <nav class="list-nav">
                <ul>
                  {#each memberGroups as group_membership}
                    <li class="flex items-center justify-between">
                      <button
                        class="flex-grow space-x-2 btn btn-md variant-outline"
                        on:click|preventDefault={() => setGroup(group_membership)}
                      >
                        <span class="badge bg-violet-500"> 👥 </span>
                        <span class="group-name flex-auto">{group_membership.group.name}</span>
                      </button>
                    </li>
                  {/each}
                </ul>
              </nav>
            {/if}
          </div>
        {:else}
          <p>You are not currently in any groups. Create or join one.</p>
        {/if}
      {/await}

      <div class="flex space-x-4 mt-4">
        <button class="btn btn-lg variant-filled-surface" on:click={() => currentPage.set('create')}
          >Create Group</button
        >
        <button class="btn btn-lg variant-filled-surface" on:click={() => currentPage.set('join')}
          >Join Group</button
        >
      </div>
    {:else}
      <h1 class="p-6 text-8xl text-white text-center">
        Please <a
          href={$page ? `/login?redirect=${$page.route.id}` : '/'}
          class="hover:underline dark:text-blue-500">Login</a
        >
      </h1>
    {/if}
  </div>
{:else if $currentPage === 'create'}
  <div class="flex flex-col items-center justify-center">
    <h3 class="text-center text-3xl my-4">Create Group</h3>
    <form class="create-group-form" on:submit|preventDefault={createGroup}>
      <label class="label">
        <span class="text-xl">Name</span>
        <input class="input" type="text" bind:value={groupName} />
      </label>
      <label class="label">
        <span class="text-xl">Description</span>
        <input class="input" type="text" bind:value={groupDescription} />
      </label>
      <br />
      {#if createGroupLoading}
        <button class="btn btn-lg variant-filled-surface" disabled>
          <Loading />
          Loading
        </button>
      {:else}
        <button class="btn btn-lg variant-filled-surface" type="submit">Create Group</button>
      {/if}
      {#if $createGroupError}
        <p class="text-red-500">{$createGroupError}</p>
      {/if}
    </form>
    <button class="btn btn-lg variant-filled-surface mt-4" on:click={() => currentPage.set('main')}
      >Back</button
    >
  </div>
{:else if $currentPage === 'join'}
  <div class="flex flex-col items-center justify-center">
    <h3 class="text-center text-3xl my-4">Join Group</h3>
    <form class="create-group-form" on:submit|preventDefault={joinGroup}>
      <label class="label">
        <span class="text-xl">Code</span>
        <input class="input" type="text" bind:value={joinCode} />
      </label>
      <br />
      {#if joinGroupLoading}
        <button class="btn btn-lg variant-filled-surface" disabled>
          <Loading />
          Loading
        </button>
      {:else}
        <button class="btn btn-lg variant-filled-surface" type="submit">Join Group</button>
      {/if}
      {#if $joinGroupError}
        <p class="text-red-500">{$joinGroupError}</p>
      {/if}
    </form>
    <button class="btn btn-lg variant-filled-surface mt-4" on:click={() => currentPage.set('main')}
      >Back</button
    >
  </div>
{/if}
