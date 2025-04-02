<script>
  import { websocket } from '$lib/stores/websocket'
</script>

<style>
  .total-header {
    text-align: center;
    margin: 1rem 0;
    height: 13%;
    display: flex;
    justify-content: center;
    /* background-color: pink; */
    flex-direction: column;
  }
  .total-votes {
    font-weight: bold;
    font-size: max(4vh, 2vw, 1.5rem);
    color: #5e5757;
    margin-bottom: 4vh;
  }
  .total-users {
    position: absolute;
    align-self: center;
    font-family: 'Verdana', 'Geneva', 'sans-serif';
    color: #5e5757;
    opacity: 90%;
    margin-top: 10vh;
    font-size: max(2.25vh, 1.25vw, 1rem);
  }
  @media screen and (max-height: 600px) {
    .total-users {
      margin-top: 10vh; /* adjust as needed */
    }
  }
</style>

<div class="total-header">
  <div class="total-votes">
    <span>{$websocket.total}</span> clicks
  </div>
  <div class="total-users">
    <span>{$websocket.total_users}</span> clickers
  </div>
</div>
