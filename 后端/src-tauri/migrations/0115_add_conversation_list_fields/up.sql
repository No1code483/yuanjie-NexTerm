-- Migration v115: add unread_count and sort_order to conversations table
-- 类型: Static ALTER TABLE
-- 规范: spec ai-chat-enhancement Phase 2 §2.1
-- 说明: 支持会话列表的最后消息预览、未读数标记、拖拽自定义排序功能

ALTER TABLE conversations ADD COLUMN unread_count INTEGER NOT NULL DEFAULT 0;
ALTER TABLE conversations ADD COLUMN sort_order INTEGER NOT NULL DEFAULT 0;
