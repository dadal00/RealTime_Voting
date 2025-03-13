<script>
  import "./app.css";
  import { onMount } from "svelte"
  
  let counters = { red: 0, green: 0, blue: 0, purple: 0 }
  const color_order = ["red", "green", "blue", "purple"]
  
  onMount(async () => {
    try {
      const response = await fetch("http://localhost:8080/counters")
      counters = await response.json()
    } catch (error) {
      console.error("Error - (Svelte)onMount - Failed to Fetch Counters:", error);
    }
  })
  
  try {
    const eventSource = new EventSource("http://localhost:8080/updates")
    eventSource.onmessage = (e) => {
      try {
        counters = JSON.parse(e.data);
      } catch (parseError) {
        console.error("Error - (Svelte)updates - Failed to Parse Updates:", parseError);
      }
    };
    eventSource.onerror = (error) => {
      console.error("Error - (Svelte)updates - Failed to Fetch Updates:", error);
    };
  } catch (error) {
    console.error("Error - (Svelte)updates - Failed to Fetch Updates:", error);
  }
  
  async function increment(color) {
    try {
      await fetch(`http://localhost:8080/increment/${color}`, { method: "POST" })
    } catch (error) {
      console.error("Error - (Svelte)increment - Failed to Increment:", error);
    }
  }
</script>

<main>
  <div class="buttons">
    {#each color_order.map(color => [color, counters[color]]) as [color, count]}
      <button 
        
        on:click={() => increment(color)}
      >
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
