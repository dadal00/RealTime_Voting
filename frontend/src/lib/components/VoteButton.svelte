<script>
  import { websocket } from '$lib/stores/websocket'

  export let color

  let animations = []
  let container

  const handleClick = (event) => {
    websocket.sendVote(color)

    const rect = container.getBoundingClientRect()
    const id = Date.now()

    animations = [
      ...animations,
      {
        id,
        x: event.clientX - rect.left - 20 + (Math.random() * 6 - 3),
        y: event.clientY - rect.top - 20 + (Math.random() * 6 - 3),
      },
    ]
  }
</script>

<style>
  .red-button {
    background-color: #d95b5b;
  }

  .blue-button {
    background-color: #5b98d9;
  }

  .green-button {
    background-color: #6cd859;
  }

  .purple-button {
    background-color: #d064dd;
  }

  .button-container {
    position: relative;
    width: 13.4vw;
    max-width: 7.5rem;
    max-height: 8.7vh;
    aspect-ratio: 2.3;
    display: flex;
    z-index: 0;
  }

  .button-background {
    top: 0;
    left: 0;
    right: 0;
    position: absolute;
    height: 100%;
    z-index: 1;
    border: 2px solid #5e5757;
    border-radius: 11px;
  }

  button {
    width: 100%;
    height: 90%;
    border: none;
    border: 2px solid #5e5757;
    border-radius: 11px;
    z-index: 2;
  }

  .button-container:hover button {
    transform: translateY(-2px);
  }

  .button-container:active .button-background {
    top: 2px;
    height: calc(100% - 2px);
  }

  .button-container:active button {
    transform: translateY(2px);
  }

  .click-animation {
    position: absolute;
    font-weight: bold;
    font-size: 1rem;
    animation: fly-animation 2s linear;
    color: #5e5757;
    z-index: 3;
  }

  @keyframes fly-animation {
    0% {
      transform: translate(0, 0);
      opacity: 0.7;
    }
    50% {
      transform: translate(0, -10vh);
      opacity: 0.35;
    }
    100% {
      transform: translate(0, -20vh);
      opacity: 0;
    }
  }
</style>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<div
  class="button-container"
  bind:this={container}
  on:click={handleClick}
  role="button"
  tabindex="0"
  aria-label="{color} button"
>
  <div class="button-background {color}-button"></div>
  <button class="{color}-button" aria-label="{color} button"></button>

  {#each animations as animation (animation.id)}
    <span
      class="click-animation"
      style="left: {animation.x}px; top: {animation.y}px;"
      on:animationend={() => (animations = animations.filter((a) => a.id !== animation.id))}
    >
      +1
    </span>
  {/each}
</div>
