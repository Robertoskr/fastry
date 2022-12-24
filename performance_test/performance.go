package main

import (
	"fmt"
	"io/ioutil"
	"net/http"
	"sync"
	"time"
)

func main() {
	client := &http.Client{
		Timeout: time.Second,
	}

	// Create a wait group to synchronize the goroutines
	var wg sync.WaitGroup

	// Send multiple asynchronous GET requests concurrently
	for i := 0; i < 1000; i++ {
        for j := 0; j < 10; j++ { 
            wg.Add(1)
            go func(i int) {
                defer wg.Done()

                // Send an asynchronous GET request to localhost
                resp, err := client.Get("http://127.0.0.1:8080/")
                if err != nil {
                    fmt.Println(err)
                    return
                }
                defer resp.Body.Close()

                // Read the response body
                body, err := ioutil.ReadAll(resp.Body)
                if err != nil {
                    fmt.Println(err)
                    return
                }

                fmt.Printf("Request %d: %s\n", i, string(body))
            }(i)
        } 
        // Wait for all goroutines to finish
        wg.Wait()
	}
}
