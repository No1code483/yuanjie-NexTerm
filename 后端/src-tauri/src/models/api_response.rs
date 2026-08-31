use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ApiResponse<T: Serialize> {
    pub code: i32,
    pub message: String,
    pub data: Option<T>,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            code: 0,
            message: "success".into(),
            data: Some(data),
        }
    }

    pub fn success_msg(message: &str) -> Self {
        Self {
            code: 0,
            message: message.into(),
            data: None,
        }
    }

    pub fn error(code: i32, message: &str) -> Self {
        Self {
            code,
            message: message.into(),
            data: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_success_response() {
        let response = ApiResponse::success("test data");
        assert_eq!(response.code, 0);
        assert_eq!(response.message, "success");
        assert_eq!(response.data, Some("test data"));
    }

    #[test]
    fn test_success_msg_response() {
        let response: ApiResponse<()> = ApiResponse::success_msg("operation completed");
        assert_eq!(response.code, 0);
        assert_eq!(response.message, "operation completed");
        assert!(response.data.is_none());
    }

    #[test]
    fn test_error_response() {
        let response: ApiResponse<()> = ApiResponse::error(1005, "authentication failed");
        assert_eq!(response.code, 1005);
        assert_eq!(response.message, "authentication failed");
        assert!(response.data.is_none());
    }

    #[test]
    fn test_response_serialization() {
        let response = ApiResponse::success(vec![1, 2, 3]);
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"code\":0"));
        assert!(json.contains("\"message\":\"success\""));
        assert!(json.contains("[1,2,3]"));
    }

    #[test]
    fn test_error_response_serialization() {
        let response: ApiResponse<String> = ApiResponse::error(5001, "AI model error");
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"code\":5001"));
        assert!(json.contains("\"message\":\"AI model error\""));
        assert!(json.contains("\"data\":null"));
    }

    #[test]
    fn test_response_with_complex_data() {
        let data = serde_json::json!({
            "id": 42,
            "name": "test",
            "items": ["a", "b"]
        });
        let response = ApiResponse::success(data);
        assert_eq!(response.code, 0);
        
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"id\":42"));
        assert!(json.contains("\"name\":\"test\""));
    }

    #[test]
    fn test_response_deserialization() {
        let json = r#"{"code":0,"message":"success","data":{"value":123}}"#;
        let response: ApiResponse<serde_json::Value> = serde_json::from_str(json).unwrap();
        assert_eq!(response.code, 0);
        assert_eq!(response.message, "success");
        assert!(response.data.is_some());
    }
}
