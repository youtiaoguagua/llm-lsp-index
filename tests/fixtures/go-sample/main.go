package main

import "fmt"

// Greeter interface for greeting
type Greeter interface {
	Greet(name string) string
}

// SimpleGreeter implements Greeter
type SimpleGreeter struct {
	Prefix string
}

// Greet implements the Greeter interface
func (s *SimpleGreeter) Greet(name string) string {
	return fmt.Sprintf("%s, %s!", s.Prefix, name)
}

// Add calculates the sum of two integers
func Add(a, b int) int {
	return a + b
}

func main() {
	greeter := &SimpleGreeter{Prefix: "Hello"}
	fmt.Println(greeter.Greet("World"))
	fmt.Println(Add(1, 2))
}
