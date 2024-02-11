<script lang="ts">
  import { page } from '$app/stores';
</script>

{#if $page.error}
  <h1 class="text-center text-2xl font-bold my-4 mb-4 text-red-500">Error: {$page.status}</h1>

  <div class="flex flex-col items-center justify-center">
    {#if $page.error.message}
      <h2 class="text-center text-lg font-bold text-red-500">
        Message: {$page.error.message}
      </h2>
    {/if}
    <br />
    <img src="/images/error-404.webp" alt="" />
  </div>
{/if}
