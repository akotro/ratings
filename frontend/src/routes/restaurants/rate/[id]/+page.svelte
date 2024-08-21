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
  import type {
    AverageRatingPerPeriod,
    NewRating,
    Period,
    Rating,
    RatingsByPeriod
  } from '$lib/models.js';
  import Chart from '$lib/chart.svelte';
  import { onMount } from 'svelte';
  import { readTokenCookie } from '$lib/auth.js';
  import { TabGroup, Tab } from '@skeletonlabs/skeleton';

  export let data;
  export let id: string = data.restaurant.id;

  let isRatingComplete = false;
  let hasRated = false;

  let rating = 0;
  let rateLoading = false;

  let ratingsByPeriod: RatingsByPeriod;
  let current_period_ratings = Array<Rating>();
  let historical_ratings = Array<AverageRatingPerPeriod>();

  let currentDatasetData = Array<number>();
  let currentLabels = Array<string>();
  let currentBackgroundColors = Array<string>();

  let historicalDatasetData = Array<number>();
  let historicalLabels = Array<string>();

  let currentAverageRating = 0;
  let historicalAverageRating = 0;
  let rateFailed = false;

  let tabSet = 0;

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
      if ($user && $user.token.length > 0 && $user.groupMembership != null) {
        const response = await axios.get(
          GET_RATING_ENDPOINT($user.id, id, $user.groupMembership.group_id),
          {
            headers: {
              'Content-Type': 'application/json',
              Authorization: 'Bearer ' + $user.token
            }
          }
        );
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
    hasRated = false;
    rateLoading = true;
    rateFailed = false;

    try {
      if ($user && $user.token.length > 0 && $user.groupMembership != null) {
        const response = await axios.post(
          RATE_ENDPOINT($user.id),
          {
            id: -1,
            restaurant_id: id,
            user_id: $user.id,
            username: $user.username,
            score: rating,
            group_id: $user.groupMembership.group_id
          } as NewRating,
          {
            headers: {
              'Content-Type': 'application/json',
              Authorization: 'Bearer ' + $user.token
            }
          }
        );
        const data = response.data;
        if (data.success && data.data) {
          hasRated = true;
        } else {
          rateFailed = true;
        }
      }
    } catch (error) {
      // console.log('Rate error: ' + error);
      rateFailed = true;
    }

    rateLoading = false;
  }

  async function ratingComplete() {
    try {
      while (!hasRated) {
        // console.log('Waiting for rating to be made');
        await new Promise((r) => setTimeout(r, 1000));
      }
      if ($user && $user.token.length > 0 && $user.groupMembership != null) {
        const response = await axios.get(
          RESTAURANT_RATING_COMPLETE_ENDPOINT(id, $user.groupMembership.group_id),
          {
            headers: {
              'Content-Type': 'application/json',
              Authorization: 'Bearer ' + $user.token
            }
          }
        );
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
      if ($user && $user.token.length > 0 && $user.groupMembership != null) {
        const response = await axios.get(
          GET_RESTAURANT_RATINGS_ENDPOINT(id, $user.groupMembership.group_id),
          {
            headers: {
              'Content-Type': 'application/json',
              Authorization: 'Bearer ' + $user.token
            }
          }
        );
        // await new Promise((r) => setTimeout(r, 1000));

        const data = response.data;
        if (data.success && data.data) {
          ratingsByPeriod = data.data;
          current_period_ratings = ratingsByPeriod.current_period_ratings;
          historical_ratings = ratingsByPeriod.historical_ratings;

          currentAverageRating =
            current_period_ratings.reduce((acc, cur) => acc + cur.score, 0) /
            current_period_ratings.length;
          currentDatasetData = [];
          currentLabels = [];
          current_period_ratings.forEach((rating) => {
            currentDatasetData.push(rating.score);
            currentLabels.push(rating.username);
            if (rating.color) {
              currentBackgroundColors.push(rating.color);
            }
          });

          historicalAverageRating =
            historical_ratings.reduce((acc, cur) => acc + cur.average_score, 0) /
            historical_ratings.length;
          historicalDatasetData = [];
          historicalLabels = [];
          historical_ratings.forEach((averate_rating_per_period) => {
            historicalDatasetData.push(averate_rating_per_period.average_score);
            historicalLabels.push(
              getPeriodString(averate_rating_per_period.year, averate_rating_per_period.period)
            );
          });
        } else {
          current_period_ratings = Array<Rating>();
        }
      }
    } catch (error) {
      // console.log('Get restaurant ratings error: ' + error);
    }
  }

  function getPeriodString(year: number, period: Period) {
    return year + '-' + period;
  }
</script>

{#if checkingAuth}
  <!-- <div class="flex items-center justify-center my-12"> -->
  <!--   <Loading /> -->
  <!-- </div> -->
{:else if $user && $user.token.length > 0 && $user.groupMembership != null}
  {#await get_user_rating()}
    <!-- <div class="flex items-center justify-center my-12"> -->
    <!--   <Loading /> -->
    <!-- </div> -->
  {:then}
    <h1 class="text-center text-5xl font-bold my-4 mb-4">{id}</h1>
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

  {#if rateFailed}
    <div class="flex items-center justify-center my-12">
      <p class="text-red-500">Operation failed. Please try again.</p>
    </div>
  {/if}

  {#if hasRated}
    {#await ratingComplete()}
      <!-- <div class="flex items-center justify-center my-12"> -->
      <!--   <Loading /> -->
      <!-- </div> -->
    {:then}
      {#if isRatingComplete}
        <div class="flex items-center justify-center my-6">
          <h2 class="text-center text-xl font-bold mb-0">Restaurant Ratings</h2>
        </div>

        {#await getRestaurantRatings()}
          <!-- <div class="flex items-center justify-center my-12"> -->
          <!--   <Loading /> -->
          <!-- </div> -->
        {:then}
          {#if current_period_ratings.length > 0}
            <TabGroup justify="justify-center">
              <Tab bind:group={tabSet} name="tab1" value={0}>
                <svelte:fragment slot="lead">⌛</svelte:fragment>
                <span>Current</span>
                <span
                  >{getPeriodString(
                    ratingsByPeriod.current_year,
                    ratingsByPeriod.current_period
                  )}</span
                >
              </Tab>
              <Tab bind:group={tabSet} name="tab2" value={1}>
                <svelte:fragment slot="lead">🗓</svelte:fragment>️
                <span>Historical</span>
              </Tab>
              <!-- Tab Panels --->
              <svelte:fragment slot="panel">
                {#if tabSet === 0}
                  <div class="flex items-center justify-center my-4">
                    <h2 class="text-center text-lg font-bold">
                      Current Average Rating: {currentAverageRating.toFixed(2)}
                    </h2>
                  </div>

                  <div class="my-4 mx-auto max-w-7xl">
                    <Chart
                      labels={currentLabels}
                      datasetData={currentDatasetData}
                      backgroundColors={currentBackgroundColors}
                    />
                  </div>
                {:else if tabSet === 1}
                  <div class="flex items-center justify-center my-4">
                    <h2 class="text-center text-lg font-bold">
                      Historical Average Rating: {historicalAverageRating.toFixed(2)}
                    </h2>
                  </div>

                  <div class="my-4 mx-auto max-w-7xl">
                    <Chart
                      chartType="line"
                      labels={historicalLabels}
                      datasetData={historicalDatasetData}
                    />
                  </div>
                {/if}
              </svelte:fragment>
            </TabGroup>
          {/if}
        {/await}
      {/if}
    {/await}
  {/if}
{:else if $user == null || $user.token == null}
  <h1 class="p-6 text-8xl text-white text-center">
    Please <a href="/" class="hover:underline dark:text-blue-500">Login</a>
  </h1>
{:else if $user.groupMembership == null}
  <h1 class="p-6 text-8xl text-white text-center">
    Please <a href="/" class="hover:underline dark:text-blue-500">Select a Group</a>
  </h1>
{/if}
