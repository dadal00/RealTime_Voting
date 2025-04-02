<script>
  import * as d3 from 'd3'
  import { onMount, onDestroy } from 'svelte'

  const { data } = $props()

  let width = 1000
  let height = 625
  let delay = 200
  let svg
  let container
  let resizeObserver
  let outer_padding = 0.01

  function calculateDimensions() {
    if (!container) return

    const containerRect = container.getBoundingClientRect()

    let factor

    switch (true) {
      case containerRect.height < 300:
        factor = 0.7
        outer_padding = 0.02
        break
      default:
        factor = 0.83
    }

    width = containerRect.width
    height = containerRect.height * 0.9075 * factor

    if (svg) {
      svg?.remove()
      chart_init()
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
    if (num >= 1000000000) {
      return (num / 1000000000).toFixed(1) + 'B'
    } else if (num >= 1000000) {
      return (num / 1000000).toFixed(1) + 'M'
    } else if (num >= 1000) {
      return (num / 1000).toFixed(1) + 'K'
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
    if (!svg) return

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

    newBars
      .append('rect')
      .attr('height', yScale.bandwidth())
      .attr('fill', (d) => `${colors[d.color]}`)
      .transition()
      .duration(delay)
      .attr('width', (d) => Math.max(50, xScale(d.count)))
      .attr('stroke', '#5e5757')
      .attr('stroke-width', '2')
      .attr('rx', 11)
      .attr('ry', 11)

    newBars
      .append('text')
      .attr('class', 'value-label')
      .attr('x', (d) => xScale(d.count) - 8)
      .attr('y', -12)
      .attr('dy', '0.35em')
      .style('font-size', 'max(3.4vh, 1.4vw, 1rem)')
      .style('font-family', 'Verdana, Geneva, sans-serif')
      // .style('font-weight', '400')
      .style('fill', 'white')
      .style('text-anchor', 'end')
      .attr('y', yScale.bandwidth() / 2)
      .text((d) => format_number(d.count))

    bars
      .transition()
      .duration(delay)
      .attr('transform', (d) => `translate(1,${yScale(d.color)})`)

    bars
      .select('rect')
      .transition()
      .duration(delay)
      .attr('width', (d) => Math.max(100, xScale(d.count)))
      .attr('stroke', '#5e5757')
      .attr('stroke-width', '2')
      .attr('rx', 11)
      .attr('ry', 11)

    bars
      .select('.value-label')
      .transition()
      .duration(delay)
      .attr('x', (d) => Math.max(100, xScale(d.count)) - 20)
      .text((d) => format_number(d.count))
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
    if (svg && data) {
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

<div class="chart-container" bind:this={container}>
  <div id="chart"></div>
</div>
