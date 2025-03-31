<script>
  import * as d3 from 'd3'
  import { onMount } from 'svelte'

  const { data } = $props()

  let filteredData = $derived(
    data.filter((/** @type {{ color: string; }} */ item) => item.color !== 'total')
  )

  let margin = { top: 50, right: 0, bottom: 50, left: 25 }
  let width = 900
  let height = 600
  let delay = 200
  let chartWidth = width - margin.left - margin.right
  let chartHeight = height - margin.top - margin.bottom
  let svg
  const color_order = ['red', 'green', 'blue', 'purple']
  const gradients = {
    red: { start: '#be2047', end: '#FF7477' },
    green: { start: '#04773B', end: '#85f9a2' },
    blue: { start: '#2081C3', end: '#74d7ff' },
    purple: { start: '#8e04e0', end: '#9175bd' },
  }

  function chart_init() {
    svg = d3
      .select('#chart')
      .append('svg')
      .attr('viewBox', [0, 0, width, height])
      .attr('width', '100%')
      .attr('height', height)
      .attr('style', 'max-width: 100%; height: auto; left:0;')

    svg
      .append('g')
      .attr('class', 'headers')
      .attr('transform', `translate(${margin.left},${margin.top - 20})`)

    color_order.forEach((color) => {
      const gradient = svg
        .append('defs')
        .append('linearGradient')
        .attr('id', `gradient-${color}`)
        .attr('x1', '0%')
        .attr('y1', '0%')
        .attr('x2', '100%')
        .attr('y2', '0%')
      gradient.append('stop').attr('offset', '0%').attr('stop-color', gradients[color].start)
      gradient.append('stop').attr('offset', '100%').attr('stop-color', gradients[color].end)
    })
  }

  function update_chart() {
    if (!svg) return

    filteredData.sort((a, b) => b.count - a.count)

    const xScale = d3
      .scaleLinear()
      .domain([0, d3.max(filteredData, (d) => d.count) * 1.1])
      .range([0, chartWidth])

    const yScale = d3
      .scaleBand()
      .domain(filteredData.map((d) => d.color))
      .range([0, chartHeight])
      .padding(0.5)

    const bars = svg.selectAll('.bar').data(filteredData, (d) => d.color)

    bars.exit().transition().duration(delay).attr('width', 0).remove()

    const newBars = bars
      .enter()
      .append('g')
      .attr('class', 'bar')
      .attr('transform', (d) => `translate(${margin.left},${margin.top + yScale(d.color)})`)

    newBars
      .append('rect')
      .attr('height', yScale.bandwidth())
      .attr('width', 0)
      .attr('fill', (d) => `url(#gradient-${d.color})`)
      .transition()
      .duration(delay)
      .attr('width', (d) => xScale(d.count))

    newBars
      .append('text')
      .attr('class', 'value-label')
      .attr('x', chartWidth - 5)
      .attr('y', -12)
      .attr('dy', '0.35em')
      .style('font-size', '12px')
      .style('fill', 'white')
      .style('text-anchor', 'end')
      .text((d) => d.count.toLocaleString())

    bars
      .transition()
      .duration(delay)
      .attr('transform', (d) => `translate(${margin.left},${margin.top + yScale(d.color)})`)

    bars
      .select('rect')
      .transition()
      .duration(delay)
      .attr('width', (d) => xScale(d.count))

    bars
      .select('.value-label')
      .transition()
      .duration(delay)
      .attr('x', chartWidth - 5)
      .text((d) => d.count.toLocaleString())
      .style('text-anchor', 'end')
  }

  onMount(() => {
    chart_init()
    update_chart()

    return () => {
      if (svg) {
        svg.remove()
        svg = null
      }
    }
  })

  $effect(() => {
    if (svg && filteredData) {
      update_chart()
    }
  })
</script>

<style>
  .chart-container {
    width: 100%;
    padding: 1rem;
    background-color: #f7fafc;
    border-radius: 0.5rem;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
  }
</style>

<div class="chart-container">
  <div id="chart"></div>
</div>
