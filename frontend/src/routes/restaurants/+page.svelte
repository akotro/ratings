<script lang="ts">
  import { user } from '../..//lib/store';
  import { RESTAURANTS_WITH_AVG_RATING_ENDPOINT } from '$lib/endpoints';
  import type { Restaurant } from '$lib/models';
  import axios from 'axios';
  import { onMount } from 'svelte';
  import { readTokenCookie } from '$lib/auth';
  import Loading from '$lib/loading.svelte';

  let restaurants: Array<[Restaurant, number]> = [];

  let checkingAuth = true;
  onMount(() => {
    const token = readTokenCookie();
    if (token) {
      checkingAuth = false;
    } else {
      checkingAuth = false;
    }
  });

  async function get_restaurants(token: string, groupId: string) {
    const res = await axios.get(RESTAURANTS_WITH_AVG_RATING_ENDPOINT(groupId), {
      headers: {
        'Content-Type': 'application/json',
        Authorization: 'Bearer ' + token
      }
    });
    var data = await res.data;
    // await new Promise((r) => setTimeout(r, 500));
    if (data.success && data.data) {
      restaurants = data.data;
    } else {
      throw new Error('Failed getting restaurants');
    }
  }
</script>

<div class="flex flex-col items-center justify-center">
  <h1 class="text-center text-6xl my-4">Restaurants</h1>
  {#if checkingAuth}
    <!-- <div class="flex items-center justify-center my-12"> -->
    <!--   <Loading /> -->
    <!-- </div> -->
  {:else if $user && $user.token.length > 0 && $user.groupMembership != null}
    {#await get_restaurants($user.token, $user.groupMembership.group_id)}
      <div class="flex items-center justify-center my-12">
        <Loading />
      </div>
    {:then}
      <nav class="list-nav">
        <!-- (optionally you can provide a label here) -->
        <ul>
          {#each restaurants as [restaurant, avg_rating]}
            <li>
              <a href="/restaurants/rate/{restaurant.id}">
                <span class="badge bg-tertiary-500">🍽️</span>
                <span class="flex-auto">{restaurant.id}</span>
                {#if avg_rating > 0}
                  <span class="badge bg-secondary-500">{avg_rating.toFixed(2)}</span>
                {/if}
              </a>
            </li>
          {/each}
          <li />
        </ul>
      </nav>
    {:catch error}
      <p style="color: red">{error.message}</p>
    {/await}
  {:else if $user == null || $user.token == null}
    <h1 class="p-6 text-8xl text-white text-center">
      Please <a href="/" class="hover:underline dark:text-blue-500">Login</a>
    </h1>
  {:else if $user.groupMembership == null}
    <h1 class="p-6 text-8xl text-white text-center">
      Please <a href="/" class="hover:underline dark:text-blue-500">Select a Group</a>
    </h1>
  {/if}
</div>
