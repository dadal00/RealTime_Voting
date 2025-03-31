<script>
  import { onMount } from 'svelte'
  import { websocket } from '$lib/stores/websocket'
  import BarChart from '$lib/components/BarChart.svelte'
  import VoteButton from '$lib/components/VoteButton.svelte'
  import TotalVotes from '$lib/components/TotalVotes.svelte'

  const chartData = $derived(
    Object.entries($websocket)
      .filter(([key]) => key !== 'total')
      .map(([color, count]) => ({ color, count }))
  )

  onMount(() => {
    websocket.connect()
    return () => {}
  })
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
    height: 16.5vh; 
    width: 100%;
    background-color: #F5F0E6;
    border-bottom: 1px solid rgb(188, 185, 178);
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 2.5rem;
  }

  .body-container {
    background-color: #FAF4EE;
    flex: 1;
  }

</style>

<main class="page-container">
  <div class="header-container">
    <!-- <VoteButton color="red" />
    <VoteButton color="blue" />
    <VoteButton color="green" />
    <VoteButton color="purple" /> -->
    <p>Hello</p>
    <p>Hello</p>
    <p>Hello</p>
    <p>Hello</p>
  </div>
  <div class="body-container">
    <p>Hello</p>
    <p>Hello</p>
    <!-- <TotalVotes large={true} /> -->
    
    <!-- <BarChart data={chartData} /> -->
  </div>
</main>
