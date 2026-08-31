import { t } from "i18next";
/**
* IPC Mock 层 - 前端独立开发时模拟后端响应
* 
* 使用方式:
* 1. 在 lib/ipc.ts 中设置 USE_MOCK = true
* 2. 所有 invoke 调用将返回 Mock 数据
* 3. 联调时设置 USE_MOCK = false 切换到真实后端
*/

import type { UserInfo, BackendPermission, ChatSession, AIModel, KnowledgeCategory, KnowledgeItem, TodoItem, NewsItem, RecycledItem, GroupChatOrchestration } from '@/types';

// Mock 数据延迟 (模拟网络请求)
const MOCK_DELAY = 300;
const delay = (ms: number) => new Promise(resolve => setTimeout(resolve, ms));

// 统一响应格式 (对齐后端 ApiResponse<T>)
interface ApiResponse<T> {
  code: number;
  message: string;
  data?: T;
}

// ==================== Auth Mock ====================

export const mockAuth = {
  async login(args: any): Promise<ApiResponse<{
    user: UserInfo;
    token: string;
  }>> {
    await delay(MOCK_DELAY);
    const username = args?.request?.username || args?.username || '';
    void (args?.request?.password || args?.password || '');
    if (!username || username.length < 3) {
      return {
        code: 1002,
        message: t("lib.ipcMock.k1")
      };
    }
    const mockUser: UserInfo = {
      id: 1,
      username,
      role: username.includes('guest') ? 'guest' : 'admin',
      is_permanent: !username.includes('guest')
    };
    const token = `mock-token-${Date.now()}-${Math.random().toString(36).slice(2)}`;
    console.log(`[Mock] 用户登录成功: ${username}, 角色: ${mockUser.role}`);
    return {
      code: 0,
      message: t("lib.ipcMock.k2"),
      data: {
        user: mockUser,
        token
      }
    };
  },
  async register(args: any): Promise<ApiResponse<{
    user: UserInfo;
    token: string;
  }>> {
    await delay(MOCK_DELAY);
    const username = args?.username || '';
    const password = args?.password || '';
    if (!username || username.length < 3) {
      return {
        code: 4001,
        message: t("lib.ipcMock.k3")
      };
    }
    if (!password || password.length < 6) {
      return {
        code: 4002,
        message: t("lib.ipcMock.k4")
      };
    }
    const mockUser: UserInfo = {
      id: Date.now(),
      username,
      role: 'admin',
      is_permanent: true
    };
    const token = `mock-token-${Date.now()}`;
    console.log(`[Mock] 用户注册成功: ${username}`);
    return {
      code: 0,
      message: t("lib.ipcMock.k5"),
      data: {
        user: mockUser,
        token
      }
    };
  },
  async verifyToken(args: any): Promise<ApiResponse<UserInfo>> {
    await delay(MOCK_DELAY / 2);
    const token = args?.token || '';
    if (!token || !token.startsWith('mock-token-')) {
      return {
        code: 1001,
        message: t("lib.ipcMock.k6")
      };
    }
    const storedUser = localStorage.getItem('nexterm-auth');
    if (storedUser) {
      try {
        const parsed = JSON.parse(storedUser);
        if (parsed.state?.user) {
          return {
            code: 0,
            message: t("lib.ipcMock.k7"),
            data: parsed.state.user
          };
        }
      } catch (e) {
        // ignore parse error
      }
    }
    return {
      code: 1001,
      message: t("lib.ipcMock.k6")
    };
  },
  async logout(): Promise<ApiResponse<null>> {
    await delay(MOCK_DELAY / 2);
    console.log('[Mock] 用户已登出');
    return {
      code: 0,
      message: t("lib.ipcMock.k8")
    };
  },
  async getPermissions(args: any): Promise<ApiResponse<BackendPermission[]>> {
    await delay(MOCK_DELAY);
    const role = args?.role || 'admin';
    const makePerm = (resource: string, can_read: boolean, can_write: boolean, can_delete: boolean, can_modify: boolean): BackendPermission => ({
      id: 0,
      role,
      resource,
      can_read,
      can_write,
      can_delete,
      can_modify
    });
    const full = (r: string) => makePerm(r, true, true, true, true);
    const readOnly = (r: string) => makePerm(r, true, false, false, false);
    const noAccess = (r: string) => makePerm(r, false, false, false, false);
    const adminPermissions: BackendPermission[] = [full('home'), full('home_news'), full('home_todo'), full('home_log'), full('home_timer'), full('ai_chat'), full('knowledge'), full('terminal'), full('terminal_manual'), full('terminal_yuancode'), full('terminal_linux'), full('game'), full('profile_settings'), full('recycle_bin'), full('search'), full('xin'), full('spyglass')];
    const userPermissions: BackendPermission[] = [full('home'), full('home_news'), full('home_todo'), full('home_log'), full('home_timer'), makePerm('ai_chat', true, true, true, false), makePerm('knowledge', true, true, true, false), full('terminal'), full('terminal_manual'), full('terminal_yuancode'), full('terminal_linux'), makePerm('game', true, false, false, false), makePerm('profile_settings', true, true, false, false), makePerm('recycle_bin', true, true, true, false), full('search'), full('xin'), makePerm('spyglass', true, true, false, true)];
    const guestPermissions: BackendPermission[] = [readOnly('home'), readOnly('home_news'), readOnly('home_todo'), readOnly('home_log'), readOnly('home_timer'), makePerm('ai_chat', true, true, false, false), readOnly('knowledge'), noAccess('terminal'), readOnly('terminal_manual'), readOnly('terminal_yuancode'), readOnly('terminal_linux'), readOnly('game'), readOnly('profile_settings'), noAccess('recycle_bin'), readOnly('search'), readOnly('xin'), noAccess('spyglass')];
    const permissionsMap: Record<string, BackendPermission[]> = {
      admin: adminPermissions,
      user: userPermissions,
      guest: guestPermissions
    };
    const permissions = permissionsMap[role] || adminPermissions;
    return {
      code: 0,
      message: t("lib.ipcMock.k9"),
      data: permissions
    };
  },
  async verifyRecoveryPhrase(args: any): Promise<ApiResponse<boolean>> {
    await delay(MOCK_DELAY);
    const words = args?.words || args || [];
    console.log('[Mock] 验证恢复短语:', words);
    return {
      code: 0,
      message: t("lib.ipcMock.k7"),
      data: true
    };
  },
  async resetPassword(_args: any): Promise<ApiResponse<null>> {
    await delay(MOCK_DELAY);
    console.log('[Mock] 密码重置成功');
    return {
      code: 0,
      message: t("lib.ipcMock.k10")
    };
  }
};

// ==================== Home Mock ====================

export const mockHome = {
  async getTodos(_args: any): Promise<ApiResponse<TodoItem[]>> {
    await delay(MOCK_DELAY);
    const mockTodos: TodoItem[] = [{
      id: 1,
      user_id: 1,
      title: t("lib.ipcMock.k11"),
      description: t("lib.ipcMock.k12"),
      completed: false,
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString(),
      priority: 'high'
    }, {
      id: 2,
      user_id: 1,
      title: t("lib.ipcMock.k13"),
      description: '',
      completed: true,
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString(),
      priority: 'medium'
    }, {
      id: 3,
      user_id: 1,
      title: t("lib.ipcMock.k14"),
      description: t("lib.ipcMock.k15"),
      completed: false,
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString(),
      priority: 'high'
    }];
    return {
      code: 0,
      message: t("lib.ipcMock.k16"),
      data: mockTodos
    };
  },
  async createTodo(args: any): Promise<ApiResponse<TodoItem>> {
    await delay(MOCK_DELAY);
    const now = new Date().toISOString();
    const newTodo: TodoItem = {
      id: Date.now(),
      user_id: 1,
      title: args?.title || '',
      description: args?.description,
      completed: false,
      created_at: now,
      updated_at: now,
      priority: args?.priority || 'medium'
    };
    return {
      code: 0,
      message: t("lib.ipcMock.k17"),
      data: newTodo
    };
  },
  async updateTodo(args: any): Promise<ApiResponse<TodoItem>> {
    await delay(MOCK_DELAY);
    const id = args?.id || 0;
    return {
      code: 0,
      message: t("lib.ipcMock.k18"),
      data: {
        id,
        ...args
      } as TodoItem
    };
  },
  async deleteTodo(args: any): Promise<ApiResponse<null>> {
    await delay(MOCK_DELAY);
    console.log(`[Mock] 删除待办: ${args?.id}`);
    return {
      code: 0,
      message: t("lib.ipcMock.k19")
    };
  },
  async getNews(): Promise<ApiResponse<NewsItem[]>> {
    await delay(MOCK_DELAY);
    const mockNews: NewsItem[] = [{
      id: 1,
      title: t("lib.ipcMock.k20"),
      summary: t("lib.ipcMock.k21"),
      content: '',
      source: t("components.GroupChatOrchestrationPanel.k2"),
      published_at: new Date().toISOString(),
      read: true,
      category: 'system'
    }];
    return {
      code: 0,
      message: t("lib.ipcMock.k22"),
      data: mockNews
    };
  },
  async getNewsByCategory(_args: any): Promise<ApiResponse<NewsItem[]>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k23"),
      data: []
    };
  },
  async fetchNews(): Promise<ApiResponse<number>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k24"),
      data: 0
    };
  },
  async addNews(args: any): Promise<ApiResponse<NewsItem>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k25"),
      data: {
        id: Date.now(),
        ...args
      } as NewsItem
    };
  },
  async markNewsRead(_args: any): Promise<ApiResponse<null>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k26")
    };
  },
  async clearOldNews(): Promise<ApiResponse<null>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k27")
    };
  },
  async toggleNewsFavorite(args: any): Promise<ApiResponse<null>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: args.isFavorite ? t("lib.ipcMock.k28") : t("lib.ipcMock.k29")
    };
  },
  async getPendingDeleteNews(): Promise<ApiResponse<any[]>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k30"),
      data: []
    };
  },
  async getLogs(args: any): Promise<ApiResponse<Array<{
    id: number;
    content: string;
    timestamp: string;
  }>>> {
    await delay(MOCK_DELAY);
    const date = args?.date || new Date().toISOString().split('T')[0];
    const mockLogs = [{
      id: 1,
      content: t("lib.ipcMock.k31", {
        date: date
      }),
      timestamp: new Date().toISOString()
    }, {
      id: 2,
      content: t("lib.ipcMock.k32"),
      timestamp: new Date(Date.now() - 3600000).toISOString()
    }];
    return {
      code: 0,
      message: t("lib.ipcMock.k33"),
      data: mockLogs
    };
  },
  async saveLog(_args: any): Promise<ApiResponse<null>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k34")
    };
  },
  async deleteLog(_args: any): Promise<ApiResponse<null>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k35")
    };
  },
  async getTimers(): Promise<ApiResponse<any[]>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k36"),
      data: []
    };
  },
  async createTimer(args: any): Promise<ApiResponse<any>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k37"),
      data: {
        id: Date.now(),
        ...args
      }
    };
  },
  async updateTimerState(_args: any): Promise<ApiResponse<null>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k38")
    };
  },
  async deleteTimer(_args: any): Promise<ApiResponse<null>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k39")
    };
  },
  async timerAction(_args: any): Promise<ApiResponse<null>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k40")
    };
  }
};

// ==================== AI Mock ====================

export const mockAI = {
  async getModels(): Promise<ApiResponse<AIModel[]>> {
    await delay(MOCK_DELAY);
    const mockModels: AIModel[] = [{
      id: 1,
      name: 'GPT-4',
      provider: 'OpenAI',
      type: 'api',
      status: 'active',
      model_name: 'gpt-4'
    }, {
      id: 2,
      name: 'Llama 3 8B',
      provider: 'Ollama',
      type: 'local',
      status: 'active',
      model_name: 'llama3:8b'
    }, {
      id: 3,
      name: 'Claude 3',
      provider: 'Anthropic',
      type: 'api',
      status: 'inactive',
      model_name: 'claude-3-opus'
    }];
    return {
      code: 0,
      message: t("lib.ipcMock.k41"),
      data: mockModels
    };
  },
  async createModel(args: any): Promise<ApiResponse<AIModel>> {
    await delay(MOCK_DELAY);
    const newModel: AIModel = {
      id: Date.now(),
      name: args?.name || '',
      provider: args?.provider || t("lib.ipcMock.k42"),
      type: args?.type || 'local',
      status: 'active',
      model_name: args?.model_name || ''
    };
    return {
      code: 0,
      message: t("lib.ipcMock.k43"),
      data: newModel
    };
  },
  async updateModel(_args: any): Promise<ApiResponse<null>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k44")
    };
  },
  async deleteModel(args: any): Promise<ApiResponse<null>> {
    await delay(MOCK_DELAY);
    console.log(`[Mock] 删除模型: ${args?.id}`);
    return {
      code: 0,
      message: t("lib.ipcMock.k19")
    };
  },
  async getAgents(): Promise<ApiResponse<any[]>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k45"),
      data: []
    };
  },
  async createAgent(args: any): Promise<ApiResponse<any>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k46"),
      data: {
        id: Date.now(),
        ...args
      }
    };
  },
  async deleteAgent(_args: any): Promise<ApiResponse<null>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k47")
    };
  },
  async getSessions(): Promise<ApiResponse<ChatSession[]>> {
    await delay(MOCK_DELAY);
    const mockSessions: ChatSession[] = [{
      id: 1,
      name: t("lib.ipcMock.k48"),
      type: 'single',
      model_id: 1,
      model_name: 'GPT-4',
      last_message: t("lib.ipcMock.k49"),
      last_message_at: new Date().toISOString(),
      unread_count: 2,
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString()
    }, {
      id: 2,
      name: t("lib.ipcMock.k50"),
      type: 'single',
      model_id: 2,
      model_name: 'Llama 3',
      last_message: t("lib.ipcMock.k51"),
      unread_count: 0,
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString()
    }];
    return {
      code: 0,
      message: t("lib.ipcMock.k52"),
      data: mockSessions
    };
  },
  async getMessages(_args: any): Promise<ApiResponse<any[]>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k53"),
      data: []
    };
  },
  async getParticipants(_args: any): Promise<ApiResponse<any[]>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k54"),
      data: []
    };
  },
  async sendMessage(args: any): Promise<ApiResponse<{
    id: number;
    content: string;
    role: string;
  }>> {
    await delay(MOCK_DELAY * 2);
    const sessionId = args?.conversation_id || args?.sessionId || 0;
    const content = args?.content || '';
    console.log(`[Mock] 发送消息到会话 ${sessionId}: ${content}`);
    const mockResponse = t("lib.ipcMock.k55", {
      arg0: content.slice(0, 20)
    });
    return {
      code: 0,
      message: t("lib.ipcMock.k56"),
      data: {
        id: Date.now(),
        content: mockResponse,
        role: 'assistant'
      }
    };
  },
  async startGroupChat(args: any): Promise<ApiResponse<{
    sessionId: number;
    orchestration: GroupChatOrchestration;
  }>> {
    await delay(MOCK_DELAY);
    const sessionId = Date.now();
    const orchestration: GroupChatOrchestration = {
      session_id: sessionId,
      current_round: 0,
      max_rounds: args?.maxRounds || 10,
      total_token_budget: args?.tokenBudget || 50000,
      used_tokens: 0,
      status: 'discussing',
      timeout_models: [],
      started_at: new Date().toISOString()
    };
    console.log(`[Mock] 启动群聊编排: ${args?.name}, 最大轮次: ${orchestration.max_rounds}`);
    return {
      code: 0,
      message: t("lib.ipcMock.k57"),
      data: {
        sessionId,
        orchestration
      }
    };
  },
  async getOrchestrationStatus(args: any): Promise<ApiResponse<GroupChatOrchestration>> {
    await delay(MOCK_DELAY / 2);
    const sessionId = args?.sessionId || args?.conversation_id || 0;
    const mockOrchestration: GroupChatOrchestration = {
      session_id: sessionId,
      current_round: Math.floor(Math.random() * 10),
      max_rounds: 10,
      total_token_budget: 50000,
      used_tokens: Math.floor(Math.random() * 50000),
      status: ['discussing', 'converged', 'summarizing'][Math.floor(Math.random() * 3)] as GroupChatOrchestration['status'],
      timeout_models: [],
      started_at: new Date().toISOString()
    };
    return {
      code: 0,
      message: t("lib.ipcMock.k58"),
      data: mockOrchestration
    };
  },
  async endGroupChat(args: any): Promise<ApiResponse<{
    summary: string;
    summarizer: string;
  }>> {
    await delay(MOCK_DELAY);
    const sessionId = args?.sessionId || args?.conversation_id || 0;
    console.log(`[Mock] 结束群聊: ${sessionId}`);
    return {
      code: 0,
      message: t("lib.ipcMock.k59"),
      data: {
        summary: t("lib.ipcMock.k60"),
        summarizer: 'GPT-4'
      }
    };
  }
};

// ==================== Knowledge Mock ====================

export const mockKnowledge = {
  async getCategories(): Promise<ApiResponse<KnowledgeCategory[]>> {
    await delay(MOCK_DELAY);
    const mockCategories: KnowledgeCategory[] = [{
      id: 1,
      name: t("lib.ipcMock.k61"),
      item_count: 5,
      level: 0,
      expanded: false
    }, {
      id: 2,
      name: t("lib.ipcMock.k62"),
      item_count: 12,
      level: 0,
      expanded: false
    }, {
      id: 21,
      name: 'Rust',
      parent_id: 2,
      item_count: 3,
      level: 1,
      expanded: false
    }, {
      id: 22,
      name: 'TypeScript',
      parent_id: 2,
      item_count: 4,
      level: 1,
      expanded: false
    }, {
      id: 3,
      name: t("components.AddExtensionModal.k12"),
      item_count: 8,
      level: 0,
      expanded: false
    }, {
      id: 4,
      name: t("lib.ipcMock.k63"),
      item_count: 6,
      level: 0,
      expanded: false
    }];
    return {
      code: 0,
      message: t("lib.ipcMock.k64"),
      data: mockCategories
    };
  },
  async addCategory(args: any): Promise<ApiResponse<KnowledgeCategory>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k65"),
      data: {
        id: Date.now(),
        ...args
      } as KnowledgeCategory
    };
  },
  async deleteCategory(_args: any): Promise<ApiResponse<null>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k66")
    };
  },
  async getItems(args: any): Promise<ApiResponse<KnowledgeItem[]>> {
    await delay(MOCK_DELAY);
    const categoryId = args?.category_id || args?.categoryId;
    const mockItems: KnowledgeItem[] = [{
      id: 1,
      category_id: 21,
      title: t("lib.ipcMock.k67"),
      source: 'link',
      url: 'https://doc.rust-lang.org/',
      preview: t("lib.ipcMock.k68"),
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString()
    }, {
      id: 2,
      category_id: 21,
      title: 'Rust by Example',
      source: 'file',
      file_path: '/docs/rust-by-example.pdf',
      preview: t("lib.ipcMock.k69"),
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString()
    }, {
      id: 3,
      category_id: 22,
      title: 'TypeScript Handbook',
      source: 'link',
      url: 'https://www.typescriptlang.org/docs/',
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString()
    }];
    const filtered = categoryId ? mockItems.filter(item => item.category_id === categoryId) : mockItems;
    return {
      code: 0,
      message: t("lib.ipcMock.k70"),
      data: filtered
    };
  },
  async createItem(args: any): Promise<ApiResponse<KnowledgeItem>> {
    await delay(MOCK_DELAY);
    const newItem: KnowledgeItem = {
      id: Date.now(),
      category_id: args?.category_id || 1,
      title: args?.title || '',
      source: args?.source || 'text',
      file_path: args?.file_path,
      url: args?.url,
      preview: args?.preview,
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString()
    };
    return {
      code: 0,
      message: t("lib.ipcMock.k71"),
      data: newItem
    };
  },
  async deleteItem(args: any): Promise<ApiResponse<null>> {
    await delay(MOCK_DELAY);
    console.log(`[Mock] 删除知识条目: ${args?.id}`);
    return {
      code: 0,
      message: t("lib.ipcMock.k19")
    };
  },
  async search(args: any): Promise<ApiResponse<KnowledgeItem[]>> {
    await delay(MOCK_DELAY);
    const query = args?.keyword || args?.query || '';
    console.log(`[Mock] 搜索知识库: ${query}`);
    return {
      code: 0,
      message: t("lib.ipcMock.k72"),
      data: []
    };
  },
  async importFolder(args: any): Promise<ApiResponse<{
    categories: KnowledgeCategory[];
    items: KnowledgeItem[];
  }>> {
    await delay(MOCK_DELAY * 2);
    const folderPath = args?.folder_path || args?.folderPath || '';
    const categoryId = args?.category_id || args?.categoryId || 1;
    console.log(`[Mock] 导入文件夹: ${folderPath} -> 分类 ${categoryId}`);
    const mockCategories: KnowledgeCategory[] = [{
      id: Date.now(),
      name: t("lib.ipcMock.k73"),
      parent_id: categoryId,
      item_count: 3,
      level: 1,
      expanded: false
    }];
    const mockItems: KnowledgeItem[] = [{
      id: Date.now() + 1,
      category_id: Date.now(),
      title: t("lib.ipcMock.k74"),
      source: 'file',
      file_path: t("lib.ipcMock.k75", {
        folderPath: folderPath
      }),
      preview: '',
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString()
    }, {
      id: Date.now() + 2,
      category_id: Date.now(),
      title: t("lib.ipcMock.k76"),
      source: 'text',
      file_path: t("lib.ipcMock.k77", {
        folderPath: folderPath
      }),
      preview: '',
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString()
    }, {
      id: Date.now() + 3,
      category_id: Date.now(),
      title: t("lib.ipcMock.k78"),
      source: 'text',
      file_path: t("lib.ipcMock.k79", {
        folderPath: folderPath
      }),
      preview: '',
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString()
    }];
    return {
      code: 0,
      message: t("lib.ipcMock.k80", {
        length: mockCategories.length,
        arg0: mockItems.length
      }),
      data: {
        categories: mockCategories,
        items: mockItems
      }
    };
  }
};

// ==================== Recycle Bin Mock ====================

export const mockRecycleBin = {
  async getItems(): Promise<ApiResponse<RecycledItem[]>> {
    await delay(MOCK_DELAY);
    const mockItems: RecycledItem[] = [{
      id: 1,
      original_id: 101,
      type: 'todo',
      title: t("lib.ipcMock.k81"),
      deleted_at: new Date(Date.now() - 86400000).toISOString(),
      deleted_by: 1,
      restoreable: true
    }, {
      id: 2,
      original_id: 201,
      type: 'knowledge_item',
      title: t("lib.ipcMock.k82"),
      deleted_at: new Date(Date.now() - 172800000).toISOString(),
      deleted_by: 1,
      restoreable: true
    }, {
      id: 3,
      original_id: 301,
      type: 'chat_session',
      title: t("lib.ipcMock.k83"),
      deleted_at: new Date(Date.now() - 259200000).toISOString(),
      deleted_by: 1,
      restoreable: true
    }];
    return {
      code: 0,
      message: t("lib.ipcMock.k84"),
      data: mockItems
    };
  },
  async moveToRecycle(_args: any): Promise<ApiResponse<null>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k85")
    };
  },
  async restore(args: any): Promise<ApiResponse<null>> {
    await delay(MOCK_DELAY);
    const ids = args?.ids || [];
    console.log(`[Mock] 恢复项目: ${Array.isArray(ids) ? ids.join(', ') : ids}`);
    return {
      code: 0,
      message: t("lib.ipcMock.k86")
    };
  },
  async permanentDelete(args: any): Promise<ApiResponse<null>> {
    await delay(MOCK_DELAY);
    const ids = args?.ids || [];
    console.log(`[Mock] 永久删除: ${Array.isArray(ids) ? ids.join(', ') : ids}`);
    return {
      code: 0,
      message: t("lib.ipcMock.k87")
    };
  },
  async cleanupExpired(): Promise<ApiResponse<null>> {
    await delay(MOCK_DELAY);
    console.log('[Mock] 清理过期回收站项目');
    return {
      code: 0,
      message: t("lib.ipcMock.k88")
    };
  },
  async empty(): Promise<ApiResponse<null>> {
    await delay(MOCK_DELAY);
    console.log('[Mock] 清空回收站');
    return {
      code: 0,
      message: t("lib.ipcMock.k89")
    };
  }
};

// ==================== Terminal Mock ====================

export const mockTerminal = {
  async createSession(args: any): Promise<ApiResponse<{
    sessionId: number;
  }>> {
    await delay(MOCK_DELAY);
    const type = args?.type || 'terminal';
    console.log(`[Mock] 创建终端会话: ${type}`);
    return {
      code: 0,
      message: t("lib.ipcMock.k90"),
      data: {
        sessionId: Date.now()
      }
    };
  },
  async executeCommand(args: any): Promise<ApiResponse<{
    output: string;
  }>> {
    await delay(MOCK_DELAY);
    const sessionId = args?.session_id || args?.sessionId || 0;
    const command = args?.input || args?.command || '';
    console.log(`[Mock] 执行命令 [${sessionId}]: ${command}`);
    if (command === 'pwd') {
      return {
        code: 0,
        message: '',
        data: {
          output: 'C:\\Users\\User\\NexTerm'
        }
      };
    }
    if (command === 'ls' || command === 'dir') {
      return {
        code: 0,
        message: '',
        data: {
          output: 'Documents  Downloads  Desktop  NexTerm'
        }
      };
    }
    if (command.startsWith('echo ')) {
      return {
        code: 0,
        message: '',
        data: {
          output: command.slice(5)
        }
      };
    }
    return {
      code: 0,
      message: '',
      data: {
        output: t("lib.ipcMock.k91", {
          command: command
        })
      }
    };
  },
  async resize(_args: any): Promise<ApiResponse<null>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k92")
    };
  },
  async closeSession(args: any): Promise<ApiResponse<null>> {
    await delay(MOCK_DELAY / 2);
    const sessionId = args?.session_id || args?.sessionId || 0;
    console.log(`[Mock] 关闭终端会话: ${sessionId}`);
    return {
      code: 0,
      message: t("lib.ipcMock.k93")
    };
  }
};

// ==================== Profile Mock ====================

export const mockProfile = {
  async getProfile(_args: any): Promise<ApiResponse<any>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k94"),
      data: null
    };
  },
  async setProfile(args: any): Promise<ApiResponse<any>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k95"),
      data: args
    };
  },
  async getResumes(): Promise<ApiResponse<any[]>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k96"),
      data: []
    };
  },
  async addResume(args: any): Promise<ApiResponse<any>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k97"),
      data: {
        id: Date.now(),
        ...args
      }
    };
  },
  async updateResume(args: any): Promise<ApiResponse<any>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k98"),
      data: {
        id: args.id,
        title: args.title,
        content: args.content
      }
    };
  },
  async deleteResume(_args: any): Promise<ApiResponse<null>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k99")
    };
  },
  async savePersonalInfo(args: any): Promise<ApiResponse<null>> {
    await delay(MOCK_DELAY);
    localStorage.setItem('nt_mock_personal_info', args.json);
    return {
      code: 0,
      message: t("lib.ipcMock.k100")
    };
  },
  async getPersonalInfo(): Promise<ApiResponse<string | null>> {
    await delay(MOCK_DELAY);
    const data = localStorage.getItem('nt_mock_personal_info') || null;
    return {
      code: 0,
      message: t("lib.ipcMock.k101"),
      data
    };
  },
  async getTimeline(): Promise<ApiResponse<any[]>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k102"),
      data: []
    };
  },
  async addTimelineItem(args: any): Promise<ApiResponse<any>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k103"),
      data: {
        id: Date.now(),
        ...args
      }
    };
  },
  async updateTimelineItem(args: any): Promise<ApiResponse<any>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k104"),
      data: {
        id: args.id,
        date: args.date,
        title: args.title,
        description: args.description
      }
    };
  },
  async deleteTimelineItem(_args: any): Promise<ApiResponse<null>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k105")
    };
  },
  async getRandomQuote(): Promise<ApiResponse<any>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k106"),
      data: null
    };
  },
  async addQuote(args: any): Promise<ApiResponse<any>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k107"),
      data: {
        id: Date.now(),
        ...args
      }
    };
  },
  async changePassword(_args: any): Promise<ApiResponse<null>> {
    await delay(MOCK_DELAY);
    console.log('[Mock] 密码修改成功');
    return {
      code: 0,
      message: t("lib.ipcMock.k108")
    };
  },
  async changeUsername(args: any): Promise<ApiResponse<null>> {
    await delay(MOCK_DELAY);
    const newUsername = args?.request?.new_username || args?.newUsername || '';
    console.log(`[Mock] 用户名修改为: ${newUsername}`);
    return {
      code: 0,
      message: t("lib.ipcMock.k109")
    };
  },
  async createTempAccount(args: any): Promise<ApiResponse<{
    username: string;
    password: string;
    expires_at: number;
  }>> {
    await delay(MOCK_DELAY);
    const duration_hours = args?.duration_hours || 1;
    const now = Date.now();
    const expires_at = now + duration_hours * 3600 * 1000;
    const username = args?.username?.trim() || `guest_${Date.now()}`;
    console.log(`[Mock] 临时账号创建成功: ${username}, 有效期: ${duration_hours}小时`);
    return {
      code: 0,
      message: t("lib.ipcMock.k110"),
      data: {
        username,
        password: 'TempPass123!',
        expires_at
      }
    };
  },
  async getNewsSources(): Promise<ApiResponse<any[]>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k111"),
      data: []
    };
  },
  async addNewsSource(args: any): Promise<ApiResponse<any>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k112"),
      data: {
        id: Date.now(),
        ...args
      }
    };
  },
  async deleteNewsSource(_args: any): Promise<ApiResponse<null>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k113")
    };
  }
};

// ==================== System Mock ====================

export const mockSystem = {
  async getInfo(): Promise<ApiResponse<any>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k114"),
      data: {
        platform: 'windows',
        arch: 'x64',
        hostname: 'nexterm-pc',
        uptime: 86400,
        memory: {
          total: 17179869184,
          free: 8589934592,
          used: 8589934592
        },
        disk: {
          total: 500107862016,
          free: 250053931008,
          used: 250053931008
        }
      }
    };
  },
  async setConfig(_args: any): Promise<ApiResponse<null>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k115")
    };
  },
  async getAllConfigs(): Promise<ApiResponse<any[]>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k116"),
      data: []
    };
  },
  async openFile(_args: any): Promise<ApiResponse<null>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k117")
    };
  },
  async openUrl(_args: any): Promise<ApiResponse<null>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k118")
    };
  },
  async getAppInfo(): Promise<ApiResponse<any>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k119"),
      data: {
        version: '1.0.0',
        name: 'NexTerm'
      }
    };
  }
};

// ==================== Extension Mock ====================

export const mockExtension = {
  async getEntry(_args: any): Promise<ApiResponse<any>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k120"),
      data: null
    };
  },
  async listModules(): Promise<ApiResponse<any[]>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k121"),
      data: []
    };
  }
};

// ==================== Agent Mock ====================

export const mockAgent = {
  async spawn(args: any): Promise<ApiResponse<any>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k122"),
      data: {
        agent_id: 'agent_' + Date.now(),
        role: args.role || 'assistant',
        status: 'idle'
      }
    };
  },
  async fork(args: any): Promise<ApiResponse<any>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k123"),
      data: {
        agent_id: 'agent_' + Date.now(),
        parent_id: args.parent_id
      }
    };
  },
  async list(): Promise<ApiResponse<any[]>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k124"),
      data: [{
        agent_id: 'agent_1',
        role: 'assistant',
        status: 'idle'
      }]
    };
  },
  async listAll(): Promise<ApiResponse<any[]>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k125"),
      data: []
    };
  },
  async status(args: any): Promise<ApiResponse<any>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k126"),
      data: {
        agent_id: args.agent_id,
        status: 'idle'
      }
    };
  },
  async abort(args: any): Promise<ApiResponse<void>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k127", {
        agent_id: args.agent_id
      })
    };
  },
  async sendMessage(args: any): Promise<ApiResponse<any>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k128"),
      data: {
        id: 'msg_' + Date.now(),
        from: args.from_agent_id,
        to: args.to_agent_id
      }
    };
  },
  async receiveMessages(_args: any): Promise<ApiResponse<any[]>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k129"),
      data: []
    };
  },
  async listRoles(): Promise<ApiResponse<any[]>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k130"),
      data: [{
        name: 'assistant',
        description: t("lib.ipcMock.k131")
      }]
    };
  },
  async getCount(): Promise<ApiResponse<number>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k132"),
      data: 1
    };
  },
  async cleanup(): Promise<ApiResponse<void>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k133")
    };
  }
};

// ==================== Sandbox Mock ====================

export const mockSandbox = {
  async create(args: any): Promise<ApiResponse<any>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k134"),
      data: {
        sandbox_id: 'sandbox_' + Date.now(),
        sandbox_type: args.sandbox_type || 'FileSystem',
        state: 'Idle'
      }
    };
  },
  async list(): Promise<ApiResponse<any[]>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k135"),
      data: []
    };
  },
  async listByAgent(_args: any): Promise<ApiResponse<any[]>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k136"),
      data: []
    };
  },
  async get(args: any): Promise<ApiResponse<any>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k137"),
      data: {
        sandbox_id: args.sandbox_id,
        state: 'Idle'
      }
    };
  },
  async execute(_args: any): Promise<ApiResponse<any>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k138"),
      data: {
        exit_code: 0,
        stdout: 'mock output',
        stderr: '',
        duration_ms: 100
      }
    };
  },
  async readFile(_args: any): Promise<ApiResponse<string>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k139"),
      data: 'mock file content'
    };
  },
  async writeFile(_args: any): Promise<ApiResponse<void>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k140")
    };
  },
  async deleteFile(_args: any): Promise<ApiResponse<void>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k141")
    };
  },
  async listFiles(_args: any): Promise<ApiResponse<string[]>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k142"),
      data: []
    };
  },
  async terminate(_args: any): Promise<ApiResponse<void>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k143")
    };
  },
  async cleanup(): Promise<ApiResponse<void>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k144")
    };
  },
  async count(): Promise<ApiResponse<number>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k145"),
      data: 0
    };
  }
};

// ==================== MCP Mock ====================

export const mockMcp = {
  async registerServer(args: any): Promise<ApiResponse<any>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k146"),
      data: {
        server_id: 'mcp_' + Date.now(),
        name: args.name
      }
    };
  },
  async connectServer(args: any): Promise<ApiResponse<any>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k147"),
      data: {
        server_id: args.server_id,
        status: 'connected'
      }
    };
  },
  async disconnectServer(_args: any): Promise<ApiResponse<void>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k148")
    };
  },
  async listServers(): Promise<ApiResponse<any[]>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k149"),
      data: []
    };
  },
  async listAllTools(): Promise<ApiResponse<any[]>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k150"),
      data: []
    };
  },
  async serverStatus(args: any): Promise<ApiResponse<any>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k151"),
      data: {
        server_id: args.server_id,
        status: 'disconnected'
      }
    };
  },
  async callTool(_args: any): Promise<ApiResponse<any>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k152"),
      data: {
        content: [{
          content_type: 'text',
          text: 'mock tool result'
        }]
      }
    };
  },
  // ===== D1 v3.1 Task 3.2.1-3.2.12: 内置 MCP 生态 Mock =====
  async listBuiltinServers(): Promise<ApiResponse<string[]>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: 'ok',
      data: ['github', 'gitlab', 'sqlite', 'postgres', 'slack', 'jira', 'memory', 'websearch']
    };
  },
  async listBuiltinTools(): Promise<ApiResponse<any[]>> {
    await delay(MOCK_DELAY);
    const servers = ['github', 'gitlab', 'sqlite', 'postgres', 'slack', 'jira', 'memory', 'websearch'];
    const tools: any[] = [];
    for (const s of servers) {
      tools.push({
        server_id: s,
        server_name: s,
        tool: { name: `${s}_tool_1`, description: `${s} 工具 1`, input_schema: { type: 'object' } }
      });
    }
    return { code: 0, message: 'ok', data: tools };
  },
  async callBuiltinTool(args: any): Promise<ApiResponse<any>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: 'ok',
      data: {
        content: [{
          type: 'text',
          text: `[Mock] ${args?.server_name}/${args?.tool_name} 调用成功`
        }],
        isError: false
      }
    };
  },
  async configureBuiltin(_args: any): Promise<ApiResponse<null>> {
    await delay(MOCK_DELAY);
    return { code: 0, message: 'ok', data: null };
  },
  async listCallHistory(): Promise<ApiResponse<any[]>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: 'ok',
      data: [{
        server_id: 'memory',
        server_name: 'memory',
        tool_name: 'store',
        arguments: { key: 'k1', value: 'v1' },
        success: true,
        latency_ms: 3,
        result_preview: '{"stored":true}',
        called_at: Date.now() / 1000
      }]
    };
  }
};

// ==================== Skill Mock ====================

export const mockSkill = {
  async addRoot(_args: any): Promise<ApiResponse<void>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k153")
    };
  },
  async setProjectFiles(_args: any): Promise<ApiResponse<void>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k154")
    };
  },
  async loadAll(): Promise<ApiResponse<any>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k155"),
      data: {
        skills: [],
        total: 0,
        enabled: 0
      }
    };
  },
  async get(_args: any): Promise<ApiResponse<any>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k156"),
      data: null
    };
  },
  async list(): Promise<ApiResponse<any[]>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k157"),
      data: []
    };
  },
  async detectImplicit(_args: any): Promise<ApiResponse<any[]>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k158"),
      data: []
    };
  },
  async register(_args: any): Promise<ApiResponse<void>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k159")
    };
  },
  async unregister(_args: any): Promise<ApiResponse<void>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k160")
    };
  },
  async trigger(_args: any): Promise<ApiResponse<any>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k161"),
      data: {
        invocation_id: 'inv_' + Date.now()
      }
    };
  },
  async autoDiscover(): Promise<ApiResponse<any[]>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k162"),
      data: []
    };
  },
  async match(_args: any): Promise<ApiResponse<any[]>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k163"),
      data: []
    };
  },
  async renderContext(_args: any): Promise<ApiResponse<string>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k164"),
      data: ''
    };
  },
  async stats(): Promise<ApiResponse<any>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k165"),
      data: {
        skill_count: 0,
        enabled_count: 0,
        invocation_count: 0
      }
    };
  }
};

// ==================== Compact Mock ====================

export const mockCompact = {
  async configGet(): Promise<ApiResponse<any>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k166"),
      data: {
        strategy: 'sliding_window',
        max_tokens: 8000,
        min_keep_turns: 5
      }
    };
  },
  async configUpdate(_args: any): Promise<ApiResponse<void>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k167")
    };
  },
  async estimate(_args: any): Promise<ApiResponse<number>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k168"),
      data: 1500
    };
  },
  async check(_args: any): Promise<ApiResponse<any>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k169"),
      data: {
        needs_compaction: false,
        token_usage: 1500,
        token_limit: 8000
      }
    };
  },
  async execute(_args: any): Promise<ApiResponse<any>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k170"),
      data: {
        strategy_used: 'sliding_window',
        original_turns: 10,
        compacted_turns: 5
      }
    };
  },
  async session(_args: any): Promise<ApiResponse<any>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k171"),
      data: null
    };
  },
  async reset(_args: any): Promise<ApiResponse<void>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k172")
    };
  }
};

// ==================== Goal Mock ====================

export const mockGoal = {
  async create(args: any): Promise<ApiResponse<any>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k173"),
      data: {
        goal_id: 'goal_' + Date.now(),
        title: args.title,
        status: 'active'
      }
    };
  },
  async update(_args: any): Promise<ApiResponse<void>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k174")
    };
  },
  async delete(_args: any): Promise<ApiResponse<void>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k175")
    };
  },
  async get(_args: any): Promise<ApiResponse<any>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k176"),
      data: null
    };
  },
  async list(_args: any): Promise<ApiResponse<any[]>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k177"),
      data: []
    };
  },
  async complete(_args: any): Promise<ApiResponse<void>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k178")
    };
  },
  async abort(_args: any): Promise<ApiResponse<void>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k179")
    };
  },
  async stats(): Promise<ApiResponse<any>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: t("lib.ipcMock.k180"),
      data: {
        total: 0,
        active: 0,
        completed: 0,
        aborted: 0
      }
    };
  }
};

// ==================== Sync Mock（A5 离线与同步机制 Phase 1-4）====================

export const mockSync = {
  // 内存态模拟数据
  _queue: [] as any[],
  _devices: [] as any[],
  _nextId: 1,
  _conflicts: [] as any[],

  async enqueue(args: any): Promise<ApiResponse<number>> {
    await delay(MOCK_DELAY);
    const id = this._nextId++;
    this._queue.push({
      id,
      table_name: args.table_name,
      record_id: args.record_id,
      operation: args.operation,
      payload: args.payload,
      device_id: args.device_id ?? null,
      sync_status: 'pending',
      retry_count: 0,
      last_error: null,
      created_at: new Date().toISOString(),
      synced_at: null,
    });
    return { code: 0, message: 'ok', data: id };
  },

  async fetchPending(args: any): Promise<ApiResponse<any[]>> {
    await delay(MOCK_DELAY);
    const limit = args?.limit ?? 100;
    const pending = this._queue.filter((q) => q.sync_status === 'pending').slice(0, limit);
    return { code: 0, message: 'ok', data: pending };
  },

  async markSynced(args: any): Promise<ApiResponse<null>> {
    await delay(MOCK_DELAY);
    const item = this._queue.find((q) => q.id === args.id);
    if (item) {
      item.sync_status = 'synced';
      item.synced_at = new Date().toISOString();
    }
    return { code: 0, message: 'ok', data: null };
  },

  async markFailed(args: any): Promise<ApiResponse<null>> {
    await delay(MOCK_DELAY);
    const item = this._queue.find((q) => q.id === args.id);
    if (item) {
      item.sync_status = 'failed';
      item.last_error = args.error_msg;
      item.retry_count += 1;
    }
    return { code: 0, message: 'ok', data: null };
  },

  async getQueueStats(): Promise<ApiResponse<any>> {
    await delay(MOCK_DELAY);
    const stats = {
      pending: this._queue.filter((q) => q.sync_status === 'pending').length,
      syncing: this._queue.filter((q) => q.sync_status === 'syncing').length,
      synced: this._queue.filter((q) => q.sync_status === 'synced').length,
      failed: this._queue.filter((q) => q.sync_status === 'failed').length,
      conflict: this._queue.filter((q) => q.sync_status === 'conflict').length,
    };
    return { code: 0, message: 'ok', data: stats };
  },

  async registerDevice(args: any): Promise<ApiResponse<null>> {
    await delay(MOCK_DELAY);
    if (args.is_current) {
      this._devices.forEach((d) => (d.is_current_device = 0));
    }
    this._devices.push({
      id: args.id,
      device_name: args.device_name,
      device_type: args.device_type,
      public_key: args.public_key,
      registered_at: new Date().toISOString(),
      last_seen_at: null,
      last_sync_at: null,
      is_current_device: args.is_current ? 1 : 0,
    });
    return { code: 0, message: 'ok', data: null };
  },

  async listDevices(): Promise<ApiResponse<any[]>> {
    await delay(MOCK_DELAY);
    return { code: 0, message: 'ok', data: [...this._devices] };
  },

  async unregisterDevice(args: any): Promise<ApiResponse<null>> {
    await delay(MOCK_DELAY);
    this._devices = this._devices.filter((d) => d.id !== args.device_id);
    return { code: 0, message: 'ok', data: null };
  },

  async testTransport(args: any): Promise<ApiResponse<boolean>> {
    await delay(MOCK_DELAY);
    const backendType = args?.backend_type;
    if (!['webdav', 's3'].includes(backendType)) {
      return { code: 400, message: '不支持的传输后端类型', data: false };
    }
    return { code: 0, message: 'ok', data: true };
  },

  async runOnce(): Promise<ApiResponse<any>> {
    await delay(MOCK_DELAY);
    const result = {
      total_pending: this._queue.filter((q) => q.sync_status === 'pending').length,
      succeeded: this._queue.filter((q) => q.sync_status === 'pending').length,
      failed: 0,
      conflicts: 0,
      retried: 0,
      duration_ms: 50,
      errors: [] as string[],
    };
    this._queue.forEach((q) => {
      if (q.sync_status === 'pending') {
        q.sync_status = 'synced';
        q.synced_at = new Date().toISOString();
      }
    });
    return { code: 0, message: 'ok', data: result };
  },

  async getStatus(): Promise<ApiResponse<any>> {
    return this.getQueueStats();
  },

  async listConflicts(): Promise<ApiResponse<any[]>> {
    await delay(MOCK_DELAY);
    const conflicts = this._queue.filter((q) => q.sync_status === 'conflict');
    return { code: 0, message: 'ok', data: conflicts };
  },

  async resolveConflict(args: any): Promise<ApiResponse<null>> {
    await delay(MOCK_DELAY);
    const item = this._queue.find((q) => q.id === args.id);
    if (item) {
      item.sync_status = 'pending';
      item.last_error = null;
      item.retry_count = 0;
      if (args.resolved_payload) {
        item.payload = args.resolved_payload;
      }
    }
    return { code: 0, message: 'ok', data: null };
  },

  async ecdhGenerateKeypair(): Promise<ApiResponse<any>> {
    await delay(MOCK_DELAY);
    // Mock 公钥（base64 编码的 65 字节 NIST P-256 未压缩点）
    const mockPublicKey = 'BHDx' + Math.random().toString(36).slice(2).padEnd(60, 'A').slice(0, 60);
    return {
      code: 0,
      message: 'ok',
      data: { private_key_b64: '', public_key_b64: mockPublicKey },
    };
  },

  async e2eeEncrypt(args: any): Promise<ApiResponse<any>> {
    await delay(MOCK_DELAY);
    const mockNonce = Array.from({ length: 12 }, () =>
      Math.floor(Math.random() * 256).toString(16).padStart(2, '0'),
    ).join('');
    const mockCiphertext = btoa(unescape(encodeURIComponent(args.payload))).slice(0, 200);
    return {
      code: 0,
      message: 'ok',
      data: {
        ephemeral_public_key: 'BHDx' + Math.random().toString(36).slice(2).padEnd(60, 'A').slice(0, 60),
        nonce: mockNonce,
        ciphertext: mockCiphertext,
      },
    };
  },

  async e2eeValidate(args: any): Promise<ApiResponse<boolean>> {
    await delay(MOCK_DELAY);
    const enc = args?.encrypted;
    const valid = !!(
      enc &&
      typeof enc.ephemeral_public_key === 'string' &&
      typeof enc.nonce === 'string' &&
      typeof enc.ciphertext === 'string'
    );
    return { code: 0, message: 'ok', data: valid };
  },

  // Phase 3 网络状态检测 — Mock 模式下用浏览器 navigator.onLine 模拟
  // 开发者可通过浏览器断网 / DevTools Network offline 触发离线横幅
  async getNetworkStatus(): Promise<ApiResponse<{ is_online: boolean; last_checked: number }>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: 'ok',
      data: {
        is_online: typeof navigator !== 'undefined' ? navigator.onLine : true,
        last_checked: Date.now(),
      },
    };
  },

  async checkNetworkNow(): Promise<ApiResponse<{ is_online: boolean; last_checked: number }>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: 'ok',
      data: {
        is_online: typeof navigator !== 'undefined' ? navigator.onLine : true,
        last_checked: Date.now(),
      },
    };
  },
};

// ==================== Cloud API Mock (D1 v3.1 Task 3.5) ====================
// 规范：项目核心设计意图 §三 — 仅允许云端 provider，本地底层智能模型不会出现

const MOCK_CLOUD_PROVIDERS = [
  {
    provider: 'openai',
    display_name: 'OpenAI (GPT-4o / o1 / o3-mini)',
    default_api_url: 'https://api.openai.com/v1',
    default_model: 'gpt-4o-mini',
    available_models: ['gpt-4o', 'gpt-4o-mini', 'o1', 'o1-mini', 'o3-mini'],
  },
  {
    provider: 'anthropic',
    display_name: 'Anthropic (Claude Sonnet/Opus/Haiku)',
    default_api_url: 'https://api.anthropic.com',
    default_model: 'claude-3-5-sonnet-20241022',
    available_models: ['claude-3-5-sonnet-20241022', 'claude-3-5-haiku-20241022', 'claude-3-opus-20240229'],
  },
  {
    provider: 'deepseek',
    display_name: 'DeepSeek (Chat / Reasoner / V3)',
    default_api_url: 'https://api.deepseek.com',
    default_model: 'deepseek-chat',
    available_models: ['deepseek-chat', 'deepseek-reasoner'],
  },
  {
    provider: 'azure',
    display_name: 'Azure OpenAI',
    default_api_url: '',
    default_model: 'gpt-4o',
    available_models: ['gpt-4o', 'gpt-4o-mini', 'gpt-35-turbo'],
  },
  {
    provider: 'moonshot',
    display_name: 'Moonshot Kimi',
    default_api_url: 'https://api.moonshot.cn/v1',
    default_model: 'moonshot-v1-8k',
    available_models: ['moonshot-v1-8k', 'moonshot-v1-32k', 'moonshot-v1-128k'],
  },
  {
    provider: 'zhipu',
    display_name: '智谱 GLM',
    default_api_url: 'https://open.bigmodel.cn/api/paas/v4',
    default_model: 'glm-4',
    available_models: ['glm-4', 'glm-4-air', 'glm-4-flash'],
  },
  {
    provider: 'qwen',
    display_name: '阿里通义千问',
    default_api_url: 'https://dashscope.aliyuncs.com/api/v1',
    default_model: 'qwen-plus',
    available_models: ['qwen-plus', 'qwen-max', 'qwen-turbo'],
  },
  {
    provider: 'custom',
    display_name: '自定义（OpenAI 兼容）',
    default_api_url: '',
    default_model: '',
    available_models: [],
  },
];

let mockCloudApiKeys: any[] = [
  {
    id: 1,
    provider: 'openai',
    display_name: 'OpenAI 测试 Key',
    api_url: 'https://api.openai.com/v1',
    is_cloud_only: true,
    is_enabled: true,
    has_api_key: true,
    last_used_at: null,
    created_at: Math.floor(Date.now() / 1000) - 3600,
    updated_at: Math.floor(Date.now() / 1000) - 3600,
  },
];

export const mockCloudApi = {
  async list(): Promise<ApiResponse<any[]>> {
    await delay(MOCK_DELAY);
    return { code: 0, message: 'ok', data: mockCloudApiKeys };
  },

  async upsert(args: any): Promise<ApiResponse<any>> {
    await delay(MOCK_DELAY);
    const provider = args?.provider || 'openai';
    const providerInfo = MOCK_CLOUD_PROVIDERS.find(p => p.provider === provider);
    const existing = mockCloudApiKeys.find(k => k.provider === provider);
    const now = Math.floor(Date.now() / 1000);
    const row = {
      id: existing?.id || Date.now(),
      provider,
      display_name: args?.display_name || providerInfo?.display_name || provider,
      api_url: args?.api_url || providerInfo?.default_api_url || '',
      is_cloud_only: true,
      is_enabled: args?.is_enabled ?? true,
      has_api_key: !!args?.api_key_plain,
      last_used_at: existing?.last_used_at || null,
      created_at: existing?.created_at || now,
      updated_at: now,
    };
    if (existing) {
      const idx = mockCloudApiKeys.findIndex(k => k.id === existing.id);
      mockCloudApiKeys[idx] = row;
    } else {
      mockCloudApiKeys.push(row);
    }
    return { code: 0, message: 'ok', data: row };
  },

  async setEnabled(args: any): Promise<ApiResponse<null>> {
    await delay(MOCK_DELAY);
    const item = mockCloudApiKeys.find(k => k.id === args?.id);
    if (item) {
      item.is_enabled = !!args?.is_enabled;
      item.updated_at = Math.floor(Date.now() / 1000);
    }
    return { code: 0, message: 'ok', data: null };
  },

  async delete(args: any): Promise<ApiResponse<null>> {
    await delay(MOCK_DELAY);
    mockCloudApiKeys = mockCloudApiKeys.filter(k => k.id !== args?.id);
    return { code: 0, message: 'ok', data: null };
  },

  async providers(): Promise<ApiResponse<any[]>> {
    await delay(MOCK_DELAY);
    return { code: 0, message: 'ok', data: MOCK_CLOUD_PROVIDERS };
  },

  async testConnection(args: any): Promise<ApiResponse<any>> {
    await delay(MOCK_DELAY + 200); // 模拟网络延迟
    const provider = args?.provider || 'openai';
    const modelName = args?.model_name || 'gpt-4o-mini';
    // Mock：约 80% 成功率
    const success = Math.random() > 0.2;
    return {
      code: 0,
      message: 'ok',
      data: {
        success,
        provider,
        model_used: modelName,
        response_snippet: success ? 'OK' : '',
        latency_ms: Math.floor(Math.random() * 400) + 100,
        error: success ? null : 'Mock: 模拟 API Key 无效或网络错误',
      },
    };
  },
};

// ==================== Agent V3 Mock (D1 v3.1 Task 3.1) ====================
// 规范：项目核心设计意图 §三/§八 — 编程 AI 走云端 API，安全检查强制拦截

const MOCK_AGENT_TYPES = [
  { type_name: 'coding', display_name: '编码', description: '从需求生成新代码（函数/类/模块）' },
  { type_name: 'refactor', display_name: '重构', description: '在不改变行为的前提下改进代码结构' },
  { type_name: 'test', display_name: '测试', description: '生成单元测试 / 集成测试 / E2E 测试' },
  { type_name: 'documentation', display_name: '文档', description: '生成/更新 API 文档、README、注释' },
  { type_name: 'debug', display_name: '调试', description: '定位 Bug + 提出最小修复方案' },
  { type_name: 'migration', display_name: '迁移', description: '版本升级 / 框架切换 / API 兼容' },
  { type_name: 'review', display_name: '评审', description: '审查代码质量、安全、性能' },
];

interface MockAgentEntry {
  agent_id: string;
  agent_type: string;
  phase: string;
  created_at: number;
  task_prompt: string;
  result?: any;
}

const mockAgentRegistry: Map<string, MockAgentEntry> = new Map();

export const mockAgentV3 = {
  async types(): Promise<ApiResponse<any[]>> {
    await delay(MOCK_DELAY);
    return { code: 0, message: 'ok', data: MOCK_AGENT_TYPES };
  },

  async create(args: any): Promise<ApiResponse<any>> {
    await delay(MOCK_DELAY);
    const req = args?.request || args || {};
    const agentType = req.agent_type || 'coding';
    const taskPrompt = req.task_prompt || '';
    const now = Math.floor(Date.now() / 1000);
    const agentId = `mock_agent_v3_${agentType}_${now}`;
    const entry: MockAgentEntry = {
      agent_id: agentId,
      agent_type: agentType,
      phase: 'pending',
      created_at: now,
      task_prompt: taskPrompt,
    };
    mockAgentRegistry.set(agentId, entry);
    return {
      code: 0,
      message: 'ok',
      data: {
        agent_id: agentId,
        agent_type: agentType,
        phase: 'pending',
        created_at: now,
      },
    };
  },

  async plan(args: any): Promise<ApiResponse<any>> {
    await delay(MOCK_DELAY + 300); // 模拟云端 API 调用
    const req = args?.request || args || {};
    const agentId = req.agent_id;
    const entry = mockAgentRegistry.get(agentId);
    if (!entry) {
      return { code: 1, message: 'Agent 不存在', data: null };
    }
    entry.phase = 'planned';
    const plan = {
      agent_id: agentId,
      agent_type: entry.agent_type,
      title: `${entry.agent_type} 任务计划`,
      summary: `针对需求 "${entry.task_prompt.slice(0, 80)}${entry.task_prompt.length > 80 ? '...' : ''}" 生成 ${entry.agent_type} 计划`,
      steps: [
        {
          step_id: 0,
          title: '分析需求',
          description: '解析任务目标与上下文',
          target_files: [],
          requires_confirmation: false,
          status: 'completed',
        },
        {
          step_id: 1,
          title: '生成实现方案',
          description: '基于云端 API（Mock）生成代码 diff',
          target_files: ['src/example.ts'],
          requires_confirmation: true,
          status: 'pending',
        },
        {
          step_id: 2,
          title: '验证结果',
          description: '检查生成代码的语法和依赖',
          target_files: [],
          requires_confirmation: false,
          status: 'pending',
        },
      ],
      estimated_steps: 3,
      estimated_duration_sec: 5,
      created_at: Math.floor(Date.now() / 1000),
    };
    return {
      code: 0,
      message: 'ok',
      data: {
        plan,
        safety_check: {
          passed: true,
          blocked_steps: [],
          reasons: [],
          risk_level: 'safe',
        },
      },
    };
  },

  async execute(args: any): Promise<ApiResponse<any>> {
    await delay(MOCK_DELAY + 500); // 模拟沙箱执行
    const req = args?.request || args || {};
    const agentId = req.agent_id;
    const entry = mockAgentRegistry.get(agentId);
    if (!entry) {
      return { code: 1, message: 'Agent 不存在', data: null };
    }
    entry.phase = 'executed';
    const result = {
      agent_id: agentId,
      agent_type: entry.agent_type,
      plan: {
        agent_id: agentId,
        agent_type: entry.agent_type,
        title: `${entry.agent_type} 任务计划`,
        summary: entry.task_prompt,
        steps: [],
        estimated_steps: 1,
        created_at: entry.created_at,
      },
      diffs: [
        {
          path: 'src/example.ts',
          unified_diff: `--- a/src/example.ts
+++ b/src/example.ts
@@ -1,3 +1,5 @@
+// 新增：${entry.agent_type} Agent 生成
+export function generatedExample() {
+  return 'mock result';
+}
`,
          original_content: '',
          modified_content: `// 新增：${entry.agent_type} Agent 生成\nexport function generatedExample() {\n  return 'mock result';\n}\n`,
          added_lines: 4,
          removed_lines: 0,
          is_new_file: true,
        },
      ],
      execution_log: [
        '[mock] 调用 cloud_api_router',
        '[mock] provider=openai, model=gpt-4o-mini',
        '[mock] 生成 diff: src/example.ts',
      ],
      provider: 'openai',
      model_used: 'gpt-4o-mini',
      tokens_used: 152,
      summary: `Mock 模拟：${entry.agent_type} Agent 已生成示例 diff`,
      completed_at: Math.floor(Date.now() / 1000),
    };
    entry.result = result;
    return { code: 0, message: 'ok', data: result };
  },

  async review(args: any): Promise<ApiResponse<any>> {
    await delay(MOCK_DELAY);
    const req = args || {};
    const agentId = req.agent_id;
    const entry = mockAgentRegistry.get(agentId);
    if (!entry || !entry.result) {
      return { code: 1, message: 'Agent 尚未执行，无 diff 可 review', data: null };
    }
    const decision = req.decision || 'approve';
    let appliedFiles: string[] = [];
    let skippedFiles: string[] = [];
    if (decision === 'reject') {
      skippedFiles = entry.result.diffs.map((d: any) => d.path);
    } else if (decision === 'approve') {
      appliedFiles = entry.result.diffs.map((d: any) => d.path);
    } else if (decision === 'approve_partial') {
      const selected = new Set(req.selected_files || []);
      appliedFiles = entry.result.diffs.filter((d: any) => selected.has(d.path)).map((d: any) => d.path);
      skippedFiles = entry.result.diffs.filter((d: any) => !selected.has(d.path)).map((d: any) => d.path);
    }
    entry.phase = decision === 'reject' ? 'rejected' : 'applied';
    return {
      code: 0,
      message: 'ok',
      data: {
        decision,
        applied_files: appliedFiles,
        skipped_files: skippedFiles,
        errors: [],
      },
    };
  },

  async safetyCheck(_args: any): Promise<ApiResponse<any>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: 'ok',
      data: {
        passed: true,
        blocked_steps: [],
        reasons: [],
        risk_level: 'safe',
      },
    };
  },

  async status(args: any): Promise<ApiResponse<any>> {
    await delay(MOCK_DELAY);
    const entry = mockAgentRegistry.get(args?.agent_id);
    if (!entry) {
      return { code: 1, message: 'Agent 不存在', data: null };
    }
    return {
      code: 0,
      message: 'ok',
      data: {
        agent_id: entry.agent_id,
        agent_type: entry.agent_type,
        phase: entry.phase,
        created_at: entry.created_at,
      },
    };
  },

  async list(): Promise<ApiResponse<any[]>> {
    await delay(MOCK_DELAY);
    const list = Array.from(mockAgentRegistry.values()).map(e => ({
      agent_id: e.agent_id,
      agent_type: e.agent_type,
      phase: e.phase,
      created_at: e.created_at,
    }));
    list.sort((a, b) => b.created_at - a.created_at);
    return { code: 0, message: 'ok', data: list };
  },

  async destroy(args: any): Promise<ApiResponse<null>> {
    await delay(MOCK_DELAY);
    mockAgentRegistry.delete(args?.agent_id);
    return { code: 0, message: 'ok', data: null };
  },
};

// ==================== Model Routing Mock (D1 v3.2 Task 3.4.1) ====================
// 规范：项目核心设计意图 §三/§八 — 编程 AI 必须走云端 API 模型
// 强制约束：mock provider 仅返回云端白名单（openai/anthropic/deepseek/...），不出本地底层智能模型

const MOCK_TASK_TYPES = [
  { task_type: 'programming', display_name: '编程（Programming）', description: '代码生成 / 补全 / 重构 / 测试 / 迁移 / 调试', default_provider: 'openai', default_model: 'gpt-4o', default_temperature: 0.2 },
  { task_type: 'analysis', display_name: '分析（Analysis）', description: '代码分析 / 项目索引摘要 / 调用图分析', default_provider: 'deepseek', default_model: 'deepseek-chat', default_temperature: 0.0 },
  { task_type: 'review', display_name: '审查（Review）', description: '代码审查 / 安全检查 / diff 评估', default_provider: 'anthropic', default_model: 'claude-sonnet-4-20250514', default_temperature: 0.0 },
  { task_type: 'documentation', display_name: '文档（Documentation）', description: '文档生成 / 注释 / README / API 文档', default_provider: 'openai', default_model: 'gpt-4o', default_temperature: 0.4 },
  { task_type: 'general', display_name: '通用（General）', description: '未指定任务类型时的兜底路由', default_provider: 'openai', default_model: 'gpt-4o', default_temperature: 0.3 },
];

let mockModelRoutingRules: any[] = [];

export const mockModelRouting = {
  async list(): Promise<ApiResponse<any[]>> {
    await delay(MOCK_DELAY);
    return { code: 0, message: 'ok', data: mockModelRoutingRules };
  },

  async upsert(args: any): Promise<ApiResponse<any>> {
    await delay(MOCK_DELAY);
    const req = args?.request || args || {};
    const taskType = req.task_type;
    if (!taskType) {
      return { code: 1003, message: 'task_type 不能为空' };
    }
    // 强制约束（mock 同步守卫）：仅允许云端 provider
    const CLOUD_PROVIDERS = ['openai', 'anthropic', 'azure', 'deepseek', 'moonshot', 'zhipu', 'qwen', 'custom'];
    if (!CLOUD_PROVIDERS.includes(String(req.provider || '').toLowerCase())) {
      return {
        code: 1003,
        message: 'Yuan Code 模型路由规则仅允许云端 API provider，禁止本地底层智能模型（如 ollama）[项目核心设计意图 §三/§八]',
      };
    }
    const now = Math.floor(Date.now() / 1000);
    const existing = mockModelRoutingRules.find(r => r.task_type === taskType);
    const row = {
      id: existing?.id || Date.now(),
      task_type: taskType,
      provider: req.provider,
      model_name: req.model_name,
      temperature: req.temperature ?? null,
      max_tokens: req.max_tokens ?? null,
      is_enabled: req.is_enabled ?? true,
      is_cloud_only: true,
      priority: 100,
      created_at: existing?.created_at || now,
      updated_at: now,
    };
    if (existing) {
      const idx = mockModelRoutingRules.findIndex(r => r.id === existing.id);
      mockModelRoutingRules[idx] = row;
    } else {
      mockModelRoutingRules.push(row);
    }
    return { code: 0, message: 'ok', data: row };
  },

  async setEnabled(args: any): Promise<ApiResponse<null>> {
    await delay(MOCK_DELAY);
    const rule = mockModelRoutingRules.find(r => r.id === args?.id);
    if (rule) {
      rule.is_enabled = !!args?.is_enabled;
      rule.updated_at = Math.floor(Date.now() / 1000);
    }
    return { code: 0, message: 'ok', data: null };
  },

  async delete(args: any): Promise<ApiResponse<null>> {
    await delay(MOCK_DELAY);
    mockModelRoutingRules = mockModelRoutingRules.filter(r => r.id !== args?.id);
    return { code: 0, message: 'ok', data: null };
  },

  async resolve(args: any): Promise<ApiResponse<any>> {
    await delay(MOCK_DELAY);
    const taskType = args?.task_type;
    const rule = mockModelRoutingRules.find(r => r.task_type === taskType && r.is_enabled);
    if (rule) {
      return {
        code: 0,
        message: 'ok',
        data: {
          task_type: rule.task_type,
          provider: rule.provider,
          model_name: rule.model_name,
          temperature: rule.temperature,
          max_tokens: rule.max_tokens,
          source: 'rule',
        },
      };
    }
    // 回退到默认云端 provider
    const meta = MOCK_TASK_TYPES.find(t => t.task_type === taskType) || MOCK_TASK_TYPES[0];
    return {
      code: 0,
      message: 'ok',
      data: {
        task_type: taskType,
        provider: meta.default_provider,
        model_name: meta.default_model,
        temperature: meta.default_temperature,
        max_tokens: null,
        source: 'default',
      },
    };
  },

  async taskTypes(): Promise<ApiResponse<any[]>> {
    await delay(MOCK_DELAY);
    return { code: 0, message: 'ok', data: MOCK_TASK_TYPES };
  },
};

// ==================== Yuan Code Monitor Mock (D1 v3.2 Task 3.4.2) ====================
// 规范：项目核心设计意图 §二/§三/§8.2 — 非侵入式监测，可关闭

let mockMonitorEnabled = true;

export const mockYuanCodeMonitor = {
  async recordEvent(_args: any): Promise<ApiResponse<null>> {
    await delay(MOCK_DELAY);
    // 非侵入式 + 可关闭：关闭时直接 no-op
    if (!mockMonitorEnabled) {
      return { code: 0, message: 'ok', data: null };
    }
    // mock：仅记录到控制台
    console.log('[Mock] Yuan Code 监测事件:', _args);
    return { code: 0, message: 'ok', data: null };
  },

  async status(): Promise<ApiResponse<any>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: 'ok',
      data: {
        enabled: mockMonitorEnabled,
        recent_event_count: 42,
        recent_cloud_api_calls: 18,
        recent_agent_executions: 6,
      },
    };
  },

  async summary(): Promise<ApiResponse<any>> {
    await delay(MOCK_DELAY);
    return {
      code: 0,
      message: 'ok',
      data: {
        enabled: mockMonitorEnabled,
        total_events: 42,
        by_operation: [
          { operation: 'yuan_code_cloud_api_call', count: 18 },
          { operation: 'yuan_code_agent_execute', count: 6 },
          { operation: 'yuan_code_completion', count: 12 },
          { operation: 'yuan_code_analysis', count: 4 },
          { operation: 'yuan_code_route_resolved', count: 2 },
        ],
        daily_counts: [
          { date: '2026-07-18', count: 5 },
          { date: '2026-07-19', count: 8 },
          { date: '2026-07-20', count: 12 },
          { date: '2026-07-21', count: 9 },
          { date: '2026-07-22', count: 7 },
          { date: '2026-07-23', count: 1 },
          { date: '2026-07-24', count: 0 },
        ],
      },
    };
  },
};

// ==================== Collab Session Mock (D1 v3.2 Task 3.4.3 / 3.4.4) ====================
// 规范：Yjs 接口骨架 + 多人协作（PoC：内存态会话管理）

interface MockCollabSession {
  session_id: string;
  name: string;
  workspace_root: string;
  created_by: number;
  created_at: number;
  participants: any[];
  is_closed: boolean;
}

const mockCollabSessions: Map<string, MockCollabSession> = new Map();

function toSummary(s: MockCollabSession) {
  return {
    session_id: s.session_id,
    name: s.name,
    workspace_root: s.workspace_root,
    created_by: s.created_by,
    created_at: s.created_at,
    participant_count: s.participants.length,
    is_closed: s.is_closed,
  };
}

function genSessionId(): string {
  return 'mock-' + Math.random().toString(36).slice(2, 10) + '-' + Date.now().toString(36);
}

export const mockCollabSession = {
  async create(args: any): Promise<ApiResponse<any>> {
    await delay(MOCK_DELAY);
    const name = args?.name || '未命名协作会话';
    const workspaceRoot = args?.workspace_root || '/workspace';
    const now = Math.floor(Date.now() / 1000);
    const session: MockCollabSession = {
      session_id: genSessionId(),
      name,
      workspace_root: workspaceRoot,
      created_by: 1,
      created_at: now,
      participants: [{
        user_id: 1,
        display_name: 'Mock 用户',
        avatar_url: null,
        joined_at: now,
        last_active_at: now,
        cursor: null,
      }],
      is_closed: false,
    };
    mockCollabSessions.set(session.session_id, session);
    return { code: 0, message: 'ok', data: session };
  },

  async list(): Promise<ApiResponse<any[]>> {
    await delay(MOCK_DELAY);
    const list = Array.from(mockCollabSessions.values())
      .filter(s => !s.is_closed)
      .map(toSummary);
    return { code: 0, message: 'ok', data: list };
  },

  async get(args: any): Promise<ApiResponse<any | null>> {
    await delay(MOCK_DELAY);
    const session = mockCollabSessions.get(args?.session_id);
    if (!session) {
      return { code: 0, message: 'ok', data: null };
    }
    return { code: 0, message: 'ok', data: session };
  },

  async join(args: any): Promise<ApiResponse<any>> {
    await delay(MOCK_DELAY);
    const session = mockCollabSessions.get(args?.session_id);
    if (!session) {
      return { code: 1004, message: '会话不存在' };
    }
    if (session.is_closed) {
      return { code: 1003, message: '会话已关闭，无法加入' };
    }
    const now = Math.floor(Date.now() / 1000);
    // Mock：模拟加入第二个用户
    if (!session.participants.find((p: any) => p.user_id === 2)) {
      session.participants.push({
        user_id: 2,
        display_name: 'Mock 协作者',
        avatar_url: null,
        joined_at: now,
        last_active_at: now,
        cursor: null,
      });
    }
    return { code: 0, message: 'ok', data: session };
  },

  async leave(args: any): Promise<ApiResponse<null>> {
    await delay(MOCK_DELAY);
    const session = mockCollabSessions.get(args?.session_id);
    if (session) {
      session.participants = session.participants.filter((p: any) => p.user_id !== 2);
      if (session.participants.length === 0) {
        session.is_closed = true;
      }
    }
    return { code: 0, message: 'ok', data: null };
  },

  async close(args: any): Promise<ApiResponse<null>> {
    await delay(MOCK_DELAY);
    const session = mockCollabSessions.get(args?.session_id);
    if (session) {
      session.is_closed = true;
    }
    return { code: 0, message: 'ok', data: null };
  },

  async updateCursor(args: any): Promise<ApiResponse<any>> {
    await delay(MOCK_DELAY);
    const session = mockCollabSessions.get(args?.session_id);
    if (!session) {
      return { code: 1004, message: '会话不存在' };
    }
    const cursor = args?.cursor;
    const user = session.participants.find((p: any) => p.user_id === 1);
    if (user) {
      user.cursor = cursor;
      user.last_active_at = Math.floor(Date.now() / 1000);
    }
    return { code: 0, message: 'ok', data: session };
  },
};

// ==================== 处理器注册 ====================

export const ipcMockHandlers: Record<string, any> = {
  auth: mockAuth,
  home: mockHome,
  ai: mockAI,
  knowledge: mockKnowledge,
  recycle: mockRecycleBin,
  terminal: mockTerminal,
  profile: mockProfile,
  system: mockSystem,
  extension: mockExtension,
  agent: mockAgent,
  sandbox: mockSandbox,
  mcp: mockMcp,
  skill: mockSkill,
  compact: mockCompact,
  goal: mockGoal,
  sync: mockSync,
  // D1 v3.1 Task 3.5 / 3.1
  cloudApi: mockCloudApi,
  agentV3: mockAgentV3,
  // D1 v3.2 Task 3.4.1 / 3.4.2 / 3.4.3 / 3.4.4: 多模型与协作
  modelRouting: mockModelRouting,
  yuanCodeMonitor: mockYuanCodeMonitor,
  collabSession: mockCollabSession
};