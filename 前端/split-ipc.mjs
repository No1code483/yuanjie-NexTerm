// Script to split ipc.ts into ipc/ subdirectory modules
import { readFileSync, writeFileSync, mkdirSync, existsSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const ipcPath = join(__dirname, 'src', 'lib', 'ipc.ts');
const ipcDir = join(__dirname, 'src', 'lib', 'ipc');

const content = readFileSync(ipcPath, 'utf-8');
const lines = content.split('\n');

// Find line indices (0-based) of each export const
const moduleStarts = [];
for (let i = 0; i < lines.length; i++) {
  const match = lines[i].match(/^export const (\w+) = \{/);
  if (match) {
    moduleStarts.push({ name: match[1], line: i });
  }
}

console.log('Found modules:', moduleStarts.map(m => `${m.name}@${m.line+1}`).join(', '));

// Core = everything before the first "export const" (line 743, index 742)
// But we want to include the comment block "便捷方法导出" which starts a few lines before
const firstModuleLine = moduleStarts[0].line; // 0-based index of first export const
// Find the comment block before it (go back to find "// ====" or blank line)
let coreEnd = firstModuleLine;
while (coreEnd > 0 && !lines[coreEnd - 1].match(/^\/\//) && lines[coreEnd - 1].trim() !== '') {
  coreEnd--;
}
// Go back to include the comment block
while (coreEnd > 0 && (lines[coreEnd - 1].match(/^\/\//) || lines[coreEnd - 1].match(/^\/\*/))) {
  coreEnd--;
}

// Write core.ts (lines 0 to coreEnd-1)
let coreContent = lines.slice(0, coreEnd).join('\n');
// Fix relative imports
coreContent = coreContent.replace("from './errorCodeI18n'", "from '../errorCodeI18n'");
coreContent = coreContent.replace("from './ipcMock'", "from '../ipcMock'");
// Remove type imports that are only used in modules, not in core (IPCache/IPCService don't use them)
coreContent = coreContent.replace(/^import type \{.*\} from '@\/types';$/gm, '');
coreContent = coreContent.replace(/^import type \{.*\} from '@\/types\/game';$/gm, '');
// Clean up any blank lines left by removed imports
coreContent = coreContent.replace(/\n{3,}/g, '\n\n');
// Add header
coreContent = `// ipc/core.ts — IPC 核心基础设施（从 ipc.ts 拆分，T2.1.6）\n// 包含：IPCache / IPCService / ipc 单例 / ApiResponse 类型 / USE_MOCK\n\n${coreContent}\n`;
writeFileSync(join(ipcDir, 'core.ts'), coreContent, 'utf-8');
console.log(`Written core.ts (${coreEnd} lines)`);

// Module groupings: which modules go into which file
const groupings = {
  'auth': 'auth',
  'home': 'home',
  'ai': 'ai',
  'knowledge': 'knowledge',
  'intelligence': 'intelligence',
  'recycleBin': 'system',
  'terminal': 'terminal',
  'profile': 'profile',
  'newsSource': 'system',
  'system': 'system',
  'extension': 'system',
  'linux': 'terminal',
  'yuanCode': 'yuan-code',
  'yuanCompact': 'yuan-code',
  'yuanGoal': 'yuan-code',
  'xin': 'xin',
  'realtime': 'xin',
  'game': 'game',
};

// For each module, find its content (from its preceding type definitions to its closing "};" line)
const moduleContents = {};
for (let i = 0; i < moduleStarts.length; i++) {
  const start = moduleStarts[i].line;

  // Find this module's closing "};" line (line matching /^};$/ with no indentation)
  // This prevents including type definitions that belong to the NEXT module
  let moduleClose = start;
  while (moduleClose < lines.length && !lines[moduleClose].match(/^};$/)) {
    moduleClose++;
  }
  const end = moduleClose + 1; // Include the closing "};" line

  // Set actualStart to include all type definitions between the previous module's
  // closing "};" and this module's start. This ensures type definitions defined
  // between modules (e.g., YuanFileNode before yuanCode, XinAttachment before xin)
  // are correctly associated with the FOLLOWING module, not the preceding one.
  let actualStart;
  if (i > 0) {
    const prevStart = moduleStarts[i - 1].line;
    let prevModuleEnd = prevStart;
    while (prevModuleEnd < start && !lines[prevModuleEnd].match(/^};$/)) {
      prevModuleEnd++;
    }
    actualStart = prevModuleEnd + 1; // Line after previous module's closing "};""
  } else {
    // First module: start from coreEnd (end of core content)
    actualStart = coreEnd;
  }

  const name = moduleStarts[i].name;
  const group = groupings[name] || name;
  const moduleContent = lines.slice(actualStart, end).join('\n');

  if (!moduleContents[group]) {
    moduleContents[group] = [];
  }
  moduleContents[group].push(moduleContent);
}

// Available types from @/types and @/types/game
// Used to detect which types each module file actually needs (avoids unused import TS errors)
const typesFromTypes = ['UserInfo', 'BackendPermission', 'DashboardData', 'RealtimeStats', 'Suggestion', 'BehaviorReport', 'ActivityLog', 'ActivityStats'];
const typesFromGame = ['GameBuilding', 'GameBuildHistory', 'GameBreakthroughRecord', 'GameBuildingStatus', 'GameKbCategoryMapping', 'GameKnowledgeDomain', 'GameKnowledgeDomainId', 'GameKnowledgeProgress', 'GamePointsSourceType', 'WorldState', 'WorldSummary', 'RealmInfo', 'StartBuildingRequest', 'BreakthroughSession', 'BreakthroughAnswer', 'BreakthroughOutcome', 'BreakthroughPreviewInfo', 'BuildingCatalog', 'SyncResult', 'EventsAndTasks', 'TimelineEvent', 'RebuiltScene', 'GameNpc', 'GameNpcConversation', 'NpcChatResponse', 'GameNpcMemory', 'GameNpcRelationship', 'GameNpcRumor', 'PlayerSkill', 'Story', 'StorySummary', 'GenerateStoryRequest', 'AdvanceStoryRequest'];

// Detect which types are used in the content and generate precise import statements
function detectTypeImports(content) {
  const usedFromTypes = typesFromTypes.filter(t => new RegExp(`\\b${t}\\b`).test(content));
  const usedFromGame = typesFromGame.filter(t => new RegExp(`\\b${t}\\b`).test(content));
  const imports = [];
  if (usedFromTypes.length > 0) {
    imports.push(`import type { ${usedFromTypes.join(', ')} } from '@/types';`);
  }
  if (usedFromGame.length > 0) {
    imports.push(`import type { ${usedFromGame.join(', ')} } from '@/types/game';`);
  }
  return imports.length > 0 ? imports.join('\n') : '';
}

// Write each group file
for (const [group, contents] of Object.entries(moduleContents)) {
  const fileName = `${group}.ts`;
  let fileContent = `// ipc/${fileName} — ${group} 模块 IPC 封装（从 ipc.ts 拆分，T2.1.6）\n`;
  fileContent += `import { ipc } from './core';\n`;
  const typeImports = detectTypeImports(contents.join('\n'));
  if (typeImports) {
    fileContent += typeImports + '\n';
  }
  fileContent += '\n';
  fileContent += contents.join('\n\n');
  fileContent += '\n';

  writeFileSync(join(ipcDir, fileName), fileContent, 'utf-8');
  console.log(`Written ${fileName} (${contents.length} modules)`);
}

// Now create the re-export hub (ipc.ts)
const reExportContent = `// ipc.ts — IPC 聚合入口（T2.1.6 模块化拆分）
// 核心基础设施在 ./ipc/core.ts，各模块在 ./ipc/ 子目录下
// 此文件作为聚合入口 re-export，保持现有 import { auth, ai, ... } from '@/lib/ipc' 不变

// Core
export { ipc, USE_MOCK } from './ipc/core';
export type { ApiResponse } from './ipc/core';
export { default } from './ipc/core';

// Modules
export { auth } from './ipc/auth';
export { home } from './ipc/home';
export { ai } from './ipc/ai';
export { knowledge } from './ipc/knowledge';
export { intelligence } from './ipc/intelligence';
export { system, newsSource, extension, recycleBin } from './ipc/system';
export { terminal, linux } from './ipc/terminal';
export { profile } from './ipc/profile';
export { yuanCode, yuanCompact, yuanGoal } from './ipc/yuan-code';
export type { YuanFileNode, YuanExecutionResult, HighlightLine, HighlightResult, CodeCompletionRequest, CompletionItem, ReplaceRange, CodeCompletionResult, CodeAnalysisRequest, CodeIssue, CodeAnalysisResult, CodeSnippet, SaveSnippetRequest, UpdateSnippetRequest, DiffLine, DiffHunk, DiffResult, ReplaceFilesRequest, CopyMoveParams, FileInfoResult, WorkspaceTab, WorkspaceSession, SaveWorkspaceRequest, PromptVariable, PromptTemplateMeta, RenderPromptRequest, RenderPromptResult, AgentsMdFile, HierarchicalInstructions, AssembleSystemPromptRequest, PromptPart, AssembleSystemPromptResult } from './ipc/yuan-code';
export { xin, realtime } from './ipc/xin';
export type { XinAttachment, XinParsedAttachment, XinOutputEnhancement, XinDialogueSendOptions, RealtimeConfig, RealtimeState } from './ipc/xin';
export { game } from './ipc/game';
`;

writeFileSync(ipcPath, reExportContent, 'utf-8');
console.log('Written ipc.ts (re-export hub)');
console.log('Done!');

// Need to also export default from core
// Check if core.ts has "export default ipc"
const coreHasDefault = coreContent.includes('export default ipc');
if (!coreHasDefault) {
  // Append to core.ts
  const corePath = join(ipcDir, 'core.ts');
  let coreText = readFileSync(corePath, 'utf-8');
  coreText += '\nexport default ipc;\n';
  writeFileSync(corePath, coreText, 'utf-8');
  console.log('Added default export to core.ts');
}
