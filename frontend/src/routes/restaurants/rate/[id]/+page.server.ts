import axios from 'axios';
import type { PageServerLoad } from './$types';
import { error } from '@sveltejs/kit';
import { RESTAURANTS_ENDPOINT } from '$lib/endpoints';
import type { Restaurant } from '$lib/models';

let restaurants: Array<Restaurant> = [];

export const load: PageServerLoad = async ({ params }) => {
  const { id } = params;

  try {
    const res = await axios.get(RESTAURANTS_ENDPOINT);
    var data = await res.data;
    // await new Promise((r) => setTimeout(r, 500));
    if (data.success && data.data) {
      restaurants = data.data;

      let restaurant = restaurants.find((restaurant) => restaurant.id === id);
      if (!restaurant) {
        return error(404, 'Restaurant not found');
      }

      return {
        restaurant
      };
    } else {
      return error(404, 'Restaurants not found');
    }
  } catch (err) {
    throw err;
  }
};
