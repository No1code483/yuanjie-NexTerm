//! 板块级插件（L1）集合。
//! 阶段1 B8：首个最小插件 _hello（插件化最小闭环演示）。
//! 阶段2 S2：boards.profile 完成 registry/manifest/migration 适配。
//! 阶段3 批次1b-1：boards.home（L1 骨架 + home.todo / home.journal 两个 L2）。
//! 阶段3 批次2a-1：boards.knowledge（单 L1，无 L2 —— 裁定 33-A）。
//! 阶段3 批次2b-1：boards.ai（L1 + ai.models / ai.agent / ai.groupchat 三个 L2）。
//! 阶段3 批次3a：boards.terminal（L1 直接持 29 命令 + 6 表；terminal.linux 插槽 3b 挂载）。
//! 阶段3 批次4a-1：boards.xin（L1 基础面 28 命令 + 6 表 + xin.wellness / xin.realtime 两个 L2）。
//! 阶段3 批次4b：boards.game（L1 44 条游戏命令 + 17 表）。

pub mod _hello;
pub mod ai;
pub mod game;
pub mod home;
pub mod knowledge;
pub mod profile;
pub mod terminal;
pub mod xin;
