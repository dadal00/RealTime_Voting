<script>
    /*
      Imports
    */
    import { onDestroy, onMount } from "svelte"
    import * as d3 from 'd3'
    import { PUBLIC_GO_URL, PUBLIC_WS_URL } from '$env/static/public'
  
    /*
      Non-constant Variables
    */
    let margin = { top: 50, right: 0, bottom: 50, left: 25 }
    /**
     * @type {any[]}
     */
    let data = []
    /**
     * @type {any[]}
     */
    let click_animations = []
    let width = 900
    let height = 600
    let delay = 200
    let total_votes = 0;
    let concurrent_users = 0
    let chartWidth = width - margin.left - margin.right
    let chartHeight = height - margin.top - margin.bottom
    /**
     * @type {any[]}
     */
    let svg
    /**
     * @type {WebSocket}
     */
    let socket
    let connectionStatus = "connecting"
    
    /*
      Constant Variables
    */
    const gradients = {
      //Current Ordering 
      //start: Dark, end: Light
      //red dark #be2047
      //green dark #04773B
      //blue dark #2081C3
      //purple dark #8e04e0
      red: { start: '#be2047', end: '#FF7477' },
      green: { start: '#04773B', end: '#85f9a2' },
      blue: { start: '#2081C3', end: '#74d7ff' },
      purple: { start: '#8e04e0', end: '#9175bd' }
    };
    const color_order = ["red", "green", "blue", "purple"]
    const websocket_url = PUBLIC_WS_URL || "ws://localhost:8080/api/ws"
    const go_url = PUBLIC_GO_URL || "http://localhost:8080/api"

    // const websocket_url = "ws://localhost:8080/api/ws"
    // const go_url = "http://localhost:8080/api"
  
    $: chartWidth = width - margin.left - margin.right;
    $: chartHeight = height - margin.top - margin.bottom;
  
    /*
      WebSocket Functions
    */
    function connectWebSocket() {
      socket = new WebSocket(websocket_url)
      socket.binaryType = "arraybuffer"
      
      socket.onopen = () => {
        console.log("WebSocket connection established")
        connectionStatus = "connected"
      }
      
      socket.onmessage = (event) => {
        try {
          const message = JSON.parse(event.data)
          
          if (message.type === "users") {
              concurrent_users = message.count
          } else {
              data = Object.entries(message)
              .filter(([color]) => color !== "total")
              .map(([color, count]) => ({
                  "color": color,
                  "count": count
              }))
  
              total_votes = message.total || total_votes;
              update_chart()
          }
        } catch (error) {
          console.error("Error parsing WebSocket data:", error)
        }
      }
      
      socket.onclose = () => {
        console.log("WebSocket connection closed")
        connectionStatus = "disconnected"
        setTimeout(connectWebSocket, 3000)
      }
      
      socket.onerror = (error) => {
        console.error("WebSocket error:", error)
        connectionStatus = "error"
      }
    }
  
    /*
      Click Animation Functions
    */
    /**
     * @param {MouseEvent & { currentTarget: EventTarget & HTMLButtonElement; }} event
     * @param {string} color
     */
    function create_click_animation(event, color) {
      const rect = event.currentTarget.getBoundingClientRect();
      const id = Date.now();
      
      click_animations = [...click_animations, {
        id,
        x: event.clientX - rect.left - 20 + (Math.random() * 6 - 3),
        y: event.clientY - rect.top - 20 + (Math.random() * 6 - 3),
        color: color
      }];
    }
  
    /*
      Counter Functions
    */
    /**
     * @param {string} color
     */
    async function increment(color) {
      try {
        await fetch(`${go_url}/increment/${color}`, { method: "POST" })
      } catch (error) {
        console.error("Error - (Svelte)increment - Failed to Increment:", error)
      }
    }
  
    /*
      Bar Graph Functions
    */
    function chart_init() {
      svg = d3.select("#chart")
        .append("svg")
        .attr("viewBox", [0, 0, width, height])
        .attr("width", '100%')
        .attr("height", height)
        .attr("style", "max-width: 100%; height: auto; left:0;")
      
      svg.append("g")
        .attr("class", "headers")
        .attr("transform", `translate(${margin.left},${margin.top - 20})`);
  
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
    }
  
    function update_chart() {
      if (!svg) return;
  
      data.sort((a, b) => b.count - a.count)
  
      const xScale = d3.scaleLinear()
        .domain([0, d3.max(data, d => d.count) * 1.1])
        .range([0, chartWidth])
  
      const yScale = d3.scaleBand()
        .domain(data.map(d => d.color))
        .range([0, chartHeight])
        .padding(0.5)
  
      /*
        Bars
      */
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
        .attr("width", d => xScale(d.count))
  
      newBars.append("text")
        .attr("class", "value-label")
        .attr("x", chartWidth - 5)
        .attr("y", -12)
        .attr("dy", "0.35em")
        .style("font-size", "12px")
        .style("fill", "white")
        .style("text-anchor", "end")
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
        .attr("x", chartWidth - 5)
        .text(d => d.count.toLocaleString())
        .style("text-anchor", "end")
    }
  
    /*
      Startup
    */
    onMount(async () => {
      try {
        chart_init()
        connectWebSocket()
        update_chart()
      } catch (error) {
        console.error("Error - (Svelte)onMount - Failed to Fetch Counters:", error)
      }
    })
  
    /*
      Clean Up
    */
    onDestroy(() => {
      if (socket && socket.readyState === WebSocket.OPEN) {
        socket.close()
      }
    })
  </script>
  
  <main>
    <div style="background-color:grey; width: 100vw; height: 100vh; position: fixed; top: 0; left: 0; overflow: hidden;">
        <div class="total-votes">
          <span class="total-number">{total_votes.toLocaleString()}</span> votes
        </div>
        <div class="connection-status" class:connected={connectionStatus === "connected"} class:disconnected={connectionStatus === "disconnected"} class:error={connectionStatus === "error"}>
          {connectionStatus}
        </div>
        <div class="user-count" class:connected={connectionStatus === "connected"}>
          Online: {concurrent_users}
        </div>
        <div class="chart-container">
          <div id="chart"></div>
        </div>
        <div class="buttons">
          {#each color_order as color}
            <div class="button-wrapper">
              <button 
                on:click={(e) => 
                {
                  increment(color);
                  create_click_animation(e, color);
                }}
              >
                {color}
              </button>
              {#each click_animations as animation (animation.id)}
                  {#if animation.color === color}
                    <span
                      class="click-animation"
                      style="left: {animation.x}px; top: {animation.y}px; color: gold"
                      on:animationend={() => click_animations = click_animations.filter(a => a.id !== animation.id)}
                    >
                      +1
                    </span>
                  {/if}
                {/each}
            </div>
          {/each}
        </div>
    </div>
  </main>
  
  <style>
    .total-votes {
      position: absolute;
      top: 3rem;
      left: 50%;
      transform: translateX(-50%);
      font-size: 1.25rem;
      font-weight: bold;
      color: white;
      text-shadow: 0 2px 4px rgba(0, 0, 0, 0.5);
      background: rgba(0, 0, 0, 0.7);
      padding: 0.5rem 1rem;
      border-radius: 0.5rem;
      white-space: nowrap;
    }
  
    .total-number {
      color: #FFD700;
      margin-left: 0.5rem;
    }
    .user-count {
      position: absolute;
      top: 10px;
      left: 10px;
      padding: 5px 10px;
      border-radius: 5px;
      background-color: rgba(0, 0, 0, 0.7);
      color: white;
      font-size: 14px;
      transition: opacity 0.3s;
    }
  
    .user-count.connected {
      background-color: #4CAF50;
    }
    .connection-status {
      position: absolute;
      top: 10px;
      right: 10px;
      padding: 5px 10px;
      border-radius: 5px;
      font-size: 12px;
      color: white;
      background-color: #555;
    }
  
    .connected {
      background-color: #4CAF50;
    }
  
    .disconnected {
      background-color: #FF9800;
    }
  
    .error {
      background-color: #F44336;
    }
  
    .click-animation {
      position: absolute;
      pointer-events: none;
      font-weight: bold;
      font-size: 1rem;
      text-shadow: 0 2px 4px rgba(0, 0, 0, 0.2);
      animation: fly-animation 5s linear;
    }
  
    @keyframes fly-animation {
      0% {  
        transform: translate(0, 0);
        opacity: 0.7;
      }
      50% {
        transform: translate(0px, -200px);
        opacity: 0.35;
      }
      100% {
        transform: translate(0px, -400px);
        opacity: 0;
      }
    }
  
    .glass-container {
      position: fixed;
      top: 50%;
      left: 50%;
      transform: translate(-50%, -50%);
      background: transparent;
      backdrop-filter: blur(2px);
      border-radius: 20px;
      border: 1px solid rgba(255, 255, 255, 0.1);
      padding: 1rem;
      box-shadow: 0 8px 32px 0 #1f26875e;
      width: 90%;
      height: 90%;
      overflow-y: auto;
    }
  
    .chart-container {
      width: 100%;
      height: 100%;
      display: flex;
      justify-content: center;
      align-items: center;
      position: relative;
    }
  
    .buttons {
      position: absolute;
      bottom: 2rem;
      left: 50%;
      transform: translateX(-50%);
      display: flex;
      gap: 1rem;
      padding: 0;
    }
  
    .button-wrapper {
      position: relative;
      margin: 0 0.5rem;
    }
  
    .button-background {
      position: absolute;
      top: 0;
      left: 0;
      right: 0;
      height: calc(100% + 3px);
      border-radius: 0.35rem;
      z-index: -1;
    }
    
    button {
      padding-left: 1rem;
      font-size: 0.75rem;
      border-radius: 0.35rem;
      font-weight: bold;
      text-transform: capitalize;
      display: flex;
      justify-content: center; 
      align-items: center;
      border: 2px solid transparent;
      background-clip: padding-box;
      background-color: black;
    }
  
    button:hover {
      transform: translateY(-2px);
      outline: none;
    }
  
    .button-wrapper:hover .button-background {
      top:-2px;
      height: calc(100% + 5px);
    }
  
    .button-wrapper:active .button-background {
      top:0px;
      height: calc(100% + 3px);
    }
  
    button:focus {
      outline: none;
    }
  
    button:active {
      transform: translateY(1px);
    }
    
    :global(.bar:hover rect) {
      opacity: 0.8;
    }
  
  </style>
  