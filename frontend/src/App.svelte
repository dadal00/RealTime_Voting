<script>
  import "./app.css"
  import { onDestroy, onMount } from "svelte"
  import * as d3 from 'd3'

  export let width = 800
  export let height = 500
  export let margin = { top: 50, right: 120, bottom: 50, left: 150 }
  export let delay = 200
  export let eventSourceUrl = 'http://localhost:8080/updates'

  let counters = { red: 0, green: 0, blue: 0, purple: 0 }
  let data = []
  let svg
  let eventSource
  let latestTimestamp

  const color_order = ["red", "green", "blue", "purple"]
  const chartWidth = width - margin.left - margin.right
  const chartHeight = height - margin.top - margin.bottom
  const gradients = {
    red: { start: '#ff6b6b', end: '#ff4757' },
    green: { start: '#4CAF50', end: '#388E3C' },
    blue: { start: '#2196F3', end: '#1976D2' },
    purple: { start: '#9C27B0', end: '#7B1FA2' }
  };
  
  async function increment(color) {
    try {
      await fetch(`http://localhost:8080/increment/${color}`, { method: "POST" })
    } catch (error) {
      console.error("Error - (Svelte)increment - Failed to Increment:", error)
    }
  }

  function initChart() {

    svg = d3.select("#chart")
      .append("svg")
      .attr("viewBox", [0, 0, width, height])
      .attr("width", width)
      .attr("height", height)
      .attr("style", "max-width: 100%; height: auto;")

    color_order.forEach(color => {
      const gradient = (svg.append("defs")).append("linearGradient")
        .attr("id", `gradient-${color}`)
        .attr("x1", "0%")
        .attr("y1", "0%")
        .attr("x2", "100%")
        .attr("y2", "0%");

      gradient.append("stop")
        .attr("offset", "0%")
        .attr("stop-color", gradients[color].start);

      gradient.append("stop")
        .attr("offset", "100%")
        .attr("stop-color", gradients[color].end);
    });

    startLiveUpdates()
  }

  function startLiveUpdates() {
    
    try {
      eventSource = new EventSource(eventSourceUrl)
      
      eventSource.onmessage = (e) => {
        try {
          counters = JSON.parse(e.data)
          
          latestTimestamp = new Date()
          
          data = Object.entries(counters).map(([color, count]) => (
            {
             color, 
             count, 
             timestamp: latestTimestamp 
            }))
          
          updateChart()
        } catch (parseError) {
          console.error("Error - (Svelte)updates - Failed to Parse Updates:", parseError)
        }
      }
      
      eventSource.onerror = (error) => {
        console.error("Error - (Svelte)updates - Failed to Fetch Updates:", error)
      }
      
      console.log("EventSource connection established")
    } catch (error) {
      console.error("Error - (Svelte)updates - Failed to Fetch Updates:", error)
    }
  }


  function updateChart() {
    
    data.sort((a, b) => b.count - a.count)
    
    const xScale = d3.scaleLinear()
      .domain([0, d3.max(data, d => d.count) * 1.1])
      .range([0, chartWidth])
    
    const yScale = d3.scaleBand()
      .domain(data.map(d => d.color))
      .range([0, chartHeight])
      .padding(0.1)
    
    const bars = svg.selectAll(".bar")
      .data(data, d => d.color)
    
    bars.exit()
      .transition()
      .duration(delay)
      .attr("width", 0)
      .remove()
    
    const newBars = bars.enter()
      .append("g")
      .attr("class", "bar")
      .attr("transform", d => `translate(${margin.left},${margin.top + yScale(d.color)})`)
    
    newBars.append("rect")
      .attr("height", yScale.bandwidth())
      .attr("width", 0)
      .attr("fill", d => `url(#gradient-${d.color})`)
      .transition()
      .duration(delay)
      .attr("width", d => xScale(d.count));
    
    newBars.append("text")
      .attr("class", "value-label")
      .attr("x", d => xScale(d.count) + 5)
      .attr("y", yScale.bandwidth() / 2)
      .attr("dy", "0.35em")
      .style("font-size", "12px")
      .text(d => d.count.toLocaleString())
    
    bars.transition()
      .duration(delay)
      .attr("transform", d => `translate(${margin.left},${margin.top + yScale(d.color)})`)
    
    bars.select("rect")
      .transition()
      .duration(delay)
      .attr("width", d => xScale(d.count))
    
    bars.select(".value-label")
      .transition()
      .duration(delay)
      .attr("x", d => xScale(d.count) + 5)
      .text(d => d.count.toLocaleString())
  }

  onMount(async () => {
    try {
      const response = await fetch("http://localhost:8080/counters")
      counters = await response.json()

      data = Object.entries(counters).map(([color, count]) => ({
        category: color,
        value: count,
        timestamp: new Date()
      }))

      initChart()
    } catch (error) {
      console.error("Error - (Svelte)onMount - Failed to Fetch Counters:", error)
    }
  })

  onDestroy(() => {
    if (eventSource) {
      eventSource.close()
    }
  })
</script>

<div class="chart-container">
  <div id="chart"></div>
</div>

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
  .chart-container {
    width: 100%;
    height: 100%;
    display: flex;
    justify-content: center;
    align-items: center;
  }
  
  .buttons {
    position: fixed;
    top: 2rem;
    left: 50%;
    transform: translateX(-50%);
    display: flex;
    gap: 1rem;
    padding: 0;
  }
  
  button {
    padding: 1rem;
    font-size: 1.2rem;
    border: 2px solid black;
    border-radius: 4px;
    font-weight: bold;
  }
  
  :global(.bar:hover rect) {
    opacity: 0.8;
  }
  
  :global(.value-label) {
    font-family: sans-serif;
    fill: #333;
  }
</style>
