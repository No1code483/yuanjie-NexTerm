use std::collections::HashMap;

use crate::error::app_error::AppError;
use crate::models::plugin::{
    PluginHookEvent, PluginRuntime, PluginState,
};

/// 插件注册中心
pub struct PluginRegistry {
    plugins: HashMap<String, PluginRuntime>,
    /// 钩子 → 插件名列表
    hook_index: HashMap<PluginHookEvent, Vec<String>>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            plugins: HashMap::new(),
            hook_index: HashMap::new(),
        }
    }

    /// 注册插件
    pub fn register(&mut self, runtime: PluginRuntime) {
        let name = runtime.manifest.name.clone();

        // 构建钩子索引
        for hook in &runtime.manifest.hooks {
            self.hook_index
                .entry(hook.event.clone())
                .or_default()
                .push(name.clone());
        }

        self.plugins.insert(name, runtime);
    }

    /// 注销插件
    pub fn unregister(&mut self, name: &str) -> Result<(), AppError> {
        let plugin = self.plugins.remove(name).ok_or(AppError::NotFound)?;

        // 清理钩子索引
        for hook in &plugin.manifest.hooks {
            if let Some(list) = self.hook_index.get_mut(&hook.event) {
                list.retain(|n: &String| n != name);
            }
        }

        Ok(())
    }

    /// 检查插件是否存在
    pub fn has(&self, name: &str) -> bool {
        self.plugins.contains_key(name)
    }

    /// 获取插件运行时
    pub fn get(&self, name: &str) -> Option<&PluginRuntime> {
        self.plugins.get(name)
    }

    /// 获取所有插件
    pub fn list_all(&self) -> Vec<&PluginRuntime> {
        self.plugins.values().collect()
    }

    /// 按分类获取插件
    pub fn list_by_category(
        &self,
        category: &crate::models::plugin::PluginCategory,
    ) -> Vec<&PluginRuntime> {
        self.plugins
            .values()
            .filter(|p| p.manifest.category == *category)
            .collect()
    }

    /// 启用插件
    pub fn enable(&mut self, name: &str) -> Result<PluginState, AppError> {
        let plugin = self
            .plugins
            .get_mut(name)
            .ok_or(AppError::NotFound)?;

        plugin.state = PluginState::Enabled;
        Ok(PluginState::Enabled)
    }

    /// 禁用插件
    pub fn disable(&mut self, name: &str) -> Result<PluginState, AppError> {
        let plugin = self
            .plugins
            .get_mut(name)
            .ok_or(AppError::NotFound)?;

        plugin.state = PluginState::Disabled;
        Ok(PluginState::Disabled)
    }

    /// 获取插件状态
    pub fn state(&self, name: &str) -> Option<PluginState> {
        self.plugins.get(name).map(|p| p.state.clone())
    }

    /// 获取钩子对应的插件
    pub fn get_hooks(&self, event: &PluginHookEvent) -> Vec<&PluginRuntime> {
        self.hook_index
            .get(event)
            .map(|names: &Vec<String>| {
                names
                    .iter()
                    .filter_map(|n| self.plugins.get(n))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// 获取所有钩子事件
    pub fn hook_events(&self) -> Vec<&PluginHookEvent> {
        self.hook_index.keys().collect()
    }

    /// 重新加载插件
    pub fn reload(&mut self, name: &str) -> Result<PluginState, AppError> {
        let plugin = self
            .plugins
            .get_mut(name)
            .ok_or(AppError::NotFound)?;

        plugin.state = PluginState::Loading;
        // 实际重新加载逻辑由 loader 处理
        Ok(PluginState::Loading)
    }

    /// 插件数量
    pub fn count(&self) -> usize {
        self.plugins.len()
    }

    /// 已启用插件数量
    pub fn enabled_count(&self) -> usize {
        self.plugins
            .values()
            .filter(|p| p.state == PluginState::Enabled)
            .count()
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::plugin::{
        PluginCategory, PluginHook, PluginManifest,
    };

    fn make_plugin(name: &str) -> PluginRuntime {
        PluginRuntime {
            manifest: PluginManifest {
                name: name.to_string(),
                display_name: name.to_string(),
                version: "1.0.0".into(),
                description: "".into(),
                author: None,
                license: None,
                homepage: None,
                repository: None,
                min_yuan_version: None,
                category: PluginCategory::Utility,
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
            },
            state: PluginState::Loaded,
            install_path: "".into(),
            loaded_at: None,
            error: None,
            load_duration_ms: None,
        }
    }

    #[test]
    fn test_register_and_get() {
        let mut registry = PluginRegistry::new();
        let runtime = make_plugin("test");
        registry.register(runtime);

        assert!(registry.has("test"));
        assert_eq!(registry.count(), 1);
    }

    #[test]
    fn test_enable_disable() {
        let mut registry = PluginRegistry::new();
        registry.register(make_plugin("test"));

        let state = registry.enable("test").unwrap();
        assert_eq!(state, PluginState::Enabled);

        let state = registry.disable("test").unwrap();
        assert_eq!(state, PluginState::Disabled);
    }

    #[test]
    fn test_hooks() {
        let mut registry = PluginRegistry::new();
        let mut runtime = make_plugin("test");
        runtime.manifest.hooks = vec![PluginHook {
            event: PluginHookEvent::OnFileSave,
            handler: "on_save".into(),
        }];
        registry.register(runtime);

        let hooks = registry.get_hooks(&PluginHookEvent::OnFileSave);
        assert_eq!(hooks.len(), 1);
    }

    #[test]
    fn test_unregister() {
        let mut registry = PluginRegistry::new();
        registry.register(make_plugin("test"));
        assert!(registry.has("test"));

        registry.unregister("test").unwrap();
        assert!(!registry.has("test"));
    }
}