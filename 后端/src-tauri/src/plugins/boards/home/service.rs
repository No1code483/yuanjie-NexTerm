//! boards.home（L1）服务边界说明。
//!
//! L1 只做板块骨架与插槽声明：不持有业务表、不提供跨插件服务。
//! todos / journals 等表的归属分别登记在对应 L2（home.todo / home.journal）manifest，
//! 由 L2 的基线迁移写入 kernel_table_ownership。

pub const OWNED_TABLES: &[&str] = &[];
