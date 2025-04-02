<script>
  import { websocket } from '$lib/stores/websocket'
</script>

<style>
  .total-votes {
    text-align: center;
    margin: 1rem 0;
    font-weight: bold;
    font-size: max(4vh, 2vw, 1.5rem);
    color: #5e5757;
    height: 13%;
    justify-content: center;
    align-content: center;
  }
</style>

<div class="total-votes">
  <span>{$websocket.total}</span> clicks
</div>
