import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig, type UserConfig } from 'vite';
// import { purgeCss } from 'vite-plugin-tailwind-purgecss';

const config: UserConfig = {
  plugins: [
    sveltekit()
    // purgeCss({
    //   content: ['./src/**/*.{html,js,svelte,ts}'],
    //   safelist: [
    //     // You can add specific classes to safelist here
    //     // For example, to safelist all width and height utilities:
    //     /^w-/, // This regex will safelist any class that starts with "w-"
    //     /^h-/, // This regex will safelist any class that starts with "h-"
    //     // You can also safelist specific classes:
    //     'w-12',
    //     'h-12' // Safelist specific classes by name
    //     // If your dynamic classes follow a predictable pattern, use regex to match them
    //   ]
    // })
  ]
};

export default defineConfig(config);
