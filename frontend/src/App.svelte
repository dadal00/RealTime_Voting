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
  let tickerText = ''
  let latestTimestamp

  const color_order = ["red", "green", "blue", "purple"]
  const chartWidth = width - margin.left - margin.right
  const chartHeight = height - margin.top - margin.bottom
  const colorScale = d3.scaleOrdinal(d3.schemeCategory10)
  
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
      .attr("width", width)
      .attr("height", height)

    const g = svg.append("g")
      .attr("transform", `translate(${margin.left},${margin.top})`)

    g.append("g")
      .attr("class", "x-axis")
      .attr("transform", `translate(0,${chartHeight})`)

    g.append("g")
      .attr("class", "y-axis")

    svg.append("text")
      .attr("class", "ticker")
      .attr("x", margin.left)
      .attr("y", margin.top / 2)
      .attr("text-anchor", "start")
      .style("font-size", "16px")
      .text(tickerText)

    startLiveUpdates()
  }

  function startLiveUpdates() {
    console.log("Starting live updates")
    tickerText = "Live updates: waiting for data..."
    
    try {
      eventSource = new EventSource(eventSourceUrl)
      
      eventSource.onmessage = (e) => {
        try {
          counters = JSON.parse(e.data)
          
          latestTimestamp = new Date()
          tickerText = `Live updates: ${latestTimestamp.toLocaleString()}`
          
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
        tickerText = "Live updates: connection error"
      }
      
      console.log("EventSource connection established")
      tickerText = "Live updates: connected"
    } catch (error) {
      console.error("Error - (Svelte)updates - Failed to Fetch Updates:", error)
      tickerText = "Live updates: failed to connect"
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
    
    svg.select(".x-axis")
      .transition()
      .duration(delay)
      .call(d3.axisBottom(xScale))
    
    svg.select(".y-axis")
      .transition()
      .duration(delay)
      .call(d3.axisLeft(yScale))
    
    svg.select(".ticker")
      .text(tickerText)
    
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
      .attr("fill", d => colorScale(d.color))
      .transition()
      .duration(delay)
      .attr("width", d => xScale(d.count))
    
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
      .attr("fill", d => colorScale(d.color))
    
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
