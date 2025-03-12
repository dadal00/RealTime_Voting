package main

import (
	"bytes"
	"encoding/json"
	"fmt"
	"github.com/gin-gonic/gin"
	"io"
	"log"
	"net/http"
)

const lama_address = "http://localhost:11434"

type Lama_Request struct {
	Model  string `json:"model"`
	Prompt string `json:"prompt"`
	Stream bool   `json:"stream"`
}

type Lama_Response struct {
	Response string `json:"response"`
}

func main() {
	r := gin.Default()

	r.SetTrustedProxies(nil)

	r.Use(func(c *gin.Context) {
		c.Writer.Header().Set("Access-Control-Allow-Origin", "*")
		c.Writer.Header().Set("Access-Control-Allow-Methods", "POST, OPTIONS")
		c.Writer.Header().Set("Access-Control-Allow-Headers", "Content-Type")
	})

	r.OPTIONS("/api/chat", func(c *gin.Context) {
		c.Status(204)
	})

	r.POST("/api/chat", func(c *gin.Context) {
		var query struct {
			Message string `json:"message"`
		}
		if err := c.BindJSON(&query); err != nil {
			c.JSON(http.StatusBadRequest, gin.H{"error": err.Error()})
			log.Println("error: ", err.Error())
			return
		}

		response, err := lama_query(query.Message)
		if err != nil {
			c.JSON(http.StatusInternalServerError, gin.H{"error": err.Error()})
			log.Println("error: ", err.Error())
			return
		}

		c.JSON(http.StatusOK, gin.H{"response": response})
	})

	r.Run(":8080")
}

func lama_query(prompt string) (string, error) {
	request_body := Lama_Request{
		Model:  "deepseek-r1:1.5b",
		Prompt: prompt,
		Stream: false,
	}

	json_body, err := json.Marshal(request_body)
    if err != nil {
		return "", err
	}

	response, err := http.Post(
		lama_address + "/api/generate",
		"application/json",
		bytes.NewBuffer(json_body),
	)
	if err != nil {
		return "", err
	}
	defer response.Body.Close()

	received_body, err := io.ReadAll(response.Body)
    if err != nil {
		return "", err
	}

	var lama_response Lama_Response
	if err := json.Unmarshal(received_body, &lama_response); err != nil {
        return "", fmt.Errorf("failed to unmarshal response: %w", err)
    }

	return lama_response.Response, nil
}
