<script>
  import "./app.css";
  import { onMount } from "svelte"
  import { Button, GradientButton } from 'flowbite-svelte';
  
  let counters = { red: 0, green: 0, blue: 0, purple: 0 }
  const color_order = ["red", "green", "blue", "purple"]
  
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
      <GradientButton shadow color={color} on:click={() => increment(color)}>
        {color} ({count})
      </GradientButton>
      <!-- <button on:click={() => increment(color)}>
        {color} ({count})
      </button> -->
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
