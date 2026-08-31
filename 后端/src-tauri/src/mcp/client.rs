use std::collections::HashMap;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};

use crate::error::app_error::AppError;
use crate::models::mcp::{
    ClientInfo, InitializeParams, JsonRpcRequest, JsonRpcResponse, Tool,
};

pub struct McpClient {
    id: String,
    process: Option<Child>,
    server_info: Option<crate::models::mcp::ServerInfo>,
    tools: Vec<Tool>,
}

impl McpClient {
    pub fn new(id: &str) -> Self {
        Self {
            id: id.to_string(),
            process: None,
            server_info: None,
            tools: vec![],
        }
    }

    pub async fn connect(
        &mut self,
        command: &str,
        args: &[String],
        env: Option<&HashMap<String, String>>,
    ) -> Result<(), AppError> {
        let mut cmd = Command::new(command);
        cmd.args(args);
        cmd.stdin(std::process::Stdio::piped());
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        if let Some(env_vars) = env {
            for (k, v) in env_vars {
                cmd.env(k, v);
            }
        }

        let mut child = cmd.spawn().map_err(|e| {
            AppError::Internal(format!("无法启动MCP服务进程: {}", e))
        })?;

        let stdin = child.stdin.take().ok_or(AppError::Internal("无法获取stdin".into()))?;
        let stdout = child.stdout.take().ok_or(AppError::Internal("无法获取stdout".into()))?;

        let mut reader = BufReader::new(stdout);
        let mut writer = stdin;

        let init_params = InitializeParams {
            protocol_version: "2024-11-05".into(),
            capabilities: crate::models::mcp::ClientCapabilities {
                roots: None,
                sampling: None,
                experimental: None,
            },
            client_info: Some(ClientInfo {
                name: "YuanCode".into(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            }),
        };

        let init_request = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(serde_json::Value::Number(1.into())),
            method: "initialize".into(),
            params: Some(serde_json::to_value(&init_params).unwrap_or_default()),
        };

        let mut init_json = serde_json::to_string(&init_request)
            .map_err(|e| AppError::Internal(format!("序列化失败: {}", e)))?;
        init_json.push('\n');

        writer
            .write_all(init_json.as_bytes())
            .await
            .map_err(|e| AppError::Internal(format!("写入失败: {}", e)))?;

        let mut response_line = String::new();
        reader
            .read_line(&mut response_line)
            .await
            .map_err(|e| AppError::Internal(format!("读取响应失败: {}", e)))?;

        let response: JsonRpcResponse = serde_json::from_str(&response_line)
            .map_err(|e| AppError::Internal(format!("解析响应失败: {}", e)))?;

        if let Some(err) = response.error {
            return Err(AppError::Internal(format!(
                "MCP初始化错误: {} (code: {})",
                err.message, err.code
            )));
        }

        if let Some(result) = response.result {
            let init_result: crate::models::mcp::InitializeResult =
                serde_json::from_value(result).map_err(|e| {
                    AppError::Internal(format!("解析初始化结果失败: {}", e))
                })?;
            self.server_info = Some(init_result.server_info.clone());
        }

        let initialized_notification = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        });
        let mut notif_json = serde_json::to_string(&initialized_notification)
            .map_err(|e| AppError::Internal(format!("序列化失败: {}", e)))?;
        notif_json.push('\n');
        writer.write_all(notif_json.as_bytes()).await.ok();

        self.fetch_tools(&mut writer, &mut reader).await?;
        self.process = Some(child);

        Ok(())
    }

    async fn fetch_tools(
        &mut self,
        writer: &mut tokio::process::ChildStdin,
        reader: &mut BufReader<tokio::process::ChildStdout>,
    ) -> Result<(), AppError> {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(serde_json::Value::Number(2.into())),
            method: "tools/list".into(),
            params: None,
        };

        let mut json = serde_json::to_string(&request)
            .map_err(|e| AppError::Internal(format!("序列化失败: {}", e)))?;
        json.push('\n');
        writer
            .write_all(json.as_bytes())
            .await
            .map_err(|e| AppError::Internal(format!("写入失败: {}", e)))?;

        let mut response_line = String::new();
        reader
            .read_line(&mut response_line)
            .await
            .map_err(|e| AppError::Internal(format!("读取tools/list失败: {}", e)))?;

        let response: JsonRpcResponse = serde_json::from_str(&response_line)
            .map_err(|e| AppError::Internal(format!("解析tools/list失败: {}", e)))?;

        if let Some(result) = response.result {
            if let Some(tools_array) = result.get("tools").and_then(|v| v.as_array()) {
                self.tools = tools_array
                    .iter()
                    .filter_map(|v| serde_json::from_value::<Tool>(v.clone()).ok())
                    .collect();
            }
        }

        Ok(())
    }

    pub fn tools(&self) -> &[Tool] {
        &self.tools
    }

    pub fn server_info(&self) -> Option<&crate::models::mcp::ServerInfo> {
        self.server_info.as_ref()
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn is_connected(&self) -> bool {
        self.process.is_some()
    }

    /// 调用远程 MCP 工具
    pub async fn call_tool(
        &mut self,
        tool_name: &str,
        arguments: Option<serde_json::Value>,
    ) -> Result<crate::models::mcp::ToolCallResult, AppError> {
        let call_request = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(serde_json::Value::Number(2.into())),
            method: "tools/call".into(),
            params: Some(serde_json::json!({
                "name": tool_name,
                "arguments": arguments.unwrap_or(serde_json::json!({})),
            })),
        };

        let request_json = serde_json::to_string(&call_request)
            .map_err(|e| AppError::Internal(format!("序列化tools/call失败: {}", e)))?;

        let mut stdin = self
            .process
            .as_mut()
            .and_then(|p| p.stdin.take())
            .ok_or(AppError::Internal("无法获取stdin".into()))?;

        let stdout = self
            .process
            .as_mut()
            .and_then(|p| p.stdout.take())
            .ok_or(AppError::Internal("无法获取stdout".into()))?;

        let mut reader = BufReader::new(stdout);

        let mut json = request_json;
        json.push('\n');
        stdin
            .write_all(json.as_bytes())
            .await
            .map_err(|e| AppError::Internal(format!("写入tools/call失败: {}", e)))?;

        let mut response_line = String::new();
        reader
            .read_line(&mut response_line)
            .await
            .map_err(|e| AppError::Internal(format!("读取tools/call响应失败: {}", e)))?;

        let response: JsonRpcResponse = serde_json::from_str(&response_line)
            .map_err(|e| AppError::Internal(format!("解析tools/call响应失败: {}", e)))?;

        if let Some(error) = response.error {
            return Err(AppError::Internal(format!(
                "MCP工具调用错误: {}",
                error.message
            )));
        }

        if let Some(result) = response.result {
            serde_json::from_value::<crate::models::mcp::ToolCallResult>(result)
                .map_err(|e| AppError::Internal(format!("解析工具调用结果失败: {}", e)))
        } else {
            Err(AppError::Internal("MCP工具调用返回空结果".into()))
        }
    }

    pub async fn disconnect(&mut self) {
        if let Some(mut child) = self.process.take() {
            let _ = child.kill().await;
            let _ = child.wait().await;
        }
        self.tools.clear();
        self.server_info = None;
    }

    /// 健康检查 ping
    pub async fn ping(&mut self) -> Result<(), AppError> {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(serde_json::Value::Number(999.into())),
            method: "ping".into(),
            params: None,
        };

        let request_json = serde_json::to_string(&request)
            .map_err(|e| AppError::Internal(format!("序列化ping失败: {}", e)))?;

        let process = self
            .process
            .as_mut()
            .ok_or(AppError::Internal("服务器未连接".into()))?;

        let stdin = process.stdin.as_mut()
            .ok_or(AppError::Internal("无法获取stdin".into()))?;

        let stdout = process.stdout.as_mut()
            .ok_or(AppError::Internal("无法获取stdout".into()))?;

        let mut reader = BufReader::new(stdout);
        let mut json = request_json;
        json.push('\n');
        stdin.write_all(json.as_bytes()).await.map_err(|e| {
            AppError::Internal(format!("写入ping失败: {}", e))
        })?;

        let mut response_line = String::new();
        reader.read_line(&mut response_line).await.map_err(|e| {
            AppError::Internal(format!("读取ping响应失败: {}", e))
        })?;

        let response: JsonRpcResponse = serde_json::from_str(&response_line)
            .map_err(|e| AppError::Internal(format!("解析ping响应失败: {}", e)))?;

        if response.error.is_some() {
            return Err(AppError::Internal("MCP 服务器 ping 失败".into()));
        }

        Ok(())
    }

    /// 优雅关闭
    pub async fn shutdown(&mut self) -> Result<(), AppError> {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(serde_json::Value::Number(998.into())),
            method: "shutdown".into(),
            params: None,
        };

        let request_json = serde_json::to_string(&request)
            .map_err(|e| AppError::Internal(format!("序列化shutdown失败: {}", e)))?;

        if let Some(ref mut process) = self.process {
            if let Some(stdin) = process.stdin.as_mut() {
                let mut json = request_json;
                json.push('\n');
                let _ = stdin.write_all(json.as_bytes()).await;
            }
        }

        Ok(())
    }

    /// 强制终止进程
    pub async fn force_kill(&mut self) {
        if let Some(mut child) = self.process.take() {
            let _ = child.kill().await;
            let _ = child.wait().await;
        }
    }

    /// 列出 MCP 资源
    pub async fn list_resources(&mut self) -> Result<Vec<crate::models::mcp::Resource>, AppError> {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(serde_json::Value::Number(3.into())),
            method: "resources/list".into(),
            params: None,
        };

        let request_json = serde_json::to_string(&request)
            .map_err(|e| AppError::Internal(format!("序列化resources/list失败: {}", e)))?;

        let process = self
            .process
            .as_mut()
            .ok_or(AppError::Internal("服务器未连接".into()))?;

        let stdin = process.stdin.as_mut()
            .ok_or(AppError::Internal("无法获取stdin".into()))?;

        let stdout = process.stdout.as_mut()
            .ok_or(AppError::Internal("无法获取stdout".into()))?;

        let mut reader = BufReader::new(stdout);
        let mut json = request_json;
        json.push('\n');
        stdin.write_all(json.as_bytes()).await.map_err(|e| {
            AppError::Internal(format!("写入resources/list失败: {}", e))
        })?;

        let mut response_line = String::new();
        reader.read_line(&mut response_line).await.map_err(|e| {
            AppError::Internal(format!("读取resources/list失败: {}", e))
        })?;

        let response: JsonRpcResponse = serde_json::from_str(&response_line)
            .map_err(|e| AppError::Internal(format!("解析resources/list失败: {}", e)))?;

        if let Some(result) = response.result {
            if let Some(resources_array) = result.get("resources").and_then(|v| v.as_array()) {
                return Ok(resources_array
                    .iter()
                    .filter_map(|v| serde_json::from_value::<crate::models::mcp::Resource>(v.clone()).ok())
                    .collect());
            }
        }

        Ok(vec![])
    }

    /// 读取 MCP 资源
    pub async fn read_resource(
        &mut self,
        uri: &str,
    ) -> Result<crate::models::mcp::ReadResourceResult, AppError> {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(serde_json::Value::Number(4.into())),
            method: "resources/read".into(),
            params: Some(serde_json::json!({ "uri": uri })),
        };

        let request_json = serde_json::to_string(&request)
            .map_err(|e| AppError::Internal(format!("序列化resources/read失败: {}", e)))?;

        let process = self
            .process
            .as_mut()
            .ok_or(AppError::Internal("服务器未连接".into()))?;

        let stdin = process.stdin.as_mut()
            .ok_or(AppError::Internal("无法获取stdin".into()))?;

        let stdout = process.stdout.as_mut()
            .ok_or(AppError::Internal("无法获取stdout".into()))?;

        let mut reader = BufReader::new(stdout);
        let mut json = request_json;
        json.push('\n');
        stdin.write_all(json.as_bytes()).await.map_err(|e| {
            AppError::Internal(format!("写入resources/read失败: {}", e))
        })?;

        let mut response_line = String::new();
        reader.read_line(&mut response_line).await.map_err(|e| {
            AppError::Internal(format!("读取resources/read失败: {}", e))
        })?;

        let response: JsonRpcResponse = serde_json::from_str(&response_line)
            .map_err(|e| AppError::Internal(format!("解析resources/read失败: {}", e)))?;

        if let Some(error) = response.error {
            return Err(AppError::Internal(format!(
                "MCP 资源读取错误: {}",
                error.message
            )));
        }

        if let Some(result) = response.result {
            serde_json::from_value::<crate::models::mcp::ReadResourceResult>(result)
                .map_err(|e| AppError::Internal(format!("解析资源读取结果失败: {}", e)))
        } else {
            Err(AppError::Internal("MCP 资源读取返回空结果".into()))
        }
    }

    /// 列出资源模板
    pub async fn list_resource_templates(
        &mut self,
    ) -> Result<Vec<crate::models::mcp::Resource>, AppError> {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(serde_json::Value::Number(5.into())),
            method: "resources/templates/list".into(),
            params: None,
        };

        let request_json = serde_json::to_string(&request)
            .map_err(|e| AppError::Internal(format!("序列化resources/templates/list失败: {}", e)))?;

        let process = self
            .process
            .as_mut()
            .ok_or(AppError::Internal("服务器未连接".into()))?;

        let stdin = process.stdin.as_mut()
            .ok_or(AppError::Internal("无法获取stdin".into()))?;

        let stdout = process.stdout.as_mut()
            .ok_or(AppError::Internal("无法获取stdout".into()))?;

        let mut reader = BufReader::new(stdout);
        let mut json = request_json;
        json.push('\n');
        stdin.write_all(json.as_bytes()).await.map_err(|e| {
            AppError::Internal(format!("写入resources/templates/list失败: {}", e))
        })?;

        let mut response_line = String::new();
        reader.read_line(&mut response_line).await.map_err(|e| {
            AppError::Internal(format!("读取resources/templates/list失败: {}", e))
        })?;

        let response: JsonRpcResponse = serde_json::from_str(&response_line)
            .map_err(|e| AppError::Internal(format!("解析resources/templates/list失败: {}", e)))?;

        if let Some(result) = response.result {
            if let Some(templates) = result.get("resourceTemplates").and_then(|v| v.as_array()) {
                return Ok(templates
                    .iter()
                    .filter_map(|v| serde_json::from_value::<crate::models::mcp::Resource>(v.clone()).ok())
                    .collect());
            }
        }

        Ok(vec![])
    }

    /// 列出 MCP Prompt 模板
    pub async fn list_prompts(
        &mut self,
    ) -> Result<Vec<crate::models::mcp::Prompt>, AppError> {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(serde_json::Value::Number(6.into())),
            method: "prompts/list".into(),
            params: None,
        };

        let request_json = serde_json::to_string(&request)
            .map_err(|e| AppError::Internal(format!("序列化prompts/list失败: {}", e)))?;

        let process = self
            .process
            .as_mut()
            .ok_or(AppError::Internal("服务器未连接".into()))?;

        let stdin = process.stdin.as_mut()
            .ok_or(AppError::Internal("无法获取stdin".into()))?;

        let stdout = process.stdout.as_mut()
            .ok_or(AppError::Internal("无法获取stdout".into()))?;

        let mut reader = BufReader::new(stdout);
        let mut json = request_json;
        json.push('\n');
        stdin.write_all(json.as_bytes()).await.map_err(|e| {
            AppError::Internal(format!("写入prompts/list失败: {}", e))
        })?;

        let mut response_line = String::new();
        reader.read_line(&mut response_line).await.map_err(|e| {
            AppError::Internal(format!("读取prompts/list失败: {}", e))
        })?;

        let response: JsonRpcResponse = serde_json::from_str(&response_line)
            .map_err(|e| AppError::Internal(format!("解析prompts/list失败: {}", e)))?;

        if let Some(result) = response.result {
            if let Some(prompts) = result.get("prompts").and_then(|v| v.as_array()) {
                return Ok(prompts
                    .iter()
                    .filter_map(|v| serde_json::from_value::<crate::models::mcp::Prompt>(v.clone()).ok())
                    .collect());
            }
        }

        Ok(vec![])
    }

    /// 获取 MCP Prompt 模板
    pub async fn get_prompt(
        &mut self,
        name: &str,
        arguments: Option<serde_json::Value>,
    ) -> Result<crate::models::mcp::GetPromptResult, AppError> {
        let params = serde_json::json!({
            "name": name,
            "arguments": arguments.unwrap_or(serde_json::json!({})),
        });

        let request = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(serde_json::Value::Number(7.into())),
            method: "prompts/get".into(),
            params: Some(params),
        };

        let request_json = serde_json::to_string(&request)
            .map_err(|e| AppError::Internal(format!("序列化prompts/get失败: {}", e)))?;

        let process = self
            .process
            .as_mut()
            .ok_or(AppError::Internal("服务器未连接".into()))?;

        let stdin = process.stdin.as_mut()
            .ok_or(AppError::Internal("无法获取stdin".into()))?;

        let stdout = process.stdout.as_mut()
            .ok_or(AppError::Internal("无法获取stdout".into()))?;

        let mut reader = BufReader::new(stdout);
        let mut json = request_json;
        json.push('\n');
        stdin.write_all(json.as_bytes()).await.map_err(|e| {
            AppError::Internal(format!("写入prompts/get失败: {}", e))
        })?;

        let mut response_line = String::new();
        reader.read_line(&mut response_line).await.map_err(|e| {
            AppError::Internal(format!("读取prompts/get失败: {}", e))
        })?;

        let response: JsonRpcResponse = serde_json::from_str(&response_line)
            .map_err(|e| AppError::Internal(format!("解析prompts/get失败: {}", e)))?;

        if let Some(error) = response.error {
            return Err(AppError::Internal(format!("prompts/get错误: {}", error.message)));
        }

        if let Some(result) = response.result {
            serde_json::from_value::<crate::models::mcp::GetPromptResult>(result)
                .map_err(|e| AppError::Internal(format!("解析prompts/get结果失败: {}", e)))
        } else {
            Err(AppError::Internal("prompts/get返回空结果".into()))
        }
    }
}