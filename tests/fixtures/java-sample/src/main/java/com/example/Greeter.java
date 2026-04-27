package com.example;

/**
 * Simple greeter class.
 */
public class Greeter {
    private final String name;

    /**
     * Creates a new Greeter.
     *
     * @param name the name to greet
     */
    public Greeter(String name) {
        this.name = name;
    }

    /**
     * Returns a greeting message.
     *
     * @return greeting string
     */
    public String greet() {
        return "Hello, " + name + "!";
    }

    /**
     * Gets the name.
     *
     * @return the name
     */
    public String getName() {
        return name;
    }
}
