package main

import (
	"fmt"
	"io"
	"log"
	"net/http"
	"time"

	"github.com/gin-gonic/gin"
)

func main() {
	router := gin.Default()
	router.Use(func(c *gin.Context) {
		c.Writer.Header().Set("Access-Control-Allow-Origin", "*")
		c.Writer.Header().Set("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
		c.Writer.Header().Set("Access-Control-Allow-Headers", "Content-Type")
	})

	router.POST("/increment/:color", incrementHandler)
	router.GET("/updates", updateHandler)
	router.GET("/counters", countersHandler)

	fmt.Println("Go server running on :8080")
	router.Run(":8080")
}

func incrementHandler(c *gin.Context) {
	color := c.Param("color")
	response, err := http.Post("http://localhost:3000/increment/"+color, "text/plain", nil)
	if err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": "Failed to increment"})
		return
	}
	defer response.Body.Close()
	c.Status(http.StatusOK)
}

func countersHandler(c *gin.Context) {
	client := &http.Client{}
	response, err := client.Get("http://localhost:3000/counters")
	if err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": "Failed to fetch counters"})
		return
	}
	defer response.Body.Close()

	body, err := io.ReadAll(response.Body)
	if err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": "Failed to read response"})
		return
	}
	c.Data(http.StatusOK, "application/json", body)
}

func updateHandler(c *gin.Context) {
	c.Header("Content-Type", "text/event-stream")
	c.Header("Cache-Control", "no-cache")
	c.Header("Connection", "keep-alive")

	client := &http.Client{}

	w := c.Writer
	w.Flush()

	for {
		response, err := client.Get("http://localhost:3000/counters")
		if err != nil {
			log.Println("Failed to fetch counters:", err)
			continue
		}

		body, _ := io.ReadAll(response.Body)
		response.Body.Close()

		fmt.Fprintf(w, "data: %s\n\n", string(body))
		w.Flush()
		time.Sleep(100 * time.Millisecond)
	}
}
