import { error } from '@sveltejs/kit';
import type { PageLoad } from './$types';

export const load: PageLoad = async ({ params }) => {
  const { groupCode } = params;
  const uuidRegex =
    /^[0-9a-fA-F]{8}\-[0-9a-fA-F]{4}\-[0-9a-fA-F]{4}\-[0-9a-fA-F]{4}\-[0-9a-fA-F]{12}/i;
  if (!uuidRegex.test(groupCode)) {
    return error(500, 'Invalid group code!');
  }
  return { groupCode };
};
