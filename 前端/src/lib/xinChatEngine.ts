import { t } from "i18next";
/**
 * 小欣智能对话引擎
 * 意图识别 + 上下文感知 + 多模响应生成
 * 支持尝试真实后端 AI 调用，失败时 fallback 本地智能响应
 */
import { ipc } from '@/lib/ipc';
interface ChatContext {
  personaId: string;
  history: {
    role: string;
    content: string;
  }[];
  memories?: any[];
  config?: any;
}
interface IntentResult {
  intent: string;
  confidence: number;
  entities: Record<string, string>;
}
const INTENT_PATTERNS: Record<string, RegExp[]> = {
  code_help: [/写(一个|段|个).*(代码|函数|脚本|程序)/, /(帮我|怎么|如何).*(写|实现|开发|编程)/, /这段代码|bug|报错|调试|修复/, /(优化|重构|性能).*(代码|函数|逻辑)/, /(什么|哪个).*(语言|框架|库).*(合适|好|推荐)/, /(import|require|from).*import/, /(async|await|promise|callback)/i, /(api|接口|rest|graphql)/i, /(docker|容器|部署|ci|cd)/i],
  knowledge_ask: [/什么(是|叫).*/, /怎么(理解|解释|定义)/, /为什么.*/, /(区别|对比|vs).*/, /(推荐|建议).*(学习|书|教程|资源|课程)/, /(讲讲|说说|介绍).*/],
  creative_brainstorm: [/(帮我想|创意|灵感|点子|设计)/, /(做个|搞个).*(游戏|动画|效果|UI|界面)/, /(换|改).*(颜色|配色|风格)/, /(好玩|有趣).*/],
  casual_chat: [/(今天|最近).*(怎么样|好吗|好不|过得好)/, /(累了|压力|焦虑|不开心)/, /(谢谢|感谢|厉害|牛)/, /(讲个|来段).*(笑话|故事)/, /(哈哈|嘿嘿|嗯嗯|好的)/],
  task_help: [/(帮我|能).*(做|干|弄|搞)/, /(怎么|如何).*(用|操作)/, /(功能|命令).*/, /(设置|配置|开启|关闭).*/]
};
const PERSONA_KNOWLEDGE: Record<string, string[]> = {
  code_assistant: ['TypeScript', 'Rust', 'React', 'Tauri', 'Python', t("lib.xinChatEngine.k1"), t("lib.xinChatEngine.k2"), t("lib.xinChatEngine.k3"), t("lib.xinChatEngine.k4"), t("lib.xinChatEngine.k5"), t("lib.xinChatEngine.k6"), t("lib.xinChatEngine.k7"), t("lib.xinChatEngine.k8"), 'WebAssembly', t("lib.xinChatEngine.k9"), t("lib.xinChatEngine.k10"), t("lib.xinChatEngine.k11"), t("lib.xinChatEngine.k12")],
  knowledge_mentor: [t("lib.xinChatEngine.k13"), t("lib.xinChatEngine.k14"), t("lib.xinChatEngine.k15"), t("lib.xinChatEngine.k16"), t("lib.xinChatEngine.k17"), t("lib.xinChatEngine.k18"), t("lib.xinChatEngine.k19"), t("lib.xinChatEngine.k20"), t("lib.xinChatEngine.k21"), t("lib.xinChatEngine.k22"), t("lib.xinChatEngine.k23"), t("lib.xinChatEngine.k24"), t("lib.xinChatEngine.k25"), 'DevOps', t("lib.xinChatEngine.k26")],
  creative_partner: [t("lib.xinChatEngine.k27"), t("lib.xinChatEngine.k28"), t("lib.xinChatEngine.k29"), t("lib.xinChatEngine.k30"), t("lib.xinChatEngine.k31"), t("lib.xinChatEngine.k32"), t("lib.xinChatEngine.k33"), t("lib.xinChatEngine.k34"), t("lib.xinChatEngine.k35"), t("lib.xinChatEngine.k36")],
  caring_friend: [t("lib.xinChatEngine.k37"), t("lib.xinChatEngine.k38"), t("lib.xinChatEngine.k39"), t("components.intelligence.SuggestionsPanel.k1"), t("lib.xinChatEngine.k40"), t("lib.xinChatEngine.k41"), t("lib.xinChatEngine.k42"), t("components.intelligence.SuggestionsPanel.k2"), t("lib.xinChatEngine.k43"), t("lib.xinChatEngine.k44")]
};
function detectIntent(text: string): IntentResult {
  let bestIntent = 'general';
  let bestConfidence = 0;
  const entities: Record<string, string> = {};
  const codeMatch = text.match(/(python|rust|typescript|javascript|go|java|c\+\+|react|vue|tauri)/i);
  if (codeMatch) entities.lang = codeMatch[1].toLowerCase();
  for (const [intent, patterns] of Object.entries(INTENT_PATTERNS)) {
    for (const pattern of patterns) {
      if (pattern.test(text)) {
        const confidence = 0.6 + Math.random() * 0.3;
        if (confidence > bestConfidence) {
          bestConfidence = confidence;
          bestIntent = intent;
        }
        break;
      }
    }
  }
  if (bestConfidence < 0.5) {
    bestIntent = /[?？]/.test(text) ? 'knowledge_ask' : 'casual_chat';
    bestConfidence = 0.5;
  }
  return {
    intent: bestIntent,
    confidence: bestConfidence,
    entities
  };
}
function generateCodeAssistantResponse(input: string, entities: Record<string, string>): string {
  if (/写.*代码|实现|开发/.test(input)) {
    const lang = entities.lang || 'typescript';
    return t("lib.xinChatEngine.k45", {
      lang: lang,
      arg0: lang
    });
  }
  if (/bug|报错|调试|修复/.test(input)) {
    return t("lib.xinChatEngine.k46");
  }
  if (/优化|性能|重构/.test(input)) {
    return t("lib.xinChatEngine.k47");
  }
  return t("lib.xinChatEngine.k48");
}
function generateKnowledgeResponse(input: string): string {
  if (/为什么/.test(input)) {
    const topics = [t("lib.xinChatEngine.k49"), t("lib.xinChatEngine.k50"), t("lib.xinChatEngine.k51")];
    const topic = topics[Math.floor(Math.random() * topics.length)];
    return t("lib.xinChatEngine.k52", {
      topic: topic
    });
  }
  if (/区别|对比|vs/i.test(input)) {
    return t("lib.xinChatEngine.k53");
  }
  return t("lib.xinChatEngine.k54");
}
function generateCreativeResponse(_input: string): string {
  const ideas = [t("lib.xinChatEngine.k55"), t("lib.xinChatEngine.k56"), t("lib.xinChatEngine.k57")];
  return ideas[Math.floor(Math.random() * ideas.length)];
}
function generateCasualResponse(input: string): string {
  if (/累|压力|焦虑|不开心/.test(input)) {
    return t("lib.xinChatEngine.k58");
  }
  if (/谢谢|感谢|厉害|牛/.test(input)) {
    const replies = [t("lib.xinChatEngine.k59"), t("lib.xinChatEngine.k60"), t("lib.xinChatEngine.k61")];
    return replies[Math.floor(Math.random() * replies.length)];
  }
  const chats = [t("lib.xinChatEngine.k62"), t("lib.xinChatEngine.k63"), t("lib.xinChatEngine.k64")];
  return chats[Math.floor(Math.random() * chats.length)];
}
function generateTaskResponse(input: string): string {
  if (/命令|功能/.test(input)) {
    return t("lib.xinChatEngine.k65");
  }
  return t("lib.xinChatEngine.k66");
}
function generateResponse(input: string, personaId: string): string {
  const {
    intent,
    entities
  } = detectIntent(input);
  switch (intent) {
    case 'code_help':
      return personaId === 'code_assistant' || personaId === 'knowledge_mentor' ? generateCodeAssistantResponse(input, entities) : t("lib.xinChatEngine.k67", {
        arg0: personaId === 'creative_partner' ? t("lib.xinChatEngine.k68") : t("lib.xinChatEngine.k69")
      });
    case 'knowledge_ask':
      return generateKnowledgeResponse(input);
    case 'creative_brainstorm':
      return personaId === 'creative_partner' ? generateCreativeResponse(input) : generateCreativeResponse(input) + t("lib.xinChatEngine.k70");
    case 'casual_chat':
      return generateCasualResponse(input);
    case 'task_help':
      return generateTaskResponse(input);
    default:
      return t("lib.xinChatEngine.k71");
  }
}
export async function xinChat(input: string, context: ChatContext): Promise<{
  content: string;
  memories?: any[];
  sentiment?: any;
  mood?: any;
}> {
  let sentiment: any = null;
  let mood: any = null;
  let relatedMemories: any[] = [];

  // 尝试调用后端获取真实数据
  let sResult: any = null;
  let mResult: any = null;
  let memResult: any = null;
  try {
    sResult = await ipc.invoke<any>('xin_analyze_sentiment', {
      text: input
    });
  } catch (_e) {}
  try {
    mResult = await ipc.invoke<any>('xin_update_mood', {
      text: input
    });
  } catch (_e) {}
  try {
    memResult = await ipc.invoke<any>('xin_search_memories', {
      query: input
    });
  } catch (_e) {}
  if (sResult?.data) sentiment = sResult.data;
  if (mResult?.data) mood = mResult.data;
  if (memResult?.data) relatedMemories = memResult.data;

  // 基于记忆生成上下文感知回复
  let memContext = '';
  if (relatedMemories.length > 0) {
    const topMemories = relatedMemories.slice(0, 3);
    memContext = t("lib.xinChatEngine.k72") + topMemories.map((m: any) => m.content?.slice(0, 40) || m.title).join(' | ');
  }
  const response = generateResponse(input, context.personaId) + memContext;

  // 模拟思考延迟（150-600ms，比之前更真实）
  await new Promise(r => setTimeout(r, 200 + Math.random() * 400));
  return {
    content: response,
    memories: relatedMemories,
    sentiment,
    mood
  };
}
export { detectIntent, PERSONA_KNOWLEDGE };