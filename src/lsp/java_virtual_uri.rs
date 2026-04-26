//! Java Virtual URI Handler
//!
//! Handles JDT LS virtual URIs (jdt://contents/...) for fetching decompiled
//! source code from JAR files via the JDT LS java/classFileContents extension.

use crate::lsp::LspClient;

/// Java virtual URI handler
pub struct JavaVirtualUriHandler;

impl JavaVirtualUriHandler {
    /// Check if a URI is a Java virtual URI
    pub fn is_virtual_uri(uri: &str) -> bool {
        uri.starts_with("jdt://") || uri.starts_with("jar://")
    }

    /// Extract the class name from a jdt:// URI
    ///
    /// jdt://contents/org.springframework.web.bind.annotation/RestController.class
    /// returns "org.springframework.web.bind.annotation.RestController"
    pub fn extract_class_name(uri: &str) -> Option<String> {
        if !uri.starts_with("jdt://") {
            return None;
        }

        // Format: jdt://contents/{package}/{class}.class
        let content_part = uri.strip_prefix("jdt://")?;

        // Handle contents/ prefix
        let without_contents = if content_part.starts_with("contents/") {
            content_part.strip_prefix("contents/")?
        } else {
            content_part
        };

        // Find the last '/' to separate package from class
        let parts: Vec<&str> = without_contents.rsplitn(2, '/').collect();
        if parts.len() != 2 {
            return None;
        }

        let class_file = parts[0];
        let package = parts[1];

        // Remove .class extension
        let class_name = class_file.strip_suffix(".class")?;

        // Combine: package + . + class
        Some(format!("{}.{}", package.replace('/', "."), class_name))
    }

    /// Fetch source code for a virtual URI from JDT LS
    ///
    /// Uses the JDT LS extension command "java/classFileContents" to retrieve
    /// decompiled or source-jar attached source code.
    pub async fn fetch_source(
        client: &mut LspClient,
        uri: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        if !Self::is_virtual_uri(uri) {
            return Err(format!("Not a virtual URI: {}", uri).into());
        }

        tracing::info!("Fetching Java source for virtual URI: {}", uri);

        // JDT LS uses a custom extension to fetch class file contents
        // This is the same mechanism VS Code Java extension uses
        let params = serde_json::json!({
            "textDocument": {
                "uri": uri
            }
        });

        // Try multiple approaches - JDT LS has changed this API over time

        // Approach 1: textDocument/content (newer JDT LS)
        let result = client
            .send_custom_request("textDocument/content", params.clone())
            .await;

        match result {
            Ok(response) => {
                if let Some(content) = response.get("content").and_then(|c| c.as_str()) {
                    return Ok(content.to_string());
                }
            }
            Err(e) => {
                tracing::debug!("textDocument/content failed: {}", e);
            }
        }

        // Approach 2: java/classFileContents (older JDT LS)
        let params = serde_json::json!({
            "uri": uri
        });

        let result = client
            .send_custom_request("java/classFileContents", params)
            .await;

        match result {
            Ok(response) => {
                if let Some(content) = response.get("content").and_then(|c| c.as_str()) {
                    return Ok(content.to_string());
                }
                // Sometimes the content is directly in the response
                if let Some(content) = response.as_str() {
                    return Ok(content.to_string());
                }
            }
            Err(e) => {
                tracing::debug!("java/classFileContents failed: {}", e);
            }
        }

        // Approach 3: workspace/executeCommand with java.classFileContents
        let params = serde_json::json!({
            "command": "java.classFileContents",
            "arguments": [uri]
        });

        let result = client
            .send_custom_request("workspace/executeCommand", params)
            .await;

        match result {
            Ok(response) => {
                if let Some(content) = response.as_str() {
                    return Ok(content.to_string());
                }
            }
            Err(e) => {
                tracing::debug!("workspace/executeCommand failed: {}", e);
            }
        }

        Err("Failed to fetch source from JDT LS. All approaches failed.".into())
    }

    /// Format virtual URI for display (remove protocol prefix)
    pub fn format_for_display(uri: &str) -> String {
        if uri.starts_with("jdt://") {
            uri.strip_prefix("jdt://").unwrap_or(uri).to_string()
        } else if uri.starts_with("jar://") {
            uri.strip_prefix("jar://").unwrap_or(uri).to_string()
        } else {
            uri.to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_virtual_uri() {
        assert!(JavaVirtualUriHandler::is_virtual_uri(
            "jdt://contents/org.example/Class.class"
        ));
        assert!(JavaVirtualUriHandler::is_virtual_uri(
            "jar://file.jar!/org/example/Class.class"
        ));
        assert!(!JavaVirtualUriHandler::is_virtual_uri(
            "file:///home/user/project/Main.java"
        ));
    }

    #[test]
    fn test_extract_class_name() {
        assert_eq!(
            JavaVirtualUriHandler::extract_class_name(
                "jdt://contents/org.springframework.web.bind.annotation/RestController.class"
            ),
            Some("org.springframework.web.bind.annotation.RestController".to_string())
        );

        assert_eq!(
            JavaVirtualUriHandler::extract_class_name(
                "jdt://contents/java.lang/String.class"
            ),
            Some("java.lang.String".to_string())
        );
    }

    #[test]
    fn test_format_for_display() {
        assert_eq!(
            JavaVirtualUriHandler::format_for_display(
                "jdt://contents/org.example/Class.class"
            ),
            "contents/org.example/Class.class"
        );

        assert_eq!(
            JavaVirtualUriHandler::format_for_display(
                "file:///home/user/Main.java"
            ),
            "file:///home/user/Main.java"
        );
    }
}
