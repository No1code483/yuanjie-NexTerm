use crate::models::sandbox::{NetworkPolicy, PermissionProfile};

pub struct PermissionManager;

impl PermissionManager {
    pub fn create_read_only(read_paths: Vec<String>) -> PermissionProfile {
        PermissionProfile {
            name: "read_only".into(),
            read_paths,
            write_paths: vec![],
            allowed_commands: vec![],
            network_policy: NetworkPolicy::DenyAll,
            max_file_size: 10 * 1024 * 1024,
            max_output_size: 1024 * 1024,
        }
    }

    pub fn create_read_write(read_paths: Vec<String>, write_paths: Vec<String>) -> PermissionProfile {
        PermissionProfile {
            name: "read_write".into(),
            read_paths,
            write_paths,
            allowed_commands: vec![],
            network_policy: NetworkPolicy::DenyAll,
            max_file_size: 50 * 1024 * 1024,
            max_output_size: 5 * 1024 * 1024,
        }
    }

    pub fn create_runner(
        read_paths: Vec<String>,
        write_paths: Vec<String>,
        commands: Vec<String>,
    ) -> PermissionProfile {
        PermissionProfile {
            name: "runner".into(),
            read_paths,
            write_paths,
            allowed_commands: commands,
            network_policy: NetworkPolicy::DenyAll,
            max_file_size: 20 * 1024 * 1024,
            max_output_size: 5 * 1024 * 1024,
        }
    }

    pub fn create_network_restricted(allowed_domains: Vec<String>) -> PermissionProfile {
        PermissionProfile {
            name: "network_restricted".into(),
            read_paths: vec![".".into()],
            write_paths: vec![".".into(), "/tmp".into()],
            allowed_commands: vec![],
            network_policy: NetworkPolicy::AllowList(allowed_domains),
            max_file_size: 10 * 1024 * 1024,
            max_output_size: 5 * 1024 * 1024,
        }
    }

    pub fn expand_paths(&self, profile: &mut PermissionProfile, base_dir: &str) {
        let base = base_dir.trim_end_matches('/');

        profile.read_paths = profile
            .read_paths
            .iter()
            .map(|p| {
                if p.starts_with('.') {
                    format!("{}{}", base, p.trim_start_matches('.'))
                } else {
                    p.clone()
                }
            })
            .collect();

        profile.write_paths = profile
            .write_paths
            .iter()
            .map(|p| {
                if p.starts_with('.') {
                    format!("{}{}", base, p.trim_start_matches('.'))
                } else {
                    p.clone()
                }
            })
            .collect();
    }
}

impl Default for PermissionManager {
    fn default() -> Self {
        Self
    }
}