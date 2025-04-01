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
  }

  .header-container {
    min-height: 4rem;
    height: 12.5vh;
    background-color: #f5f0e6;
    border-top: 1px solid rgb(188, 185, 178);
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 2.5rem;
  }

  .body-container {
    background-color: #faf4ee;
    flex: 1;
  }
</style>

<main class="page-container">
  <div class="body-container">
    <p>Hello</p>
    <p>Hello</p>
    <!-- <TotalVotes large={true} /> -->

    <!-- <BarChart data={chartData} /> -->
  </div>
  <div class="header-container">
    <VoteButton color="red" />
    <VoteButton color="blue" />
    <VoteButton color="green" />
    <VoteButton color="purple" />
  </div>
</main>
