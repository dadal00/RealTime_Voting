<script>
  import { websocket } from '$lib/stores/websocket'

  const { large } = $props()
</script>

<style>
  .total-votes {
    text-align: center;
    margin: 1rem 0;
    font-weight: 500;
  }

  .large {
    font-size: 1.5rem;
  }

  .vote-count {
    font-weight: 700;
    color: #4a5568;
  }
</style>

<div class="total-votes {large ? 'large' : ''}">
  <span class="vote-count">{$websocket.total}</span> clicks
</div>
