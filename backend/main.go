package main

import (
	"fmt"
	"io"
	"log"
	"net/http"
	"time"

	"github.com/gin-gonic/gin"
)

const port = ":8080"

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

	c.Status(http.StatusOK)
}

func counter_handler(c *gin.Context) {

	client := &http.Client{}

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

	client := &http.Client{}

	w := c.Writer
	w.Flush()

	for {

		response, err := client.Get("http://localhost:3000/counters")
		if err != nil {
			log.Println("Error - (Go)update_handler - Failed to Fetch Update:", err)
			continue
		}

		body, _ := io.ReadAll(response.Body)
		response.Body.Close()

		fmt.Fprintf(w, "data: %s\n\n", string(body))
		w.Flush()
		time.Sleep(100 * time.Millisecond)
	}
}
