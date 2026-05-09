use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FacadeResponse {
    pub status_code: u16,
    pub content_type: String,
    pub body: String,
    pub headers: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FacadeResponder {
    server_name: String,
}

impl FacadeResponder {
    pub fn new(server_name: String) -> Self {
        Self { server_name }
    }

    pub fn to_http_bytes(response: &FacadeResponse) -> Vec<u8> {
        let status_text = match response.status_code {
            200 => "OK",
            403 => "Forbidden",
            404 => "Not Found",
            _ => "OK",
        };

        let mut http = format!(
            "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n",
            response.status_code,
            status_text,
            response.content_type,
            response.body.len(),
        );

        for (key, value) in &response.headers {
            http.push_str(&format!("{key}: {value}\r\n"));
        }
        http.push_str("\r\n");
        http.push_str(&response.body);

        http.into_bytes()
    }

    pub fn respond_for_probe(&self, path: &str) -> FacadeResponse {
        let normalized_path = if path.trim().is_empty() { "/" } else { path };
        let mut headers = HashMap::new();
        headers.insert(String::from("Server"), self.server_name.clone());
        headers.insert(String::from("Cache-Control"), String::from("no-store"));

        FacadeResponse {
            status_code: 200,
            content_type: String::from("text/html; charset=utf-8"),
            body: format!(
                "<html><body><h1>Welcome</h1><p>Resource {normalized_path}</p></body></html>"
            ),
            headers,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::stealth::facade::FacadeResponder;

    #[test]
    fn facade_response_hides_tunnel_error_details() {
        let responder = FacadeResponder::new(String::from("nginx"));
        let response = responder.respond_for_probe("/login");

        assert_eq!(200, response.status_code);
        assert!(!response.body.contains("tunnel"));
        assert!(!response.body.contains("auth"));
    }
}
