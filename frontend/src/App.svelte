<script>
  import { onMount } from "svelte"

  let counters = { red: 0, green: 0, blue: 0, yellow: 0 }
  const color_order = ["red", "green", "blue", "yellow"]

  onMount(async () => {
    const response = await fetch("http://localhost:8080/counters")
    counters = await response.json()
  })

  const eventSource = new EventSource("http://localhost:8080/updates")
  eventSource.onmessage = (e) => {
    counters = JSON.parse(e.data)
  };

  async function increment(color) {
    await fetch(`http://localhost:8080/increment/${color}`, { method: "POST" })
  }
</script>

<main>
  <div class="buttons">
    {#each color_order.map(color => [color, counters[color]]) as [color, count]}
      <button on:click={() => increment(color)}>
        {color} ({count})
      </button>
    {/each}
  </div>
</main>

<style>
  .buttons {
    display: flex;
    gap: 1rem;
    padding: 2rem;
  }
  button {
    padding: 1rem;
    font-size: 1.2rem;
  }
</style>
