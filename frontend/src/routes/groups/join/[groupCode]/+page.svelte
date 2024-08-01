<script lang="ts">
  import { onMount } from 'svelte';
  import { getUserFromToken, readTokenCookie } from '$lib/auth';
  import { goto } from '$app/navigation';
  import { writable } from 'svelte/store';
  import Loading from '$lib/loading.svelte';
  import axios from 'axios';
  import { JOIN_GROUP_ENDPOINT } from '$lib/endpoints';
  import { Role } from '$lib/models';

  export let data;
  export let groupCode: string = data.groupCode;

  let joinError = writable<string | null>(null);
  let loading = true;

  async function joinGroup(groupCode: string, token: string): Promise<Boolean> {
    joinError.set(null);

    try {
      const user = getUserFromToken(token);
      if (!user) {
        return false;
      }

      const response = await axios.post(
        JOIN_GROUP_ENDPOINT,
        {
          group_id: groupCode,
          user_id: user.id,
          role: Role.Member
        },
        {
          headers: {
            'Content-Type': 'application/json',
            Authorization: 'Bearer ' + user.token
          },
          validateStatus: (status) => status >= 200 && status <= 500
        }
      );
      const data = response.data;

      if (!data.success || !data.data) {
        joinError.set(`Failed to join group: ${data.message}`);
        return false;
      }
    } catch (error) {
      joinError.set('Failed to join group');
      return false;
    }

    return true;
  }

  onMount(async () => {
    loading = true;
    const token = readTokenCookie();
    if (!token) {
      console.log('redirecting to login..');
      goto(`/login?redirect=/groups/join/${groupCode}`);
    } else {
      getUserFromToken(token);
      try {
        let joinGroupResult = await joinGroup(groupCode, token);
        if (joinGroupResult) {
          goto('/groups');
        }
      } catch (error) {
        joinError.set('Failed to join group.');
      } finally {
        loading = false;
      }
    }
  });
</script>

<h1 class="text-center text-4xl font-bold my-4 mb-4">Joining Group</h1>
{#if loading}
  <div class="flex items-center justify-center">
    <Loading />
  </div>
{:else if $joinError}
  <div class="flex items-center justify-center my-12">
    <p class="text-red-500">{$joinError}</p>
  </div>
{/if}
