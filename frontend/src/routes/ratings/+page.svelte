<script lang="ts">
  import { page } from '$app/stores';
  import { readTokenCookie } from '$lib/auth';
  import Chart from '$lib/chart.svelte';
  import { GET_RATINGS_ENDPOINT } from '$lib/endpoints';
  import Loading from '$lib/loading.svelte';
  import type { AverageRatingPerPeriod, Rating, RatingsByPeriod, Period } from '$lib/models';
  import { user } from '$lib/store';
  import axios from 'axios';
  import { onMount } from 'svelte';
  import {
    TabGroup,
    Tab,
    type PopupSettings,
    popup,
    ListBox,
    ListBoxItem
  } from '@skeletonlabs/skeleton';
  import { goto } from '$app/navigation';

  let ratingsByPeriod: RatingsByPeriod;
  let current_ratings = Array<Rating>();
  let historical_ratings = Array<AverageRatingPerPeriod>();

  let currentAverageRating = 0;
  let currentDatasetData = Array<number>();
  let currentLabels = Array<string>();

  let selectedRestaurant = '';
  let historicalAverageRating = 0;
  let historicalDatasetData = Array<number>();
  let historicalLabels = Array<string>();

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

  const popupCombobox: PopupSettings = {
    event: 'click',
    target: 'popupCombobox',
    placement: 'bottom',
    closeQuery: '.listbox-item'
  };

  async function get_ratings(token: string, userId: string, groupId: string) {
    try {
      const res = await axios.get(GET_RATINGS_ENDPOINT(userId, groupId), {
        headers: {
          'Content-Type': 'application/json',
          Authorization: 'Bearer ' + token
        }
      });

      var data = await res.data;
      // await new Promise((r) => setTimeout(r, 500));
      if (data.success && data.data) {
        ratingsByPeriod = data.data;
        current_ratings = ratingsByPeriod.current_period_ratings;
        historical_ratings = ratingsByPeriod.historical_ratings;

        currentAverageRating =
          current_ratings.reduce((acc, cur) => acc + cur.score, 0) / current_ratings.length;
        currentDatasetData = [];
        currentLabels = [];
        current_ratings.forEach((rating) => {
          currentDatasetData.push(rating.score);
          currentLabels.push(rating.restaurant_id);
        });
      }
    } catch (error) {
      // console.log('Get ratings error: ' + error);
    }
  }

  $: if (selectedRestaurant) {
    const filteredRatings = historical_ratings.filter(
      (rating) => rating.restaurant_id === selectedRestaurant
    );
    historicalAverageRating =
      filteredRatings.reduce((acc, cur) => acc + cur.average_score, 0) / filteredRatings.length;
    // console.log('historicalAverageRating: ' + historicalAverageRating);
    historicalDatasetData = filteredRatings.map((rating) => rating.average_score);
    // console.log('historicalDatasetData: ' + historicalDatasetData);
    historicalLabels = filteredRatings.map((rating) => getPeriodString(rating.year, rating.period));
    // console.log('historicalLabels: ' + historicalLabels);
  } else {
    historicalDatasetData = [];
    historicalLabels = [];
  }

  function getPeriodString(year: number, period: Period) {
    return year + '-' + period;
  }
</script>

<div class="flex-col items-center justify-center mx-auto max-w-7xl">
  <h1 class="text-center text-6xl my-4">Your Ratings</h1>
  {#if checkingAuth}
    <!-- <div class="flex items-center justify-center"> -->
    <!--   <Loading /> -->
    <!-- </div> -->
  {:else if $user && $user.token.length > 0 && $user.groupMembership != null}
    {#await get_ratings($user.token, $user.id, $user.groupMembership.group_id)}
      <div class="flex items-center justify-center">
        <Loading />
      </div>
    {:then}
      {#if current_ratings.length > 0 || historical_ratings.length > 0}
        <TabGroup justify="justify-center">
          <Tab bind:group={tabSet} name="tab1" value={0}>
            <svelte:fragment slot="lead">⌛</svelte:fragment>
            <span>Current</span>
            <span
              >{getPeriodString(ratingsByPeriod.current_year, ratingsByPeriod.current_period)}</span
            >
          </Tab>
          <Tab bind:group={tabSet} name="tab2" value={1}>
            <svelte:fragment slot="lead">🗓</svelte:fragment>️
            <span>Historical</span>
          </Tab>

          <!-- Tab Panels --->
          <svelte:fragment slot="panel">
            {#if tabSet === 0}
              <div class="table-container flex items-center justify-center my-8">
                <table class="table table-hover">
                  <thead>
                    <tr>
                      <th class="text-lg">Restaurant</th>
                      <th class="text-lg">Rating</th>
                    </tr>
                  </thead>
                  <tbody>
                    {#each current_ratings as row}
                      <tr
                        on:click={() => goto(`/restaurants/rate/${row.restaurant_id}`)}
                        class="cursor-pointer hover:bg-primary-100"
                      >
                        <td>{row.restaurant_id}</td>
                        <td>{row.score}</td>
                      </tr>
                    {/each}
                  </tbody>
                  <tfoot>
                    <tr>
                      {#if current_ratings.length > 0}
                        <th class="text-center text-xl" colspan="3">
                          Average Rating: {currentAverageRating.toFixed(2)}
                        </th>
                      {/if}
                    </tr>
                  </tfoot>
                </table>
              </div>

              {#if current_ratings.length > 0}
                <div class="my-12 mx-auto max-w-7xl">
                  <Chart labels={currentLabels} datasetData={currentDatasetData} />
                </div>
              {/if}
            {:else if tabSet === 1}
              <div class="text-center w-full my-4 mx-auto max-w-7xl">
                <label for="restaurant-select" class="block mb-2 text-xl font-bold text-center"
                  >Select a Restaurant</label
                >

                <button
                  class="btn variant-filled-surface px-4 py-2 justify-between overflow-y-visible"
                  use:popup={popupCombobox}
                >
                  <span class="capitalize"
                    >{selectedRestaurant !== '' ? selectedRestaurant : 'Select'}</span
                  >
                  <span>↓</span>
                </button>

                <div class="card w-48 shadow-xl py-2" data-popup="popupCombobox">
                  <ListBox rounded="rounded-none">
                    {#each historical_ratings as historical_rating}
                      <ListBoxItem
                        bind:group={selectedRestaurant}
                        name="medium"
                        value={historical_rating.restaurant_id}
                        >{historical_rating.restaurant_id}</ListBoxItem
                      >
                    {/each}
                  </ListBox>
                  <div class="arrow bg-surface-100-800-token" />
                </div>
              </div>

              {#if selectedRestaurant}
                <div class="flex items-center justify-center my-4">
                  <h2 class="text-center text-lg font-bold">
                    Historical Average Rating: {historicalAverageRating.toFixed(2)}
                  </h2>
                </div>

                {#key historicalDatasetData}
                  <div class="my-4 mx-auto max-w-7xl">
                    <Chart
                      chartType="line"
                      labels={historicalLabels}
                      datasetData={historicalDatasetData}
                    />
                  </div>
                {/key}
              {/if}
            {/if}
          </svelte:fragment>
        </TabGroup>
      {/if}
    {/await}
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
