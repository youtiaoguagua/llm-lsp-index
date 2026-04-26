/// A simple greeting function
pub fn greet(name: &str) -> String {
    format!("Hello, {}!", name)
}

/// Calculate the sum of two numbers
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

/// A simple struct for testing
pub struct User {
    pub name: String,
    pub age: u32,
}

impl User {
    /// Create a new User
    pub fn new(name: String, age: u32) -> Self {
        Self { name, age }
    }

    /// Get user info
    pub fn info(&self) -> String {
        format!("{} is {} years old", self.name, self.age)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_greet() {
        assert_eq!(greet("World"), "Hello, World!");
    }

    #[test]
    fn test_add() {
        assert_eq!(add(1, 2), 3);
    }
}