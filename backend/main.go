package main

import (
	"context"
	"fmt"
	"io"
	"log"
	"net/http"
	"os"
	"os/signal"
	"sync"
	"syscall"
	"time"

	"github.com/gin-gonic/gin"
	"github.com/gorilla/websocket"
	"github.com/segmentio/kafka-go"
)

const (
	port             = ":8080"
	kafka_broker     = "localhost:9092"
	counters_topic   = "counters-updates"
	rust_service_url = "http://localhost:3000"
)

var upgrader = websocket.Upgrader{
	ReadBufferSize:    1024,
	WriteBufferSize:   1024,
	EnableCompression: true,
	HandshakeTimeout:  500 * time.Millisecond,
	CheckOrigin: func(r *http.Request) bool {
		return true // Allow all origins in development
	},
}

type Client_Manager struct {
	clients    map[*websocket.Conn]bool
	broadcast  chan []byte
	register   chan *websocket.Conn
	unregister chan *websocket.Conn
	mutex      sync.Mutex
}

func Create_Client_Manager() *Client_Manager {
	return &Client_Manager{
		clients:    make(map[*websocket.Conn]bool),
		broadcast:  make(chan []byte, 10),
		register:   make(chan *websocket.Conn),
		unregister: make(chan *websocket.Conn),
	}
}

func (manager *Client_Manager) Start() {
	for {
		select {
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
				err := connection.WriteMessage(websocket.TextMessage, message)
				if err != nil {
					connection.Close()
					delete(manager.clients, connection)
				}
			}
			manager.mutex.Unlock()
		}
	}
}

func (manager *Client_Manager) Broadcast(message []byte) {
	manager.broadcast <- message
}

func main() {

	router := gin.Default()

	router.Use(func(c *gin.Context) {
		c.Writer.Header().Set("Access-Control-Allow-Origin", "*")
		c.Writer.Header().Set("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
		c.Writer.Header().Set("Access-Control-Allow-Headers", "Content-Type")
	})

	manager := Create_Client_Manager()
	go manager.Start()

	reader := kafka.NewReader(kafka.ReaderConfig{
		Brokers:     []string{kafka_broker},
		Topic:       counters_topic,
		GroupID:     "counter-service",
		MinBytes:    1,
		MaxBytes:    1e6,
		StartOffset: kafka.LastOffset,
		MaxWait:     100 * time.Millisecond,
	})

	writer := kafka.NewWriter(kafka.WriterConfig{
		Brokers: []string{kafka_broker},
		Topic:   counters_topic,
	})

	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	go func() {
		for {
			select {
			case <-ctx.Done():
				return
			default:
				message, err := reader.ReadMessage(ctx)
				if err != nil {
					log.Printf("Error reading from Kafka: %v", err)
					continue
				}

				manager.Broadcast(message.Value)
			}
		}
	}()

	router.POST("/increment/:color", func(c *gin.Context) {
		color := c.Param("color")
		increment_handler(c, color)
	})
	router.GET("/counters", counters_handler)
	router.GET("/ws", func(c *gin.Context) {
		websocket_handler(c, manager)
	})

	srv := &http.Server{
		Addr:    port,
		Handler: router,
	}

	go func() {
		quit := make(chan os.Signal, 1)
		signal.Notify(quit, syscall.SIGINT, syscall.SIGTERM)
		<-quit
		log.Println("Shutting down server...")

		if err := reader.Close(); err != nil {
			log.Printf("Failed to close Kafka reader: %v", err)
		}
		if err := writer.Close(); err != nil {
			log.Printf("Failed to close Kafka writer: %v", err)
		}

		ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
		defer cancel()
		if err := srv.Shutdown(ctx); err != nil {
			log.Printf("Server forced to shutdown: %v", err)
		}
	}()

	log.Printf("Server running on %s", port)
	if err := srv.ListenAndServe(); err != nil && err != http.ErrServerClosed {
		log.Fatalf("Failed to start server: %v", err)
	}
}

func increment_handler(c *gin.Context, color string) {
	response, err := http.Post(fmt.Sprintf("%s/increment/%s", rust_service_url, color), "text/plain", nil)
	if err != nil {
		log.Printf("Error forwarding increment request: %v", err)
		c.JSON(http.StatusInternalServerError, gin.H{"error": "Failed to increment counter"})
		return
	}
	defer response.Body.Close()

	c.Status(http.StatusOK)
}

func counters_handler(c *gin.Context) {
	counters, err := fetchCounters()
	if err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": "Failed to fetch counters"})
		return
	}
	c.Data(http.StatusOK, "application/json", counters)
}

func websocket_handler(c *gin.Context, manager *Client_Manager) {
	connection, err := upgrader.Upgrade(c.Writer, c.Request, nil)
	if err != nil {
		log.Printf("Failed to upgrade connection: %v", err)
		return
	}

	manager.register <- connection

	initial_counters, err := fetchCounters()
	if err == nil {
		connection.WriteMessage(websocket.TextMessage, initial_counters)
	}

	for {
		_, _, err := connection.ReadMessage()
		if err != nil {
			manager.unregister <- connection
			break
		}
	}
}

func fetchCounters() ([]byte, error) {
	client := &http.Client{Timeout: 2 * time.Second}
	response, err := client.Get(fmt.Sprintf("%s/counters", rust_service_url))
	if err != nil {
		return nil, err
	}
	defer response.Body.Close()

	return io.ReadAll(response.Body)
}
