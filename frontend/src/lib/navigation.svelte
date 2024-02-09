<script lang="ts">
  import { getDrawerStore } from '@skeletonlabs/skeleton';
  const drawerStore = getDrawerStore();

  function drawerClose(): void {
    drawerStore.close();
  }
</script>

<nav class="flex items-center list-nav p-2">
  <ul>
    <li class="my-2">
      <a class="btn btn-sm variant-ghost-surface" href="/" rel="noreferrer" on:click={drawerClose}>
        Home
      </a>
    </li>
    <li class="my-2">
      <a
        class="btn btn-sm variant-ghost-surface"
        href="/restaurants"
        rel="noreferrer"
        on:click={drawerClose}
      >
        Restaurants
      </a>
    </li>
    <li class="my-2">
      <a
        class="btn btn-sm variant-ghost-surface"
        href="/restaurants/ratings"
        rel="noreferrer"
        on:click={drawerClose}
      >
        Ratings
      </a>
    </li>
  </ul>
</nav>
