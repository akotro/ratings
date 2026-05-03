import axios from 'axios';
import type { PageServerLoad } from './$types';
import { error } from '@sveltejs/kit';
import { env } from '$env/dynamic/public';
import type { Restaurant } from '$lib/models';

export const load: PageServerLoad = async ({ params }) => {
  const { id } = params;

  try {
    const res = await axios.get(`${env.PUBLIC_API_BASE_URL}/restaurants/${id}`);
    var data = await res.data;
    if (data.success && data.data) {
      return {
        restaurant: data.data as Restaurant
      };
    } else {
      return error(404, 'Restaurant not found');
    }
  } catch (err) {
    throw err;
  }
};
