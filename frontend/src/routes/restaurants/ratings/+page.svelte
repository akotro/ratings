<script lang="ts">
  import { readTokenCookie } from '$lib/auth';
  import Chart from '$lib/chart.svelte';
  import { GET_RATINGS_ENDPOINT } from '$lib/endpoints';
  import type { Rating } from '$lib/models';
  import { user } from '$lib/store';
  import axios from 'axios';
  import { onMount } from 'svelte';

  let ratings = Array<Rating>();
  let averageRating = 0;
  let datasetData = Array<number>();
  let labels = Array<string>();

  let checkingAuth = true;
  onMount(() => {
    const token = readTokenCookie();
    if (token) {
      checkingAuth = false;
    } else {
      checkingAuth = false;
    }
  });

  async function get_ratings(userId: string, token: string) {
    try {
      const res = await axios.get(GET_RATINGS_ENDPOINT(userId), {
        headers: {
          'Content-Type': 'application/json',
          Authorization: 'Bearer ' + token
        }
      });

      var data = await res.data;
      // await new Promise((r) => setTimeout(r, 500));
      if (data.success && data.data) {
        ratings = data.data;
        averageRating = ratings.reduce((acc, cur) => acc + cur.score, 0) / ratings.length;
        datasetData = [];
        labels = [];
        ratings.forEach((rating) => {
          datasetData.push(rating.score);
          labels.push(rating.restaurant_id);
        });
      }
    } catch (error) {
      // console.log('Get ratings error: ' + error);
    }
  }
</script>

<div class="flex-col items-center justify-center mx-auto max-w-7xl">
  <h1 class="text-center text-6xl my-4">Your Ratings</h1>
  {#if checkingAuth}
    <!-- <div class="flex items-center justify-center"> -->
    <!--   <Loading /> -->
    <!-- </div> -->
  {:else if $user && $user.token.length > 0}
    {#await get_ratings($user.id, $user.token)}
      <!-- <div class="flex items-center justify-center"> -->
      <!--   <Loading /> -->
      <!-- </div> -->
    {:then}
      <div class="table-container flex items-center justify-center my-8">
        <table class="table table-hover">
          <thead>
            <tr>
              <th class="text-lg">Restaurant</th>
              <th class="text-lg">Rating</th>
            </tr>
          </thead>
          <tbody>
            {#each ratings as row}
              <tr>
                <td>{row.restaurant_id}</td>
                <td>{row.score}</td>
              </tr>
            {/each}
          </tbody>
          <tfoot>
            <tr>
              {#if ratings.length > 0}
                <th class="text-center text-xl" colspan="3">
                  Average Rating: {averageRating.toFixed(2)}
                </th>
              {/if}
            </tr>
          </tfoot>
        </table>
      </div>

      {#if ratings.length > 0}
        <div class="my-12 mx-auto max-w-7xl">
          <Chart {labels} {datasetData} />
        </div>
      {/if}
    {/await}
  {:else}
    <h1 class="p-6 text-8xl text-white text-center">
      Please <a href="/" class="hover:underline dark:text-blue-500">Login</a>
    </h1>
  {/if}
</div>
