pub mod registry;

use std::fs;
use std::path::{Path, PathBuf};

use crate::error::app_error::AppError;
use crate::models::plugin::{
    PluginManifest, PluginRuntime, PluginState,
};

use self::registry::PluginRegistry;

/// 插件加载器
pub struct PluginLoader {
    /// 插件搜索路径
    search_paths: Vec<PathBuf>,
    /// 插件注册中心
    registry: PluginRegistry,
}

impl PluginLoader {
    pub fn new() -> Self {
        Self {
            search_paths: Vec::new(),
            registry: PluginRegistry::new(),
        }
    }

    pub fn add_search_path(&mut self, path: &Path) {
        self.search_paths.push(path.to_path_buf());
    }

    pub fn registry(&self) -> &PluginRegistry {
        &self.registry
    }

    pub fn registry_mut(&mut self) -> &mut PluginRegistry {
        &mut self.registry
    }

    /// 从目录发现并加载所有插件
    pub async fn discover_and_load(&mut self) -> Result<Vec<PluginRuntime>, AppError> {
        let mut runtimes = Vec::new();

        for search_path in &self.search_paths.clone() {
            if !search_path.exists() {
                continue;
            }

            let entries = fs::read_dir(search_path).map_err(|e| {
                AppError::Internal(format!("无法读取插件目录: {}", e))
            })?;

            for entry in entries {
                let entry = entry.map_err(|e| {
                    AppError::Internal(format!("无法读取插件条目: {}", e))
                })?;

                let path = entry.path();
                if path.is_dir() {
                    if let Ok(runtime) = self.load_plugin(&path).await {
                        runtimes.push(runtime);
                    }
                }
            }
        }

        Ok(runtimes)
    }

    /// 加载单个插件
    pub async fn load_plugin(&mut self, plugin_dir: &Path) -> Result<PluginRuntime, AppError> {
        let start = std::time::Instant::now();

        let manifest_path = plugin_dir.join("plugin.json");
        if !manifest_path.exists() {
            return Err(AppError::Internal(format!(
                "plugin.json 未找到: {}",
                plugin_dir.display()
            )));
        }

        let manifest_content = fs::read_to_string(&manifest_path).map_err(|e| {
            AppError::Internal(format!("无法读取 plugin.json: {}", e))
        })?;

        let manifest: PluginManifest = serde_json::from_str(&manifest_content).map_err(|e| {
            AppError::Internal(format!("plugin.json 格式错误: {}", e))
        })?;

        // 验证版本
        self.validate_manifest(&manifest)?;

        // 检查依赖
        self.check_dependencies(&manifest)?;

        let install_path = plugin_dir.to_string_lossy().to_string();
        let load_duration_ms = start.elapsed().as_millis() as u64;

        let runtime = PluginRuntime {
            manifest: manifest.clone(),
            state: PluginState::Loaded,
            install_path: install_path.clone(),
            loaded_at: Some(chrono::Utc::now().timestamp_millis()),
            error: None,
            load_duration_ms: Some(load_duration_ms),
        };

        // 注册到注册中心
        self.registry.register(runtime.clone());

        Ok(runtime)
    }

    /// 验证 manifest 合法性
    fn validate_manifest(&self, manifest: &PluginManifest) -> Result<(), AppError> {
        if manifest.name.is_empty() {
            return Err(AppError::Internal("插件名称不能为空".into()));
        }
        if manifest.version.is_empty() {
            return Err(AppError::Internal("插件版本不能为空".into()));
        }
        if manifest.name.len() > 128 {
            return Err(AppError::Internal("插件名称过长".into()));
        }
        // 验证名称格式
        if !manifest.name.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
            return Err(AppError::Internal("插件名称只能包含字母、数字、连字符和下划线".into()));
        }
        Ok(())
    }

    /// 检查依赖是否满足
    fn check_dependencies(&self, manifest: &PluginManifest) -> Result<(), AppError> {
        for dep in &manifest.dependencies {
            if dep.optional {
                continue;
            }
            if !self.registry.has(&dep.name) {
                return Err(AppError::Internal(format!(
                    "插件 '{}' 缺少依赖: {} (>= {})",
                    manifest.name, dep.name, dep.version
                )));
            }
        }
        Ok(())
    }

    /// 卸载插件
    pub fn unload_plugin(&mut self, name: &str) -> Result<(), AppError> {
        self.registry.unregister(name)
    }
}

impl Default for PluginLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn create_test_plugin(dir: &Path, name: &str) {
        fs::create_dir_all(dir).unwrap();
        let manifest = serde_json::json!({
            "name": name,
            "display_name": "Test Plugin",
            "version": "1.0.0",
            "description": "A test plugin",
            "category": "utility",
            "tags": [],
            "entry": {},
            "permissions": [],
            "dependencies": [],
            "skills": [],
            "mcp_servers": [],
            "commands": [],
            "hooks": []
        });
        let manifest_path = dir.join("plugin.json");
        let mut file = fs::File::create(&manifest_path).unwrap();
        file.write_all(manifest.to_string().as_bytes()).unwrap();
    }

    #[tokio::test]
    async fn test_load_plugin() {
        let temp = std::env::temp_dir().join("nexterm_plugin_test");
        let _ = fs::remove_dir_all(&temp);
        let plugin_dir = temp.join("test-plugin");
        create_test_plugin(&plugin_dir, "test-plugin");

        let mut loader = PluginLoader::new();
        let result = loader.load_plugin(&plugin_dir).await;
        assert!(result.is_ok());

        let runtime = result.unwrap();
        assert_eq!(runtime.manifest.name, "test-plugin");
        assert_eq!(runtime.state, PluginState::Loaded);

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_validate_manifest() {
        let loader = PluginLoader::new();
        let manifest = PluginManifest {
            name: "".into(),
            display_name: "".into(),
            version: "1.0.0".into(),
            description: "".into(),
            author: None,
            license: None,
            homepage: None,
            repository: None,
            min_yuan_version: None,
            category: crate::models::plugin::PluginCategory::Utility,
            tags: vec![],
            icon: None,
            entry: Default::default(),
            permissions: vec![],
            dependencies: vec![],
            skills: vec![],
            mcp_servers: vec![],
            commands: vec![],
            hooks: vec![],
            enabled: true,
        };
        assert!(loader.validate_manifest(&manifest).is_err());
    }
}