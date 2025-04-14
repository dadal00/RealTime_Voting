<script>
  import * as d3 from 'd3'
  import { onMount, onDestroy } from 'svelte'
  import { websocket } from '$lib/stores/websocket'
  import VisibilityChange from 'svelte-visibility-change'

  const { labels } = $props()
  const data = $derived(
    Object.entries($websocket)
      .filter(([key]) => key !== 'total' && key != 'total_users')
      .map(([color, count]) => ({ color, count }))
  )
  // const data = [
  //   {color:"blue", count:999999999},
  //   {color:"red", count:999999},
  //   {color:"green", count:999999},
  //   {color:"purple", count:999999},
  // ]

  let width = 1000
  let height = 625
  let delay = 200
  let svg
  let container
  let resizeObserver
  let outer_padding = 0.01
  let minBarWidth = 200
  let visible = $state(true)
  let mobile = $state(false)
  let mobile_changed = $state(false)

  function calculateDimensions() {
    if (!container) return
    const containerRect = container.getBoundingClientRect()

    let factor = (() => {
      switch(true) {
        case containerRect.height < 245:
          return 0.6
        case containerRect.height < 300:
          return 0.7
        default:
          return 0.83
      }
    })()
    outer_padding = containerRect.height < 430 ? 0.02 : 0.01
    minBarWidth = (() => {
      switch (true) {
        case containerRect.width < 445:
          mobile_changed = mobile ? false : true
          mobile = true
          if (containerRect.height < 620) {
            return 125
          }
          return 150
        case containerRect.height < 450:
          mobile_changed = mobile ? true : false
          mobile = false
          switch (true) {
            case containerRect.width < 1200:
              return 205
            default:
              return 240
          }
        default:
          if (containerRect.width < 1200) {
            mobile_changed = mobile ? false : true
            mobile = true
            if (containerRect.height < 650) {
              return 125
            }
            return 180
          }
          mobile_changed = mobile ? true : false
          mobile = false
          return 320
      }
    })()
    width = containerRect.width
    height = containerRect.height * 0.9075 * factor

    if (svg) {
      svg.attr('viewBox', [0, 0, width, height]).attr('width', width).attr('height', height)
      update_chart()
    }
  }

  const colors = {
    red: '#d95b5b',
    green: '#6cd859',
    blue: '#5b98d9',
    purple: '#d064dd',
  }

  function format_number(num) {
    if (num >= 1_000_000_000) {
      return (num / 1_000_000_000).toFixed(1) + 'B'
    } else if (num >= 1_000_000) {
      return (num / 1_000_000).toFixed(1) + 'M'
    } else if (num >= 1_000) {
      return (num / 1_000).toFixed(1) + 'K'
    }
    return num.toString()
  }

  function chart_init() {
    svg = d3
      .select('#chart')
      .append('svg')
      .attr('viewBox', [0, 0, width, height])
      .attr('width', '100%')
      .attr('height', '100%')
  }

  function update_chart() {
    if (!svg || !visible) return

    data.sort((a, b) => b.count - a.count)

    const xScale = d3
      .scaleLinear()
      .domain([0, d3.max(data, (d) => d.count) * 1.1])
      .range([0, width])

    const yScale = d3
      .scaleBand()
      .domain(data.map((d) => d.color))
      .range([0, height])
      .paddingInner(0.35)
      .paddingOuter(outer_padding)

    const bars = svg.selectAll('.bar').data(data, (d) => d.color)

    bars.exit().transition().duration(delay).attr('width', 0).remove()

    const newBars = bars
      .enter()
      .append('g')
      .attr('class', 'bar')
      .attr('transform', (d) => `translate(0,${yScale(d.color)})`)
      .attr('opacity', 0)

    newBars
      .append('rect')
      .attr('height', yScale.bandwidth())
      .attr('fill', (d) => `${colors[d.color]}`)
      .attr('stroke', '#5e5757')
      .attr('stroke-width', '2')
      .attr('rx', 11)
      .attr('ry', 11)

    newBars
      .append('text')
      .attr('class', 'value-label')
      .style('font-size', mobile ? 'max(2.6vh, 0.7vw, 0.5rem)' : 'max(3.4vh, 1.4vw, 1rem)')
      .style('font-family', 'Verdana, Geneva, sans-serif')
      .style('fill', 'white')
      .style('text-anchor', mobile ? 'beginning' : 'end')

    newBars
      .append('text')
      .attr('class', 'name-label')
      .style('font-size', mobile ? 'max(2.6vh, 0.7vw, 0.5rem)' : 'max(3.4vh, 1.4vw, 1rem)')
      .style('font-family', 'Verdana, Geneva, sans-serif')
      .style('fill', 'white')
      .style('text-anchor', 'beginning')

    const merged = newBars.merge(bars)

    merged
      .transition()
      .duration(delay)
      .attr('transform', (d) => `translate(1,${yScale(d.color)})`)
      .attr('opacity', 1)

    merged
      .select('rect')
      .transition()
      .duration(delay)
      .attr('width', (d) => Math.max(minBarWidth, xScale(d.count)))
      .attr('height', yScale.bandwidth())

    merged
      .select('.value-label')
      .transition()
      .duration(delay)
      .attr('x', mobile ? 20 : (d) => Math.max(minBarWidth, xScale(d.count)) - 20)
      .attr('y', mobile ? yScale.bandwidth() / 2 + 20 : yScale.bandwidth() / 2)
      .attr('dy', '0.35em')
      .text((d) => format_number(d.count))

    merged
      .select('.name-label')
      .transition()
      .duration(delay)
      .attr('x', 20)
      .attr('y', mobile ? yScale.bandwidth() / 2 - 20 : yScale.bandwidth() / 2)
      .attr('dy', '0.35em')
      .text((d) => labels[d.color])
  }

  onMount(() => {
    container = document.querySelector('.chart-container')
    calculateDimensions()

    chart_init()

    update_chart()

    resizeObserver = new ResizeObserver(() => {
      calculateDimensions()
    })

    if (container) {
      resizeObserver.observe(container)
    }

    window.addEventListener('resize', calculateDimensions)
  })

  $effect(() => {
    if (visible && mobile_changed) {
      svg?.remove()
      chart_init()
      update_chart()
    } else if (visible && svg && data) {
      update_chart()
    }
  })

  onDestroy(() => {
    if (resizeObserver) {
      resizeObserver.disconnect()
      window.removeEventListener('resize', calculateDimensions)
    }
    svg?.remove()
  })
</script>

<style>
  .chart-container {
    width: 100%;
    padding-left: 3vw;
    flex: 1;
    height: 100%;
    overflow: visible;
  }
</style>

<VisibilityChange on:visible={() => (visible = true)} on:hidden={() => (visible = false)} />
<div class="chart-container" bind:this={container}>
  <div id="chart"></div>
</div>
