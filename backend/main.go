package main

import (
	"context"
	"encoding/json"
	"fmt"
	"log"
	"net/http"
	"os"
	"os/signal"
	"sync"
	"syscall"
	"time"

	"github.com/gin-gonic/gin"
	"github.com/gorilla/websocket"
)

var go_port = "8080"
var svelte_url = "http://localhost:5173"
var manager *Client_Manager

var upgrader = websocket.Upgrader{
	ReadBufferSize:    1024,
	WriteBufferSize:   1024,
	EnableCompression: true,
	HandshakeTimeout:  500 * time.Millisecond,
	CheckOrigin: func(r *http.Request) bool {
		origin := r.Header.Get("Origin")
		return origin == svelte_url
	},
}

type Client_Manager struct {
	clients    map[*websocket.Conn]bool
	broadcast  chan []byte
	register   chan *websocket.Conn
	unregister chan *websocket.Conn
	mutex      sync.Mutex
}

var counters = struct {
	sync.RWMutex
	Red    int64
	Green  int64
	Blue   int64
	Purple int64
	Total  int64
}{}

func Create_Client_Manager() *Client_Manager {
	return &Client_Manager{
		clients:    make(map[*websocket.Conn]bool),
		broadcast:  make(chan []byte, 1), //how many messages before broadcast goes
		register:   make(chan *websocket.Conn),
		unregister: make(chan *websocket.Conn),
	}
}

func (manager *Client_Manager) Start() {
	periodic_user_check := time.NewTicker(500 * time.Millisecond)
	defer periodic_user_check.Stop()

	for {
		select {
		case <-periodic_user_check.C:
			manager.mutex.Lock()
			userCount := len(manager.clients)
			manager.mutex.Unlock()

			message := []byte(fmt.Sprintf(`{"type":"users","count":%d}`, userCount))
			manager.Broadcast(message)

		case connection := <-manager.register:
			manager.mutex.Lock()
			manager.clients[connection] = true
			manager.mutex.Unlock()
			log.Printf("Client connected. Total connections: %d", len(manager.clients))

		case connection := <-manager.unregister:
			manager.mutex.Lock()
			if _, ok := manager.clients[connection]; ok {
				delete(manager.clients, connection)
				connection.Close()
			}
			manager.mutex.Unlock()
			log.Printf("Client disconnected. Total connections: %d", len(manager.clients))

		case message := <-manager.broadcast:
			manager.mutex.Lock()
			for connection := range manager.clients {
				go func(client_connection *websocket.Conn) {
					err := client_connection.WriteMessage(websocket.TextMessage, message)
					if err != nil {
						manager.unregister <- client_connection
					}
				}(connection)
			}
			manager.mutex.Unlock()
		}
	}
}
func (manager *Client_Manager) Broadcast(message []byte) {
	manager.broadcast <- message
}

func main() {
	if port := os.Getenv("GO_PORT"); port != "" {
		go_port = port
	}
	if frontend_url := os.Getenv("SVELTE_URL"); frontend_url != "" {
		svelte_url = frontend_url
	}

	router := gin.Default()

	router.Use(func(c *gin.Context) {
		c.Writer.Header().Set("Access-Control-Allow-Origin", svelte_url)
		c.Writer.Header().Set("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
		c.Writer.Header().Set("Access-Control-Allow-Headers", "Content-Type")
	})

	manager = Create_Client_Manager()
	go manager.Start()

	router.POST("/api/increment/:color", increment_handler)
	router.GET("/api/ws", websocket_handler)

	srv := &http.Server{
		Addr:    fmt.Sprintf(":%s", go_port),
		Handler: router,
	}

	go func() {
		quit := make(chan os.Signal, 1)
		signal.Notify(quit, syscall.SIGINT, syscall.SIGTERM)
		<-quit
		log.Println("Shutting down server...")

		ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
		defer cancel()
		if err := srv.Shutdown(ctx); err != nil {
			log.Printf("Server forced to shutdown: %v", err)
		}
	}()

	log.Printf("Server running on %s", go_port)
	log.Printf("Environment variables:")
	log.Printf("GO_PORT: %s", go_port)
	log.Printf("SVELTE_URL: %s", svelte_url)

	if err := srv.ListenAndServe(); err != nil && err != http.ErrServerClosed {
		log.Fatalf("Failed to start server: %v", err)
	}
}

func increment_handler(c *gin.Context) {
	color := c.Param("color")

	counters.Lock()

	switch color {
	case "red":
		counters.Red++
	case "green":
		counters.Green++
	case "blue":
		counters.Blue++
	case "purple":
		counters.Purple++
	}
	counters.Total++
	response := map[string]interface{}{
		"red":    counters.Red,
		"green":  counters.Green,
		"blue":   counters.Blue,
		"purple": counters.Purple,
		"total":  counters.Total,
	}

	counters.Unlock()

	message, err := json.Marshal(response)
	if err != nil {
		log.Printf("Error forwarding increment request: %v", err)
		c.JSON(http.StatusInternalServerError, gin.H{"error": "Failed to increment counter"})
		return
	}

	manager.Broadcast(message)
	c.Status(http.StatusOK)
}

func websocket_handler(c *gin.Context) {
	connection, err := upgrader.Upgrade(c.Writer, c.Request, nil)
	if err != nil {
		log.Printf("Failed to upgrade connection: %v", err)
		return
	}

	manager.register <- connection

	counters.RLock()

	response := map[string]interface{}{
		"red":    counters.Red,
		"green":  counters.Green,
		"blue":   counters.Blue,
		"purple": counters.Purple,
	}

	counters.RUnlock()
	message, err := json.Marshal(response)
	if err == nil {
		connection.WriteMessage(websocket.TextMessage, message)
	}

	for {
		_, _, err := connection.ReadMessage()
		if err != nil {
			manager.unregister <- connection
			break
		}
	}
}
