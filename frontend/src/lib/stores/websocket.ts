import { writable } from 'svelte/store'
import { PUBLIC_WS_URL } from '$env/static/public'

export const websocket = (() => {
  const { subscribe, set, update } = writable({
    total: 0,
    total_users: 0,
    red: 0,
    green: 0,
    blue: 0,
    purple: 0,
  })

  let socket: WebSocket
  let reconnectTimer: any
  const MAX_RECONNECT_DELAY = 5000

  const connect = () => {
    if (socket?.readyState === WebSocket.OPEN) return

    try {
      socket = new WebSocket(PUBLIC_WS_URL)
      socket.binaryType = 'arraybuffer'

      socket.onmessage = (event) => {
        const msg = JSON.parse(event.data)

        if (msg.type === 'users') {
          update((currentData) => ({
            ...currentData,
            ...(msg.count !== undefined && { total_users: msg.count }),
          }))
          return
        }

        update((currentData) => ({
          ...currentData,
          ...(msg.total !== undefined && { total: msg.total }),
          ...(msg.red !== undefined && { red: msg.red }),
          ...(msg.green !== undefined && { green: msg.green }),
          ...(msg.blue !== undefined && { blue: msg.blue }),
          ...(msg.purple !== undefined && { purple: msg.purple }),
        }))
      }
    } catch (e) {
      console.error('Network parse error:', e)
    }

    socket.onopen = () => {
      console.log('connected')
      if (reconnectTimer) {
        clearTimeout(reconnectTimer)
        reconnectTimer = null
      }
    }

    socket.onclose = () => {
      console.log('disconnected')
      attemptReconnect()
    }

    socket.onerror = (e) => {
      console.error('connection error:', e)
      attemptReconnect()
    }

    return socket
  }

  const attemptReconnect = () => {
    if (reconnectTimer) return

    const delay = Math.min(Math.random() * 3000, MAX_RECONNECT_DELAY)
    reconnectTimer = setTimeout(() => {
      reconnectTimer = null
      connect()
    }, delay)
  }

  const sendPayload = (payload: string) => {
    if (socket?.readyState === WebSocket.OPEN) {
      socket.send("" + payload)
    } else {
      console.error('Cannot send vote: not connected')
      attemptReconnect()
    }
  }

  const disconnect = () => {
    if (socket) {
      socket.close()
    }
  }

  return {
    subscribe,
    set,
    connect,
    sendPayload,
    disconnect,
  }
})()
