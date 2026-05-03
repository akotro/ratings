<script lang="ts">
  import { user } from '$lib/store';
  import {
    RESTAURANTS_ENDPOINT,
    CREATE_RESTAURANT_ENDPOINT,
    UPDATE_RESTAURANT_ENDPOINT,
    DELETE_RESTAURANT_ENDPOINT
  } from '$lib/endpoints';
  import { Role, type Restaurant } from '$lib/models';
  import axios from 'axios';
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { getModalStore } from '@skeletonlabs/skeleton';
  import type { ModalSettings } from '@skeletonlabs/skeleton';
  import SearchInput from '$lib/SearchInput.svelte';

  const modalStore = getModalStore();

  let restaurants: Restaurant[] = [];
  let filteredRestaurants: Restaurant[] = [];
  let searchInput = '';
  let newRestaurantCode = '';
  let newCuisine = '';

  let editRestaurantId = -1;
  let editRestaurantCode = '';
  let editCuisine = '';

  onMount(() => {
    if (
      !$user ||
      !$user.token ||
      !$user.groupMembership ||
      $user.groupMembership.role !== Role.Admin
    ) {
      goto('/restaurants');
      return;
    }
    loadRestaurants();
  });

  async function loadRestaurants() {
    if (!$user || !$user.groupMembership) return;
    try {
      const res = await axios.get(RESTAURANTS_ENDPOINT($user.groupMembership.group_id), {
        headers: { Authorization: `Bearer ${$user.token}` }
      });
      console.log('RESTAURANTS LOADED:', res.data);
      if (res.data.success) {
        restaurants = res.data.data || [];
        filterRestaurants();
      }
    } catch (e) {
      console.error(e);
    }
  }

  $: {
    if (searchInput.trim() === '') {
      filteredRestaurants = [...restaurants];
    } else {
      filterRestaurants();
    }
  }

  function filterRestaurants() {
    filteredRestaurants = restaurants.filter(
      (r) =>
        r.restaurant_code.toLowerCase().includes(searchInput.toLowerCase()) ||
        r.cuisine.toLowerCase().includes(searchInput.toLowerCase())
    );
  }

  async function addRestaurant() {
    if (!$user || !$user.groupMembership) return;
    try {
      await axios.post(
        CREATE_RESTAURANT_ENDPOINT,
        {
          id: 0,
          restaurant_code: newRestaurantCode,
          group_id: $user.groupMembership.group_id,
          cuisine: newCuisine,
          menu: []
        },
        { headers: { Authorization: `Bearer ${$user.token}` } }
      );
      newRestaurantCode = '';
      newCuisine = '';
      loadRestaurants();
    } catch (e) {
      console.error(e);
    }
  }

  async function updateRestaurant(id: number) {
    if (!$user || !$user.groupMembership) return;
    try {
      await axios.put(
        UPDATE_RESTAURANT_ENDPOINT(id),
        {
          id: editRestaurantId,
          restaurant_code: editRestaurantCode,
          group_id: $user.groupMembership.group_id,
          cuisine: editCuisine,
          menu: []
        },
        { headers: { Authorization: `Bearer ${$user.token}` } }
      );
      editRestaurantId = -1;
      editRestaurantCode = '';
      editCuisine = '';
      loadRestaurants();
    } catch (e) {
      console.error(e);
    }
  }

  async function deleteRestaurant(id: number) {
    if (!$user || !$user.groupMembership) return;
    try {
      await axios.delete(DELETE_RESTAURANT_ENDPOINT(id), {
        headers: { Authorization: `Bearer ${$user.token}` },
        params: { group_id: $user.groupMembership.group_id }
      });
      loadRestaurants();
    } catch (e) {
      console.error(e);
    }
  }

  function confirmDelete(id: number, code: string) {
    const modal: ModalSettings = {
      type: 'confirm',
      title: 'Please Confirm',
      body: `Are you sure you want to delete restaurant "${code}"? This cannot be undone.`,
      response: (r: boolean) => {
        if (r) {
          deleteRestaurant(id);
        }
      }
    };
    modalStore.trigger(modal);
  }

  function startEdit(r: Restaurant) {
    editRestaurantId = r.id;
    editRestaurantCode = r.restaurant_code;
    editCuisine = r.cuisine;
  }
</script>

<div class="container mx-auto p-4">
  <h1 class="text-4xl mb-4">Manage Restaurants</h1>

  {#if !$user || !$user.token}
    <p>Please login.</p>
  {:else if !$user.groupMembership}
    <p>Please select a group.</p>
  {:else if $user.groupMembership.role !== Role.Admin}
    <p>Only group admins can manage restaurants.</p>
  {:else}
    <div class="mb-8 p-4 border border-gray-700 rounded w-full">
      <h2 class="text-2xl mb-4">Add New Restaurant</h2>
      <div class="flex flex-col sm:flex-row gap-4">
        <input class="input flex-1" type="text" placeholder="Name" bind:value={newRestaurantCode} />
        <input class="input flex-1" type="text" placeholder="Cuisine" bind:value={newCuisine} />
        <button class="btn variant-filled-primary sm:w-auto w-full" on:click={addRestaurant}
          >Add</button
        >
      </div>
    </div>

    <div class="w-full">
      <h2 class="text-2xl mb-4">Existing Restaurants</h2>

      <SearchInput bind:value={searchInput} placeholder="Search existing restaurants..." />

      <ul class="space-y-4">
        {#each filteredRestaurants as r}
          <li class="p-4 border border-gray-700 rounded flex flex-col gap-4">
            {#if editRestaurantId === r.id}
              <div class="flex flex-col sm:flex-row gap-4 w-full">
                <input class="input flex-1" type="text" bind:value={editRestaurantCode} />
                <input class="input flex-1" type="text" bind:value={editCuisine} />
              </div>
              <div class="flex gap-2 justify-end w-full">
                <button
                  class="btn variant-filled-success flex-1 sm:flex-none"
                  on:click={() => updateRestaurant(r.id)}>Save</button
                >
                <button
                  class="btn variant-ghost flex-1 sm:flex-none"
                  on:click={() => (editRestaurantId = -1)}>Cancel</button
                >
              </div>
            {:else}
              <div class="w-full break-words">
                <strong class="text-lg">{r.restaurant_code}</strong> <br />
                <span class="text-gray-400">{r.cuisine}</span>
              </div>
              <div class="flex gap-2 justify-end w-full mt-2">
                <button
                  class="btn variant-ghost-warning flex-1 sm:flex-none"
                  on:click={() => startEdit(r)}>Edit</button
                >
                <button
                  class="btn variant-ghost-error flex-1 sm:flex-none"
                  on:click={() => confirmDelete(r.id, r.restaurant_code)}>Delete</button
                >
              </div>
            {/if}
          </li>
        {/each}
      </ul>
    </div>
  {/if}
</div>
