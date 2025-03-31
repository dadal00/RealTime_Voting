<script>
  import { websocket } from '$lib/stores/websocket'

  export let color

  const handleVote = () => {
    websocket.sendVote(color)
  }

  const getButtonClass = (buttonColor) => {
    const baseClasses = 'vote-button py-2 px-4 rounded-md font-medium text-white'
    const colorClasses = {
      red: 'bg-red-500 hover:bg-red-600',
      blue: 'bg-blue-500 hover:bg-blue-600',
      green: 'bg-green-500 hover:bg-green-600',
      purple: 'bg-purple-500 hover:bg-purple-600',
    }

    return `${baseClasses} ${colorClasses[buttonColor] || ''}`
  }
</script>

<style>
  .vote-button {
    transition: all 0.2s ease;
    min-width: 100px;
    margin: 0 0.5rem;
  }

  .vote-button:active {
    transform: scale(0.95);
  }
</style>

<button class={getButtonClass(color)} on:click={handleVote} aria-label="{color} button"></button>
