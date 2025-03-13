<script>
  import "./app.css";
  import { onMount } from "svelte"
  
  let counters = { red: 0, green: 0, blue: 0, purple: 0 }
  const color_order = ["red", "green", "blue", "purple"]
  const color_style = { red: "from-red-400 via-red-500 to-red-600", green: "from-green-400 via-green-500 to-green-600", blue: "from-blue-500 via-blue-600 to-blue-700", purple: "from-purple-500 via-purple-600 to-purple-700" }
  
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
        class="text-white bg-gradient-to-r {color_style[color]} hover:bg-gradient-to-br focus:ring-4 focus:outline-none focus:ring-{color}-300 dark:focus:ring-{color}-800 shadow-lg shadow-{color}-500/50 dark:shadow-lg dark:shadow-{color}-800/80 font-medium rounded-lg text-sm px-5 py-2.5 text-center me-2 mb-2"
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
