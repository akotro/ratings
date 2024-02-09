import { writable } from 'svelte/store';
import type { User } from './models';

export const user = writable<User | null>(null);
