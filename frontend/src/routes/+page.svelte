<script>
  import { onMount, onDestroy } from 'svelte'
  import { websocket } from '$lib/stores/websocket'
  import BarChart from '$lib/components/BarChart.svelte'
  import VoteButton from '$lib/components/VoteButton.svelte'
  import TotalVotes from '$lib/components/TotalVotes.svelte'

  onMount(() => {
    websocket.connect()
  })

  onDestroy(() => {
    websocket.disconnect()
  })

  const chartData = $derived(
    Object.entries($websocket)
      .filter(([key]) => key !== 'total')
      .map(([color, count]) => ({ color, count }))
  )
</script>

<style>
  :global(body, html) {
    margin: 0;
  }

  .page-container {
    display: flex;
    flex-direction: column;
    height: 100vh;
    width: 100vw;
    background-color: #faf4ee;
    position: fixed;
  }

  .footer-container {
    bottom: 0;
    left: 0;
    right: 0;
    position: fixed;
    min-height: 4rem;
    height: 17vh;
    background-color: #f5f0e6;
    border-top: 1px solid rgb(188, 185, 178);
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 6vw;
  }

  .body-container {
    overflow: hidden;
    background-color: #faf4ee;
    height: 83vh;
  }
</style>

<main class="page-container">
  <div class="body-container">
    <TotalVotes />
    <BarChart data={chartData} />
  </div>
  <div class="footer-container">
    <VoteButton color="red" />
    <VoteButton color="blue" />
    <VoteButton color="green" />
    <VoteButton color="purple" />
  </div>
</main>
