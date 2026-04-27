package com.example;

/**
 * Main class for testing Java LSP functionality.
 */
public class Main {

    /**
     * Main entry point.
     *
     * @param args command line arguments
     */
    public static void main(String[] args) {
        Greeter greeter = new Greeter("World");
        System.out.println(greeter.greet());

        Calculator calc = new Calculator();
        int result = calc.add(1, 2);
        System.out.println("1 + 2 = " + result);
    }
}
