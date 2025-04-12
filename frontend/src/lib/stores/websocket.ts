import { get, writable } from 'svelte/store'
import { PUBLIC_WS_URL } from '$env/static/public'

const MSG_KEYS = new Set(['red', 'green', 'blue', 'purple', 'total'])

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

        if (msg.type == undefined) {
          update((currentData) => ({
            ...currentData,
            ...Object.entries(msg).reduce((acc, [key, value]) => {
              if (MSG_KEYS.has(key)) {
                acc[key] = value
              }
              return acc
            }, {}),
          }))
        } else if (msg.type === 'initial') {
          update((currentData) => ({
            ...currentData,
            ...(msg.count !== undefined && { total_users: msg.count }),
            ...(msg.total !== undefined && { total: msg.total }),
            ...(msg.red !== undefined && { red: msg.red }),
            ...(msg.green !== undefined && { green: msg.green }),
            ...(msg.blue !== undefined && { blue: msg.blue }),
            ...(msg.purple !== undefined && { purple: msg.purple }),
          }))
        } else if (msg.type === 'users') {
          update((currentData) => ({
            ...currentData,
            ...(msg.count !== undefined && { total_users: msg.count }),
          }))
        }
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
    console.log('reconnecting...')
    const delay = Math.min(Math.random() * 3000, MAX_RECONNECT_DELAY)
    reconnectTimer = setTimeout(() => {
      reconnectTimer = null
      connect()
    }, delay)
  }

  const sendPayload = (payload: string) => {
    if (socket?.readyState === WebSocket.OPEN) {
      socket.send('' + payload)
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
