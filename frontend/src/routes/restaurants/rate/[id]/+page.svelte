<script lang="ts">
  import { user } from '$lib/store.js';
  import axios from 'axios';
  import {
    GET_RATING_ENDPOINT,
    GET_RESTAURANT_RATINGS_ENDPOINT,
    RATE_ENDPOINT,
    RESTAURANT_RATING_COMPLETE_ENDPOINT
  } from '$lib/endpoints';
  import Loading from '$lib/loading.svelte';
  import type { Rating } from '$lib/models.js';
  import Chart from '$lib/chart.svelte';
  import { onMount } from 'svelte';
  import { readTokenCookie } from '$lib/auth.js';

  export let data;
  export let id: string = data.restaurant.id;
  let isRatingComplete = false;
  let hasRated = false;
  let rating = 0;
  let rateLoading = false;
  let restaurantRatings = Array<Rating>();
  let datasetData = Array<number>();
  let labels = Array<string>();
  let averageRating = 0;

  let checkingAuth = true;
  onMount(() => {
    const token = readTokenCookie();
    if (token) {
      checkingAuth = false;
    } else {
      checkingAuth = false;
    }
  });

  function handleRatingChange(event: Event) {
    const target = event.target as HTMLInputElement;
    rating = parseFloat(target.value);
  }

  async function get_user_rating() {
    try {
      if ($user && $user.token.length > 0) {
        const response = await axios.get(GET_RATING_ENDPOINT($user.id, id), {
          headers: {
            'Content-Type': 'application/json',
            Authorization: 'Bearer ' + $user.token
          }
        });
        const data = response.data;
        // console.log('Get rating response: ' + JSON.stringify(data));
        if (data && data.success) {
          rating = data.data.score;
          hasRated = true;
        } else {
          rating = 0;
          hasRated = false;
        }
      }
    } catch (error) {
      // console.log('Get rating error: ' + error);
    }
  }

  async function rateRestaurant() {
    rateLoading = true;
    try {
      if ($user && $user.token.length > 0) {
        await axios.post(
          RATE_ENDPOINT($user.id),
          {
            id: -1,
            restaurant_id: id,
            user_id: $user.id,
            username: $user.username,
            score: rating
          },
          {
            headers: {
              'Content-Type': 'application/json',
              Authorization: 'Bearer ' + $user.token
            }
          }
        );
        hasRated = true;
      }
    } catch (error) {
      // console.log('Rate error: ' + error);
    }

    rateLoading = false;
  }

  async function ratingComplete() {
    try {
      while (!hasRated) {
        // console.log('Waiting for rating to be made');
        await new Promise((r) => setTimeout(r, 1000));
      }
      if ($user && $user.token.length > 0) {
        const response = await axios.get(RESTAURANT_RATING_COMPLETE_ENDPOINT(id), {
          headers: {
            'Content-Type': 'application/json',
            Authorization: 'Bearer ' + $user.token
          }
        });
        const data = response.data;
        // console.log('Is restaurant rating complete response: ' + JSON.stringify(data));
        if (data && data.success) {
          isRatingComplete = data.data;
        } else {
          isRatingComplete = false;
        }
      }
    } catch (error) {
      // console.log('Is restaurant rating complete error: ' + error);
    }
  }

  async function getRestaurantRatings() {
    try {
      if ($user && $user.token.length > 0) {
        const response = await axios.get(GET_RESTAURANT_RATINGS_ENDPOINT(id), {
          headers: {
            'Content-Type': 'application/json',
            Authorization: 'Bearer ' + $user.token
          }
        });
        // await new Promise((r) => setTimeout(r, 1000));
        const data = response.data;
        // console.log('Get restaurant ratings response: ' + JSON.stringify(data));
        if (data.success && data.data) {
          restaurantRatings = data.data;
          // console.log('Restaurant ratings: ' + JSON.stringify(restaurantRatings));
          averageRating = getAverageRating(restaurantRatings);
          datasetData = [];
          labels = [];
          restaurantRatings.forEach((rating) => {
            datasetData.push(rating.score);
            labels.push(rating.username);
          });
        } else {
          restaurantRatings = Array<Rating>();
        }
      }
    } catch (error) {
      // console.log('Get restaurant ratings error: ' + error);
    }
  }

  function getAverageRating(restaurantRatings: Rating[]) {
    return restaurantRatings.reduce((acc, cur) => acc + cur.score, 0) / restaurantRatings.length;
  }
</script>

{#if checkingAuth}
  <!-- <div class="flex items-center justify-center my-12"> -->
  <!--   <Loading /> -->
  <!-- </div> -->
{:else if $user && $user.token.length > 0}
  {#await get_user_rating()}
    <!-- <div class="flex items-center justify-center my-12"> -->
    <!--   <Loading /> -->
    <!-- </div> -->
  {:then}
    <h1 class="text-center text-2xl font-bold my-4 mb-4">{id}</h1>
    <h2 class="text-center text-xl font-bold mb-4">Rate It!</h2>

    <div class="flex text-center justify-center space-x-2">
      <form on:submit|preventDefault={rateRestaurant}>
        <label class="label my-2" for="rating"> Rating (out of 10): </label>
        <input
          class="input my-2"
          type="number"
          id="rating"
          name="rating"
          min="0"
          max="10"
          step="0.5"
          bind:value={rating}
          on:change={handleRatingChange}
        />
        {#if rateLoading}
          <button class="btn btn-lg variant-filled-surface">
            <Loading size="6" />Loading
          </button>
        {:else}
          <button class="btn btn-lg variant-filled-surface my-2" type="submit"> Submit </button>
        {/if}
      </form>
    </div>
  {/await}

  <!-- TODO: Rerender this when rating changes -->
  {#if hasRated}
    {#await ratingComplete()}
      <!-- <div class="flex items-center justify-center my-12"> -->
      <!--   <Loading /> -->
      <!-- </div> -->
    {:then}
      {#if isRatingComplete}
        <div class="flex items-center justify-center my-12">
          <h2 class="text-center text-xl font-bold mb-4">Restaurant Ratings</h2>
        </div>

        {#await getRestaurantRatings()}
          <!-- <div class="flex items-center justify-center my-12"> -->
          <!--   <Loading /> -->
          <!-- </div> -->
        {:then}
          {#if restaurantRatings.length > 0}
            <div class="my-12 mx-auto max-w-7xl">
              <Chart {labels} {datasetData} />
            </div>
            <div class="flex items-center justify-center my-12">
              <h2 class="text-center text-lg font-bold mb-4">
                Average Rating: {averageRating.toFixed(2)}
              </h2>
            </div>
          {/if}
        {/await}
      {/if}
    {/await}
  {/if}
{:else}
  <h1 class="p-6 text-8xl text-white text-center">
    Please <a href="/" class="hover:underline dark:text-blue-500">Login</a>
  </h1>
{/if}
