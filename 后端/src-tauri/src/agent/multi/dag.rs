// DAG 任务依赖图 — 对标 Codex 的多 Agent 任务编排
// 定义 Agent 之间的执行顺序和依赖关系

use std::collections::{HashMap, VecDeque};

/// DAG 节点
#[derive(Debug, Clone)]
pub struct DagNode {
    /// 节点 ID
    pub id: String,
    /// 任务名称
    pub name: String,
    /// 任务描述
    pub description: String,
    /// 目标 Agent 角色
    pub agent_role: String,
    /// 依赖节点 ID
    pub dependencies: Vec<String>,
    /// 下游节点 ID
    pub dependents: Vec<String>,
    /// 执行状态
    pub status: DagNodeStatus,
    /// 执行结果
    pub result: Option<String>,
    /// 预计耗时 (ms)
    pub estimated_duration_ms: u64,
}

/// 节点状态
#[derive(Debug, Clone, PartialEq)]
pub enum DagNodeStatus {
    Pending,
    Ready,
    Running,
    Completed,
    Failed(String),
    Skipped,
}

/// DAG 执行图
pub struct DagGraph {
    /// 节点映射
    nodes: HashMap<String, DagNode>,
    /// 执行状态
    pub is_running: bool,
    /// 已完成节点计数
    completed_count: usize,
    /// 失败节点计数
    failed_count: usize,
}

impl DagGraph {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            is_running: false,
            completed_count: 0,
            failed_count: 0,
        }
    }

    /// 添加节点
    pub fn add_node(
        &mut self,
        id: String,
        name: String,
        description: String,
        agent_role: String,
        dependencies: Vec<String>,
        estimated_duration_ms: u64,
    ) -> Result<(), String> {
        if self.nodes.contains_key(&id) {
            return Err(format!("节点已存在: {}", id));
        }

        // 验证依赖
        for dep in &dependencies {
            if !self.nodes.contains_key(dep) {
                return Err(format!("依赖节点不存在: {}", dep));
            }
        }

        let node = DagNode {
            id: id.clone(),
            name,
            description,
            agent_role,
            dependencies,
            dependents: Vec::new(),
            status: DagNodeStatus::Pending,
            result: None,
            estimated_duration_ms,
        };

        self.nodes.insert(id, node);
        self.rebuild_dependents();
        Ok(())
    }

    /// 重建下游关系
    fn rebuild_dependents(&mut self) {
        let node_ids: Vec<String> = self.nodes.keys().cloned().collect();
        let mut dependents_map: HashMap<String, Vec<String>> = HashMap::new();

        for id in &node_ids {
            if let Some(node) = self.nodes.get(id) {
                for dep in &node.dependencies {
                    dependents_map
                        .entry(dep.clone())
                        .or_default()
                        .push(id.clone());
                }
            }
        }

        for id in &node_ids {
            if let Some(node) = self.nodes.get_mut(id) {
                node.dependents = dependents_map.remove(id).unwrap_or_default();
            }
        }
    }

    /// 获取就绪节点（依赖全部完成）
    pub fn ready_nodes(&self) -> Vec<&DagNode> {
        self.nodes
            .values()
            .filter(|n| {
                n.status == DagNodeStatus::Pending
                    && n.dependencies.iter().all(|dep_id| {
                        self.nodes
                            .get(dep_id)
                            .map(|d| d.status == DagNodeStatus::Completed)
                            .unwrap_or(false)
                    })
            })
            .collect()
    }

    /// 获取根节点（无依赖）
    pub fn root_nodes(&self) -> Vec<&DagNode> {
        self.nodes
            .values()
            .filter(|n| n.dependencies.is_empty())
            .collect()
    }

    /// 获取叶子节点（无下游）
    pub fn leaf_nodes(&self) -> Vec<&DagNode> {
        self.nodes
            .values()
            .filter(|n| n.dependents.is_empty())
            .collect()
    }

    /// 拓扑排序
    pub fn topological_order(&self) -> Vec<String> {
        let mut in_degree: HashMap<&str, usize> = HashMap::new();
        let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();

        for node in self.nodes.values() {
            in_degree.entry(&node.id).or_insert(0);
            for dep in &node.dependencies {
                adj.entry(dep.as_str()).or_default().push(&node.id);
                *in_degree.entry(node.id.as_str()).or_insert(0) += 1;
            }
        }

        let mut queue: VecDeque<&str> = in_degree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(&id, _)| id)
            .collect();

        let mut order = Vec::new();

        while let Some(node_id) = queue.pop_front() {
            order.push(node_id.to_string());
            if let Some(neighbors) = adj.get(node_id) {
                for &neighbor in neighbors {
                    if let Some(deg) = in_degree.get_mut(neighbor) {
                        *deg -= 1;
                        if *deg == 0 {
                            queue.push_back(neighbor);
                        }
                    }
                }
            }
        }

        order
    }

    /// 检测是否有环
    pub fn has_cycle(&self) -> bool {
        let order = self.topological_order();
        order.len() != self.nodes.len()
    }

    /// 标记节点开始执行
    pub fn mark_running(&mut self, node_id: &str) -> Result<(), String> {
        let node = self
            .nodes
            .get_mut(node_id)
            .ok_or_else(|| format!("节点不存在: {}", node_id))?;
        node.status = DagNodeStatus::Running;
        self.is_running = true;
        Ok(())
    }

    /// 标记节点完成
    pub fn mark_completed(&mut self, node_id: &str, result: String) -> Result<(), String> {
        let node = self
            .nodes
            .get_mut(node_id)
            .ok_or_else(|| format!("节点不存在: {}", node_id))?;
        node.status = DagNodeStatus::Completed;
        node.result = Some(result);
        self.completed_count += 1;
        Ok(())
    }

    /// 标记节点失败
    pub fn mark_failed(&mut self, node_id: &str, error: &str) -> Result<(), String> {
        let node = self
            .nodes
            .get_mut(node_id)
            .ok_or_else(|| format!("节点不存在: {}", node_id))?;
        node.status = DagNodeStatus::Failed(error.to_string());
        self.failed_count += 1;

        // 级联跳过下游节点
        self.cascade_skip(node_id);
        Ok(())
    }

    /// 级联跳过下游节点
    fn cascade_skip(&mut self, failed_node_id: &str) {
        let dependents: Vec<String> = self
            .nodes
            .get(failed_node_id)
            .map(|n| n.dependents.clone())
            .unwrap_or_default();

        for dep_id in dependents {
            if let Some(node) = self.nodes.get_mut(&dep_id) {
                if node.status == DagNodeStatus::Pending {
                    node.status = DagNodeStatus::Skipped;
                    self.cascade_skip(&dep_id);
                }
            }
        }
    }

    /// 是否全部完成
    pub fn is_all_completed(&self) -> bool {
        self.nodes
            .values()
            .all(|n| n.status == DagNodeStatus::Completed || n.status == DagNodeStatus::Skipped)
    }

    /// 是否有失败
    pub fn has_failures(&self) -> bool {
        self.failed_count > 0
    }

    /// 获取统计
    pub fn stats(&self) -> DagStats {
        let total = self.nodes.len();
        let pending = self
            .nodes
            .values()
            .filter(|n| n.status == DagNodeStatus::Pending)
            .count();
        let running = self
            .nodes
            .values()
            .filter(|n| n.status == DagNodeStatus::Running)
            .count();
        let skipped = self
            .nodes
            .values()
            .filter(|n| n.status == DagNodeStatus::Skipped)
            .count();

        DagStats {
            total,
            completed: self.completed_count,
            failed: self.failed_count,
            running,
            pending,
            skipped,
        }
    }

    /// 获取所有节点
    pub fn nodes(&self) -> Vec<&DagNode> {
        self.nodes.values().collect()
    }

    /// 获取节点
    pub fn get_node(&self, node_id: &str) -> Option<&DagNode> {
        self.nodes.get(node_id)
    }

    /// 重置所有节点
    pub fn reset(&mut self) {
        self.is_running = false;
        self.completed_count = 0;
        self.failed_count = 0;
        for node in self.nodes.values_mut() {
            node.status = DagNodeStatus::Pending;
            node.result = None;
        }
    }
}

impl Default for DagGraph {
    fn default() -> Self {
        Self::new()
    }
}

/// DAG 统计
#[derive(Debug, Clone)]
pub struct DagStats {
    pub total: usize,
    pub completed: usize,
    pub failed: usize,
    pub running: usize,
    pub pending: usize,
    pub skipped: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_topological_order() {
        let mut dag = DagGraph::new();
        dag.add_node("a".into(), "A".into(), "Task A".into(), "coder".into(), vec![], 1000).unwrap();
        dag.add_node("b".into(), "B".into(), "Task B".into(), "coder".into(), vec!["a".into()], 2000).unwrap();
        dag.add_node("c".into(), "C".into(), "Task C".into(), "reviewer".into(), vec!["a".into()], 1500).unwrap();
        dag.add_node("d".into(), "D".into(), "Task D".into(), "tester".into(), vec!["b".into(), "c".into()], 3000).unwrap();

        let order = dag.topological_order();
        assert_eq!(order.len(), 4);
        // a 必须在 b, c, d 之前
        let a_pos = order.iter().position(|id| id == "a").unwrap();
        let b_pos = order.iter().position(|id| id == "b").unwrap();
        let c_pos = order.iter().position(|id| id == "c").unwrap();
        let d_pos = order.iter().position(|id| id == "d").unwrap();
        assert!(a_pos < b_pos);
        assert!(a_pos < c_pos);
        assert!(b_pos < d_pos);
        assert!(c_pos < d_pos);
    }

    #[test]
    fn test_has_cycle() {
        let mut dag = DagGraph::new();
        dag.add_node("a".into(), "A".into(), "Task A".into(), "coder".into(), vec!["b".into()], 1000).unwrap();
        dag.add_node("b".into(), "B".into(), "Task B".into(), "coder".into(), vec!["a".into()], 1000).unwrap();
        assert!(dag.has_cycle());

        let mut dag2 = DagGraph::new();
        dag2.add_node("a".into(), "A".into(), "Task A".into(), "coder".into(), vec![], 1000).unwrap();
        dag2.add_node("b".into(), "B".into(), "Task B".into(), "coder".into(), vec!["a".into()], 1000).unwrap();
        assert!(!dag2.has_cycle());
    }

    // ===== 2.5.1 新增：覆盖 add_node 错误路径 + root/leaf/ready =====

    #[test]
    fn test_add_node_rejects_duplicate_id() {
        let mut dag = DagGraph::new();
        dag.add_node("a".into(), "A".into(), "Task A".into(), "coder".into(), vec![], 1000).unwrap();
        let err = dag.add_node("a".into(), "A2".into(), "dup".into(), "coder".into(), vec![], 1000);
        assert!(err.is_err(), "重复 id 应失败");
    }

    #[test]
    fn test_add_node_rejects_missing_dependency() {
        let mut dag = DagGraph::new();
        let err = dag.add_node("a".into(), "A".into(), "Task A".into(), "coder".into(), vec!["ghost".into()], 1000);
        assert!(err.is_err(), "依赖不存在应失败");
    }

    #[test]
    fn test_root_and_leaf_nodes() {
        let mut dag = DagGraph::new();
        dag.add_node("a".into(), "A".into(), "Task A".into(), "coder".into(), vec![], 1000).unwrap();
        dag.add_node("b".into(), "B".into(), "Task B".into(), "coder".into(), vec!["a".into()], 2000).unwrap();
        dag.add_node("c".into(), "C".into(), "Task C".into(), "reviewer".into(), vec!["b".into()], 1500).unwrap();

        let roots = dag.root_nodes();
        assert_eq!(roots.len(), 1, "仅 1 个根节点");
        assert_eq!(roots[0].id, "a");

        let leaves = dag.leaf_nodes();
        assert_eq!(leaves.len(), 1, "仅 1 个叶子节点");
        assert_eq!(leaves[0].id, "c");
    }

    #[test]
    fn test_ready_nodes_after_dependency_completed() {
        let mut dag = DagGraph::new();
        dag.add_node("a".into(), "A".into(), "Task A".into(), "coder".into(), vec![], 1000).unwrap();
        dag.add_node("b".into(), "B".into(), "Task B".into(), "coder".into(), vec!["a".into()], 2000).unwrap();

        // 初始：仅 a 就绪（无依赖），b 未就绪
        let ready = dag.ready_nodes();
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].id, "a");

        // 完成 a 后，b 应就绪
        dag.mark_completed("a", "done".into()).unwrap();
        let ready = dag.ready_nodes();
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].id, "b");
    }

    #[test]
    fn test_mark_running_sets_is_running_flag() {
        let mut dag = DagGraph::new();
        dag.add_node("a".into(), "A".into(), "Task A".into(), "coder".into(), vec![], 1000).unwrap();
        assert!(!dag.is_running);
        dag.mark_running("a").unwrap();
        assert!(dag.is_running, "mark_running 后 is_running 应为 true");
    }

    #[test]
    fn test_mark_failed_cascades_skip_to_dependents() {
        let mut dag = DagGraph::new();
        dag.add_node("a".into(), "A".into(), "Task A".into(), "coder".into(), vec![], 1000).unwrap();
        dag.add_node("b".into(), "B".into(), "Task B".into(), "coder".into(), vec!["a".into()], 2000).unwrap();
        dag.add_node("c".into(), "C".into(), "Task C".into(), "reviewer".into(), vec!["b".into()], 1500).unwrap();

        dag.mark_failed("a", "boom").unwrap();

        // b 和 c 应被级联跳过
        assert_eq!(dag.get_node("b").unwrap().status, DagNodeStatus::Skipped);
        assert_eq!(dag.get_node("c").unwrap().status, DagNodeStatus::Skipped);
        assert!(dag.has_failures());
    }

    #[test]
    fn test_is_all_completed() {
        let mut dag = DagGraph::new();
        dag.add_node("a".into(), "A".into(), "Task A".into(), "coder".into(), vec![], 1000).unwrap();
        dag.add_node("b".into(), "B".into(), "Task B".into(), "coder".into(), vec!["a".into()], 2000).unwrap();

        assert!(!dag.is_all_completed());
        dag.mark_completed("a", "done".into()).unwrap();
        assert!(!dag.is_all_completed());
        dag.mark_completed("b", "done".into()).unwrap();
        assert!(dag.is_all_completed());
    }

    #[test]
    fn test_stats_reports_correct_counts() {
        let mut dag = DagGraph::new();
        dag.add_node("a".into(), "A".into(), "Task A".into(), "coder".into(), vec![], 1000).unwrap();
        dag.add_node("b".into(), "B".into(), "Task B".into(), "coder".into(), vec!["a".into()], 2000).unwrap();
        dag.add_node("c".into(), "C".into(), "Task C".into(), "reviewer".into(), vec!["b".into()], 1500).unwrap();

        dag.mark_completed("a", "ok".into()).unwrap();
        dag.mark_failed("b", "err").unwrap();
        // c 应被跳过

        let stats = dag.stats();
        assert_eq!(stats.total, 3);
        assert_eq!(stats.completed, 1);
        assert_eq!(stats.failed, 1);
        assert_eq!(stats.skipped, 1);
        assert_eq!(stats.pending, 0);
    }

    #[test]
    fn test_reset_restores_initial_state() {
        let mut dag = DagGraph::new();
        dag.add_node("a".into(), "A".into(), "Task A".into(), "coder".into(), vec![], 1000).unwrap();
        dag.mark_running("a").unwrap();
        dag.mark_completed("a", "done".into()).unwrap();

        dag.reset();
        assert!(!dag.is_running);
        let node = dag.get_node("a").unwrap();
        assert_eq!(node.status, DagNodeStatus::Pending);
        assert!(node.result.is_none());
    }

    #[test]
    fn test_mark_nonexistent_node_returns_error() {
        let mut dag = DagGraph::new();
        assert!(dag.mark_running("ghost").is_err());
        assert!(dag.mark_completed("ghost", "x".into()).is_err());
        assert!(dag.mark_failed("ghost", "e").is_err());
    }
}