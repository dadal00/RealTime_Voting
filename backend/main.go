package main

import (
	"context"
	"fmt"
	"io"
	"log"
	"net/http"
	"sync"
	"time"

	"github.com/gin-gonic/gin"
)

const (
	port                  = ":8080"
	update_channel_buffer = 10
)

type Broker struct {
	clients map[chan []byte]bool
	mutex   sync.Mutex
}

func create_broker() *Broker {
	return &Broker{
		clients: make(map[chan []byte]bool),
	}
}

func (broker *Broker) add_client() chan []byte {
	broker.mutex.Lock()
	defer broker.mutex.Unlock()

	channel := make(chan []byte, update_channel_buffer)
	broker.clients[channel] = true
	return channel
}

func (broker *Broker) remove_client(channel chan []byte) {
	broker.mutex.Lock()
	defer broker.mutex.Unlock()

	delete(broker.clients, channel)
	close(channel)
}

func (broker *Broker) broadcast(data []byte) {
	broker.mutex.Lock()
	defer broker.mutex.Unlock()

	for client := range broker.clients {
		select {
		case client <- data:
		default:
			delete(broker.clients, client)
			close(client)
		}
	}
}

var broker = create_broker()

func main() {

	router := gin.Default()

	router.Use(func(c *gin.Context) {
		c.Writer.Header().Set("Access-Control-Allow-Origin", "*")
		c.Writer.Header().Set("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
		c.Writer.Header().Set("Access-Control-Allow-Headers", "Content-Type")
	})

	router.POST("/increment/:color", increment_handler)
	router.GET("/updates", update_handler)
	router.GET("/counters", counter_handler)

	fmt.Println("Status - (Go)Program running on " + port)
	router.Run(port)
}

func increment_handler(c *gin.Context) {

	color := c.Param("color")

	response, err := http.Post("http://localhost:3000/increment/"+color, "text/plain", nil)
	if err != nil {
		log.Println("Error - (Go)increment_handler - Failed to Increment:", err)
		c.JSON(http.StatusInternalServerError, gin.H{"error": "Error - (Go)increment_handler - Failed to Increment"})
		return
	}
	defer response.Body.Close()

	go func() {
		client := &http.Client{Timeout: 2 * time.Second}
		response, err := client.Get("http://localhost:3000/counters")
		if err != nil {
			log.Println("Error - (Go)increment_handler - Failed to fetch updated counters:", err)
			return
		}
		defer response.Body.Close()

		body, err := io.ReadAll(response.Body)
		if err != nil {
			log.Println("Error - (Go)increment_handler - Failed to read counters:", err)
			return
		}

		broker.broadcast(body)
	}()

	c.Status(http.StatusOK)
}

func counter_handler(c *gin.Context) {

	client := &http.Client{Timeout: 2 * time.Second}

	response, err := client.Get("http://localhost:3000/counters")
	if err != nil {
		log.Println("Error - (Go)counter_handler - Failed to Fetch Counters:", err)
		c.JSON(http.StatusInternalServerError, gin.H{"error": "Error - (Go)counter_handler - Failed to Fetch Counters"})
		return
	}
	defer response.Body.Close()

	body, err := io.ReadAll(response.Body)
	if err != nil {
		log.Println("Error - (Go)counter_handler - Failed to Read Response:", err)
		c.JSON(http.StatusInternalServerError, gin.H{"error": "Error - (Go)counter_handler - Failed to Read Response"})
		return
	}

	c.Data(http.StatusOK, "application/json", body)
}

func update_handler(c *gin.Context) {

	c.Header("Content-Type", "text/event-stream")
	c.Header("Cache-Control", "no-cache")
	c.Header("Connection", "keep-alive")

	w := c.Writer
	flusher, ok := w.(http.Flusher)
	if !ok {
		c.AbortWithStatus(http.StatusInternalServerError)
		return
	}

	client_channel := broker.add_client()
	defer broker.remove_client(client_channel)

	initialCounters, err := fetch_initial_counters()
	if err == nil {
		fmt.Fprintf(w, "data: %s\n\n", initialCounters)
		flusher.Flush()
	}

	cancellable_context, cancel := context.WithCancel(c.Request.Context())
	defer cancel()

	for {
		select {
		case data := <-client_channel:
			fmt.Fprintf(w, "data: %s\n\n", data)
			flusher.Flush()
		case <-cancellable_context.Done():
			return
		}
	}
}

func fetch_initial_counters() ([]byte, error) {

	client := &http.Client{Timeout: 2 * time.Second}

	response, err := client.Get("http://localhost:3000/counters")
	if err != nil {
		return nil, err
	}
	defer response.Body.Close()

	return io.ReadAll(response.Body)
}
