<script>
  let input = ""
  let messages = []
  let is_loading = false

  async function sendMessage() {
    if (!input.trim()) return
    
    is_loading = true
    const user_message = input
    input = ""
    
    messages = [...messages, { text: user_message }]
    
    try {
      const response = await fetch("http://localhost:8080/api/chat", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ message: user_message })
      })

      const data = await response.json()
      messages = [...messages, { text: data.response }]
    } catch (error) {
      console.log("Error: " + error.message)
    }
    
    is_loading = false
  }
</script>

<main>
  <div class="chat-container">
      <div class="messages">
      {#each messages as msg}
          <div class:user={!msg.isBot}>
          {msg.text}
          </div>
      {/each}
      </div>

      <div class="input-area">
      <input 
          bind:value={input} 
          on:keydown={e => e.key === 'Enter' && sendMessage()}
          disabled={is_loading}
          placeholder="Type your message..."
      >
      <button on:click={sendMessage} disabled={is_loading}>
          {is_loading ? 'Sending...' : 'Send'}
      </button>
      </div>
  </div>
</main>

<style>
  .chat-container {
    max-width: 800px;
    margin: 0 auto;
    padding: 20px;
  }

  .messages {
    height: 60vh;
    overflow-y: auto;
    border: 1px solid #ccc;
    padding: 10px;
    margin-bottom: 10px;
  }

  .user {
    text-align: right;
    color: blue;
    margin: 5px 0;
  }

  .input-area {
    display: flex;
    gap: 10px;
  }

  input {
    flex: 1;
    padding: 8px;
  }

  button {
    padding: 8px 16px;
  }
</style>
