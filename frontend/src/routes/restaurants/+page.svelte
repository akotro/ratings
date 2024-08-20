<script lang="ts">
  import { user } from '../..//lib/store';
  import { RESTAURANTS_WITH_AVG_RATING_ENDPOINT } from '$lib/endpoints';
  import type { Restaurant } from '$lib/models';
  import axios from 'axios';
  import { onMount } from 'svelte';
  import { readTokenCookie } from '$lib/auth';
  import Loading from '$lib/loading.svelte';
  import { page } from '$app/stores';
  import { Autocomplete, popup, type PopupSettings } from '@skeletonlabs/skeleton';

  let restaurants: Array<[Restaurant, number]> = [];
  let filteredRestaurants: Array<[Restaurant, number]> = [];
  let searchInput = '';
  let restaurantOptions: Array<{ label: string; value: string }> = [];
  let popupSettings: PopupSettings = {
    event: 'focus-click',
    target: 'popupAutocomplete',
    placement: 'bottom'
  };

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
    if (data.success && data.data) {
      restaurants = data.data;
      filteredRestaurants = [...restaurants];
      restaurantOptions = restaurants.map(([restaurant]) => ({
        label: restaurant.id,
        value: restaurant.id
      }));
    } else {
      throw new Error('Failed getting restaurants');
    }
  }

  $: filterRestaurants();
  $: {
    if (searchInput.trim() === '') {
      filteredRestaurants = [...restaurants];
    } else {
      filterRestaurants();
    }
  }

  function filterRestaurants() {
    filteredRestaurants = restaurants.filter(([restaurant]) =>
      restaurant.id.toLowerCase().includes(searchInput.toLowerCase())
    );
  }

  // function onRestaurantSelection(event: CustomEvent<{ label: string; value: string }>): void {
  //   searchInput = event.detail.label;
  //   filterRestaurants();
  // }
</script>

<div class="flex flex-col items-center justify-center">
  <h1 class="text-center text-6xl my-4">Restaurants</h1>
  {#if checkingAuth}
    <!-- <div class="flex items-center justify-center my-12"> -->
    <!--   <Loading /> -->
    <!-- </div> -->
  {:else if $user && $user.token.length > 0 && $user.groupMembership != null}
    <div class="flex flex-col w-full max-w-lg">
      <input
        class="input autocomplete mb-4"
        type="search"
        name="autocomplete-search"
        bind:value={searchInput}
        placeholder="Search..."
        use:popup={popupSettings}
      />

      {#await get_restaurants($user.token, $user.groupMembership.group_id)}
        <div class="flex items-center justify-center my-12 flex-grow">
          <Loading />
        </div>
      {:then}
        <nav class="list-nav">
          <ul class="space-y-2">
            {#each filteredRestaurants as [restaurant, avg_rating]}
              <li>
                <a
                  href="/restaurants/rate/{restaurant.id}"
                  class="flex items-center justify-between w-full"
                >
                  <div class="flex items-center">
                    <span class="badge bg-tertiary-500 mr-2">🍽️</span>
                    <span class="text-left">{restaurant.id}</span>
                  </div>
                  {#if avg_rating > 0}
                    <span class="badge bg-secondary-500">{avg_rating.toFixed(2)}</span>
                  {/if}
                </a>
              </li>
            {/each}
          </ul>
        </nav>
      {:catch error}
        <p class="text-red-500">{error.message}</p>
      {/await}
    </div>
  {:else if $user == null || $user.token == null}
    <h1 class="p-6 text-8xl text-white text-center">
      Please <a
        href={$page ? `/login?redirect=${$page.route.id}` : '/'}
        class="hover:underline dark:text-blue-500">Login</a
      >
    </h1>
  {:else if $user.groupMembership == null}
    <h1 class="p-6 text-8xl text-white text-center">
      Please <a href="/" class="hover:underline dark:text-blue-500">Select a Group</a>
    </h1>
  {/if}
</div>
