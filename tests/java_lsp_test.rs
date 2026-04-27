//! Integration tests for Java LSP functionality

use std::path::PathBuf;

/// Test that Java project detection works for Maven projects
#[test]
fn test_java_maven_project_detection() {
    let java_sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/java-sample");

    // Check that pom.xml exists
    let pom_path = java_sample_path.join("pom.xml");
    assert!(pom_path.exists(), "pom.xml should exist in java-sample");

    // Check that source files exist
    let main_java = java_sample_path
        .join("src/main/java/com/example/Main.java");
    assert!(main_java.exists(), "Main.java should exist");

    let greeter_java = java_sample_path
        .join("src/main/java/com/example/Greeter.java");
    assert!(greeter_java.exists(), "Greeter.java should exist");

    let calculator_java = java_sample_path
        .join("src/main/java/com/example/Calculator.java");
    assert!(calculator_java.exists(), "Calculator.java should exist");
}

/// Test Java virtual URI handling
#[test]
fn test_java_virtual_uri_parsing() {
    use lsp_index::lsp::JavaVirtualUriHandler;

    // Test jdt:// URI parsing
    let uri = "jdt://contents/org.springframework.web.bind.annotation/RestController.class";
    let class_name = JavaVirtualUriHandler::extract_class_name(uri);
    assert_eq!(
        class_name,
        Some("org.springframework.web.bind.annotation.RestController".to_string())
    );

    // Test simple java.lang class
    let uri2 = "jdt://contents/java.lang/String.class";
    let class_name2 = JavaVirtualUriHandler::extract_class_name(uri2);
    assert_eq!(class_name2, Some("java.lang.String".to_string()));
}

/// Test Java source file content validation
#[test]
fn test_java_source_files_content() {
    let java_sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/java-sample");

    // Read Main.java and verify structure
    let main_content = std::fs::read_to_string(
        java_sample_path.join("src/main/java/com/example/Main.java")
    ).expect("Failed to read Main.java");

    assert!(main_content.contains("package com.example;"));
    assert!(main_content.contains("public class Main"));
    assert!(main_content.contains("public static void main"));
    assert!(main_content.contains("Greeter"));
    assert!(main_content.contains("Calculator"));

    // Read Greeter.java and verify
    let greeter_content = std::fs::read_to_string(
        java_sample_path.join("src/main/java/com/example/Greeter.java")
    ).expect("Failed to read Greeter.java");

    assert!(greeter_content.contains("private final String name"));
    assert!(greeter_content.contains("public String greet()"));
    assert!(greeter_content.contains("public String getName()"));

    // Read Calculator.java and verify
    let calculator_content = std::fs::read_to_string(
        java_sample_path.join("src/main/java/com/example/Calculator.java")
    ).expect("Failed to read Calculator.java");

    assert!(calculator_content.contains("public int add(int a, int b)"));
    assert!(calculator_content.contains("public int subtract(int a, int b)"));
    assert!(calculator_content.contains("public int multiply(int a, int b)"));
    assert!(calculator_content.contains("public int divide(int a, int b)"));
}
