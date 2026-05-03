<script lang="ts">
  export let value: string = '';
  export let placeholder: string = 'Search...';
  export let usePopup: boolean = false;
  export let popupSettings: any = undefined;

  import { popup } from '@skeletonlabs/skeleton';

  function applyPopup(node: HTMLElement, settings: any) {
    if (usePopup && settings) {
      return popup(node, settings);
    }
    return {};
  }
</script>

<input
  class="input mb-4 autocomplete"
  type="search"
  bind:value
  {placeholder}
  use:applyPopup={popupSettings}
/>
