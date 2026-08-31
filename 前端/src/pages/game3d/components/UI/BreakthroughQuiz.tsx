import { t } from "i18next";
/**
 * BreakthroughQuiz - 突破考验全屏答题页（T2.3，12_AI考验机制.md §4）
 *
 * 触发条件：`breakthroughSession` 不为 null（由 BreakthroughAltar 点击拉取后写入 store）。
 *
 * UI 状态机：
 *   ┌─ outcome === null → 答题界面（单题切换）
 *   └─ outcome !== null → 结果摘要卡（result/score/daoji/realm/ai_review）
 *
 * 答题流程：
 *   1. 单题展示（题号 + 题型徽章 + 知识点 + 难度系数）
 *   2. choice → 4 选项 radio；short_answer → 4 行 textarea；application → 8 行 textarea
 *   3. 上一题 / 下一题切换（允许回看修改）
 *   4. 最后一题 + 全部已答 → 启用"提交考验"按钮
 *   5. 提交 → 调用 `game.submitBreakthrough(session, answers)` → `setBreakthroughOutcome(outcome)`
 *   6. 结果动画由 T2.4 BreakthroughResultAnimations 在 Canvas 内独立渲染
 *   7. 用户点"返回" → `clearBreakthroughState()` 退出（保留 preview 用于祭坛状态判断）
 *
 * 风格：终端黑客（黑底 + 荧光绿 #00FF00 + 警示红 #FF0000），等宽字体。
 *
 * change-id: game-3d-rebuild-refactor
 */
import { useEffect, useMemo, useRef, useState } from 'react';
import { game } from '@/lib/ipc';
import type { BreakthroughAnswer, BreakthroughOutcome, BreakthroughQuestion, BreakthroughSession, GameBreakthroughQuestionType, GameDaoFoundation, GameKnowledgeDomainId, GameRealmMajor, GameRealmMinor, PlayerSkill, PlayerSkillLabel, ReviewSuggestion } from '@/types/game';
import { useGame3DStore } from '../../stores/gameStore';

// ===== 本地显示名映射（YAGNI：仅 BreakthroughQuiz 使用，未来其他组件复用再抽离到 utils） =====

const REALM_MAJOR_LABELS: Record<GameRealmMajor, string> = {
  mortal: t("game.components.RealmCard.k1"),
  qi_refining: t("game.components.RealmCard.k2"),
  foundation_building: t("game.components.RealmCard.k3"),
  golden_core: t("game.components.RealmCard.k4"),
  nascent_soul: t("game.components.RealmCard.k5"),
  spirit_transformation: t("game.components.RealmCard.k6"),
  unity: t("game.components.RealmCard.k7"),
  mahayana: t("game.components.RealmCard.k8"),
  tribulation: t("game.components.RealmCard.k9"),
  immortal: t("game.components.RealmCard.k10")
};
const REALM_MINOR_LABELS: Record<GameRealmMinor, string> = {
  early: t("game.components.RealmCard.k11"),
  middle: t("game.components.RealmCard.k12"),
  complete: t("game.components.RealmCard.k13")
};
const DAO_FOUNDATION_LABELS: Record<GameDaoFoundation, string> = {
  white: t("game3d.components.UI.BreakthroughQuiz.k1"),
  blue: t("game3d.components.UI.BreakthroughQuiz.k2"),
  red: t("game3d.components.UI.BreakthroughQuiz.k3"),
  purple: t("game3d.components.UI.BreakthroughQuiz.k4"),
  black: t("game3d.components.UI.BreakthroughQuiz.k5")
};
const QUESTION_TYPE_LABELS: Record<GameBreakthroughQuestionType, string> = {
  choice: t("game3d.components.UI.BreakthroughQuiz.k6"),
  short_answer: t("game3d.components.UI.BreakthroughQuiz.k7"),
  application: t("game3d.components.UI.BreakthroughQuiz.k8"),
  image_choice: t("game3d.components.UI.BreakthroughQuiz.k47")
};
const DOMAIN_LABELS: Record<GameKnowledgeDomainId, string> = {
  cs: t("game3d.components.UI.BreakthroughQuiz.k9"),
  math: t("game3d.components.UI.BreakthroughQuiz.k10"),
  physics: t("game3d.components.UI.BreakthroughQuiz.k11"),
  literature: t("game3d.components.UI.BreakthroughQuiz.k12"),
  history: t("game3d.components.UI.BreakthroughQuiz.k13"),
  art: t("game3d.components.UI.BreakthroughQuiz.k14"),
  engineering: t("game3d.components.UI.BreakthroughQuiz.k15"),
  medicine: t("game3d.components.UI.BreakthroughQuiz.k16"),
  philosophy: t("game3d.components.UI.BreakthroughQuiz.k17"),
  economics: t("game3d.components.UI.BreakthroughQuiz.k18"),
  language: t("game3d.components.UI.BreakthroughQuiz.k19"),
  other: t("game3d.components.UI.BreakthroughQuiz.k20")
};

// ===== D4.3 自适应难度：技能等级映射（对齐后端 game_difficulty_service.rs SkillLabel 边界） =====

const SKILL_LABEL_KEYS: Record<PlayerSkillLabel, string> = {
  novice: t("game3d.components.UI.BreakthroughQuiz.k51"),
  beginner: t("game3d.components.UI.BreakthroughQuiz.k52"),
  proficient: t("game3d.components.UI.BreakthroughQuiz.k53"),
  expert: t("game3d.components.UI.BreakthroughQuiz.k54"),
  master: t("game3d.components.UI.BreakthroughQuiz.k55"),
};

/** 由 skill_score 派生 5 级评级（对齐后端 game_difficulty_service.rs skill_label：0-20/21-40/41-60/61-80/81-100）。 */
function skillLabelFromScore(score: number): PlayerSkillLabel {
  if (score <= 20) return 'novice';
  if (score <= 40) return 'beginner';
  if (score <= 60) return 'proficient';
  if (score <= 80) return 'expert';
  return 'master';
}

// ===== D4.6 深化#7/#8/#9 常量 =====

/** 草稿过期时间：30 分钟（ms）。超过视为放弃，不恢复。 */
const DRAFT_TTL_MS = 30 * 60 * 1000;
/** 每题答题时长（秒）。倒计时总时长 = 题数 × 此值。 */
const SECONDS_PER_QUESTION = 180;
/** 倒计时进入红色闪烁的剩余阈值（秒）。 */
const COUNTDOWN_RED_THRESHOLD = 60;

/** 草稿在 localStorage 的 key 前缀。 */
const draftKey = (sessionId: string) => `bk_draft_${sessionId}`;

/** 草稿结构。 */
interface BreakthroughDraft {
  /** 已答内容（question_id → answer）。 */
  answers: Record<string, string>;
  /** 当前题目索引。 */
  current_index: number;
  /** 草稿保存时间戳（ms）。 */
  saved_at: number;
}

/** 读取草稿；不存在或已过期返回 null。 */
function loadDraft(sessionId: string): BreakthroughDraft | null {
  try {
    const raw = localStorage.getItem(draftKey(sessionId));
    if (!raw) return null;
    const draft = JSON.parse(raw) as BreakthroughDraft;
    if (Date.now() - draft.saved_at > DRAFT_TTL_MS) {
      // 过期，清理
      localStorage.removeItem(draftKey(sessionId));
      return null;
    }
    return draft;
  } catch {
    return null;
  }
}

/** 写入草稿。 */
function saveDraft(sessionId: string, answers: Record<string, string>, currentIndex: number) {
  try {
    const draft: BreakthroughDraft = {
      answers,
      current_index: currentIndex,
      saved_at: Date.now(),
    };
    localStorage.setItem(draftKey(sessionId), JSON.stringify(draft));
  } catch {
    // localStorage 不可用（隐私模式/满额）→ 忽略
  }
}

/** 清除草稿。 */
function clearDraft(sessionId: string) {
  try {
    localStorage.removeItem(draftKey(sessionId));
  } catch {
    // ignore
  }
}

/** 格式化倒计时为 MM:SS。 */
function formatCountdown(seconds: number): string {
  const s = Math.max(0, Math.floor(seconds));
  const m = Math.floor(s / 60);
  const r = s % 60;
  return `${String(m).padStart(2, '0')}:${String(r).padStart(2, '0')}`;
}

/** D4.6 深化#9：批改等待动画文案轮播池（修仙语境）。 */
const GRADING_PHRASES: string[] = [
  '天道审视答卷中...',
  '考官翻阅卷宗...',
  '道基光影流转...',
  '天雷隐隐酝酿...',
  '因果功过衡量...',
  '仙籍查阅比对...',
];

// ============================================================================
// 主组件
// ============================================================================

export function BreakthroughQuiz() {
  const session = useGame3DStore(s => s.breakthroughSession);
  const answersMap = useGame3DStore(s => s.breakthroughAnswers);
  const submitting = useGame3DStore(s => s.breakthroughSubmitting);
  const outcome = useGame3DStore(s => s.breakthroughOutcome);
  const setAnswer = useGame3DStore(s => s.setBreakthroughAnswer);
  const setSubmitting = useGame3DStore(s => s.setBreakthroughSubmitting);
  const setOutcome = useGame3DStore(s => s.setBreakthroughOutcome);
  const clearState = useGame3DStore(s => s.clearBreakthroughState);
  const [currentIndex, setCurrentIndex] = useState(0);
  // D4.3 自适应难度：玩家技能评分（答题界面顶部展示用，提交后下次进入会反映最新值）
  const [playerSkill, setPlayerSkill] = useState<PlayerSkill | null>(null);
  const worldId = session?.world_id;

  // D4.6 深化#8：倒计时（总时长 = 题数 × 180s）。session 变化时重置。
  const totalSeconds = session ? session.questions.length * SECONDS_PER_QUESTION : 0;
  const [remainingSeconds, setRemainingSeconds] = useState(totalSeconds);
  // 防止 useEffect 闭包捕获过期 session — 用 ref 同步
  const sessionRef = useRef(session);
  sessionRef.current = session;
  // 自动提交标志（避免倒计时归零与用户手动提交重复触发）
  const autoSubmitTriggeredRef = useRef(false);

  // D4.6 深化#9：批改等待动画进度（0-100）+ 当前文案索引
  const [gradingProgress, setGradingProgress] = useState(0);
  const [gradingPhraseIdx, setGradingPhraseIdx] = useState(0);

  // D4.6 深化#7：草稿恢复提示（仅初始化时检查一次）
  const [draftRestorePrompt, setDraftRestorePrompt] = useState<BreakthroughDraft | null>(null);

  // 切换题目时滚动到顶部
  useEffect(() => {
    const el = document.querySelector('.bk-quiz__body');
    if (el) el.scrollTop = 0;
  }, [currentIndex]);

  // D4.3 拉取玩家技能评分（worldId 变化时重新拉取；失败仅告警，不阻塞答题）
  useEffect(() => {
    if (!worldId) return;
    let cancelled = false;
    game.playerSkill(worldId)
      .then(res => {
        if (!cancelled && res.code === 0 && res.data) setPlayerSkill(res.data);
      })
      .catch(err => console.warn('[BreakthroughQuiz] D4.3 playerSkill fetch failed:', err));
    return () => { cancelled = true; };
  }, [worldId]);

  // D4.6 深化#7：会话建立时检查草稿（仅一次）
  useEffect(() => {
    if (!session) return;
    const draft = loadDraft(session.session_id);
    if (draft && Object.keys(draft.answers).length > 0) {
      setDraftRestorePrompt(draft);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [session?.session_id]);

  // D4.6 深化#7：答案变化时自动暂存草稿
  useEffect(() => {
    if (!session || outcome) return;
    saveDraft(session.session_id, answersMap, currentIndex);
  }, [session, outcome, answersMap, currentIndex]);

  // D4.6 深化#8：倒计时每秒递减，归零自动提交
  useEffect(() => {
    if (!session || outcome) return;
    const timer = window.setInterval(() => {
      setRemainingSeconds(prev => {
        if (prev <= 1) {
          // 归零：自动提交（仅触发一次）
          if (!autoSubmitTriggeredRef.current) {
            autoSubmitTriggeredRef.current = true;
            // 异步触发，避免 setState in render
            window.setTimeout(() => {
              void autoSubmit();
            }, 0);
          }
          return 0;
        }
        return prev - 1;
      });
    }, 1000);
    return () => window.clearInterval(timer);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [session?.session_id, outcome]);

  // D4.6 深化#9：批改等待动画进度推进 + 文案轮播（仅在 submitting 时运行）
  useEffect(() => {
    if (!submitting) {
      setGradingProgress(0);
      setGradingPhraseIdx(0);
      return;
    }
    setGradingProgress(0);
    setGradingPhraseIdx(0);
    const progressTimer = window.setInterval(() => {
      setGradingProgress(p => {
        // 模拟进度：前 80% 在 2.5s 内推进，后 20% 缓慢推进等待真实响应
        if (p < 80) return Math.min(80, p + Math.random() * 8 + 2);
        if (p < 95) return Math.min(95, p + Math.random() * 1.5);
        return p;
      });
    }, 200);
    const phraseTimer = window.setInterval(() => {
      setGradingPhraseIdx(i => (i + 1) % GRADING_PHRASES.length);
    }, 1500);
    return () => {
      window.clearInterval(progressTimer);
      window.clearInterval(phraseTimer);
    };
  }, [submitting]);

  // 会话不存在时不渲染（由 Game3D 控制条件挂载）
  if (!session) return null;

  // 结果态：显示结果摘要卡
  if (outcome) {
    // D4.6 深化#7：提交完成后清除草稿
    clearDraft(session.session_id);
    return <BreakthroughResultCard session={session} outcome={outcome} onClose={clearState} />;
  }
  const total = session.questions.length;
  const current = session.questions[currentIndex];
  const currentAnswer = answersMap[current.question_id] ?? '';
  const answeredCount = session.questions.filter(q => (answersMap[q.question_id] ?? '').trim().length > 0).length;
  const allAnswered = answeredCount === total;
  const isLast = currentIndex === total - 1;
  const isFirst = currentIndex === 0;
  const isCountdownRed = remainingSeconds <= COUNTDOWN_RED_THRESHOLD && remainingSeconds > 0;

  /** 提交考验 */
  const handleSubmit = async () => {
    if (!allAnswered || submitting) return;
    setSubmitting(true);
    try {
      const payload: BreakthroughAnswer[] = session.questions.map(q => ({
        question_id: q.question_id,
        user_answer: answersMap[q.question_id] ?? ''
      }));
      const res = await game.submitBreakthrough(session, payload);
      if (res.code !== 0 || !res.data) {
        console.error('[BreakthroughQuiz] submitBreakthrough failed:', res.message);
        // 失败时弹回当前页（保留答案），不重置状态
        return;
      }
      setOutcome(res.data);
    } catch (err) {
      console.error('[BreakthroughQuiz] submitBreakthrough error:', err);
    } finally {
      setSubmitting(false);
    }
  };

  /** D4.6 深化#8：倒计时归零自动提交（不要求全部已答，未答题按空串提交）。 */
  const autoSubmit = async () => {
    const s = sessionRef.current;
    if (!s || submitting) return;
    console.warn('[BreakthroughQuiz] 倒计时归零，自动提交');
    setSubmitting(true);
    try {
      const payload: BreakthroughAnswer[] = s.questions.map(q => ({
        question_id: q.question_id,
        user_answer: answersMap[q.question_id] ?? ''
      }));
      const res = await game.submitBreakthrough(s, payload);
      if (res.code === 0 && res.data) {
        setOutcome(res.data);
      }
    } catch (err) {
      console.error('[BreakthroughQuiz] autoSubmit error:', err);
    } finally {
      setSubmitting(false);
    }
  };

  /** D4.6 深化#7：恢复草稿。 */
  const handleRestoreDraft = () => {
    if (!draftRestorePrompt || !session) return;
    // 逐题恢复答案
    for (const [qid, ans] of Object.entries(draftRestorePrompt.answers)) {
      setAnswer(qid, ans);
    }
    if (draftRestorePrompt.current_index < session.questions.length) {
      setCurrentIndex(draftRestorePrompt.current_index);
    }
    setDraftRestorePrompt(null);
  };

  /** D4.6 深化#7：放弃草稿，重新开始。 */
  const handleDiscardDraft = () => {
    if (session) clearDraft(session.session_id);
    setDraftRestorePrompt(null);
  };

  return <div className="bk-overlay">
      <div className="bk-quiz">
        {/* 头部：标题 + 进度 + D4.6 深化#8 倒计时 */}
        <header className="bk-quiz__header">
          <div className="bk-quiz__title">
            <span className="bk-quiz__icon">⚡</span>
            <span>{t("game3d.components.UI.BreakthroughQuiz.k21")}</span>
            <span className="bk-quiz__realm">
              {REALM_MAJOR_LABELS[session.from_realm_major]}
              {REALM_MINOR_LABELS[session.from_realm_minor]} →{' '}
              {REALM_MAJOR_LABELS[session.target_realm_major]}
            </span>
          </div>
          <div className="bk-quiz__progress">
            {t("components.intelligence.ActivityPanel.k39")} <span className="bk-quiz__progress-current">{currentIndex + 1}</span> / {total} {t("game3d.components.UI.BreakthroughQuiz.k22")}
            <span className="bk-quiz__progress-answered">{t("game3d.components.UI.BreakthroughQuiz.k23")} {answeredCount}）</span>
          </div>
          {/* D4.6 深化#8：倒计时显示，剩余 ≤ 60s 红色闪烁 */}
          <div className={`bk-quiz__countdown ${isCountdownRed ? 'is-red' : ''} ${remainingSeconds === 0 ? 'is-zero' : ''}`}>
            <span className="bk-quiz__countdown-icon">⏱</span>
            <span className="bk-quiz__countdown-text">{formatCountdown(remainingSeconds)}</span>
          </div>
        </header>

        {/* D4.3 自适应难度：玩家技能评分条（仅答题态显示，结果态由结果卡接管） */}
        <SkillBar skill={playerSkill} />

        {/* 主体：当前题目 */}
        <div className="bk-quiz__body">
          <QuestionView question={current} index={currentIndex} total={total} answer={currentAnswer} onAnswer={v => setAnswer(current.question_id, v)} />
        </div>

        {/* 底部：导航 + 提交 */}
        <footer className="bk-quiz__footer">
          <button className="game3d-btn" onClick={() => setCurrentIndex(i => Math.max(0, i - 1))} disabled={isFirst}>
            {t("game3d.components.UI.BreakthroughQuiz.k24")}
          </button>

          <div className="bk-quiz__dots">
            {session.questions.map((q, i) => {
            const answered = (answersMap[q.question_id] ?? '').trim().length > 0;
            return <span key={q.question_id} className={`bk-quiz__dot ${i === currentIndex ? 'is-current' : ''} ${answered ? 'is-answered' : ''}`} onClick={() => setCurrentIndex(i)} title={t("game3d.components.UI.BreakthroughQuiz.k25", {
              arg0: i + 1
            })} />;
          })}
          </div>

          {isLast ? <button className="game3d-btn game3d-btn--primary" onClick={handleSubmit} disabled={!allAnswered || submitting} title={!allAnswered ? t("game3d.components.UI.BreakthroughQuiz.k26") : undefined}>
              {submitting ? t("game3d.components.UI.BreakthroughQuiz.k27") : t("game3d.components.UI.BreakthroughQuiz.k28")}
            </button> : <button className="game3d-btn game3d-btn--primary" onClick={() => setCurrentIndex(i => Math.min(total - 1, i + 1))}>
              {t("game3d.components.UI.BreakthroughQuiz.k29")}
            </button>}
        </footer>
      </div>

      {/* D4.6 深化#7：草稿恢复提示（检测到未过期草稿时弹出） */}
      {draftRestorePrompt && (
        <div className="bk-draft-prompt">
          <div className="bk-draft-prompt__body">
            <div className="bk-draft-prompt__title">{t('game3d.components.UI.BreakthroughQuiz.k60')}</div>
            <div className="bk-draft-prompt__desc">
              {t('game3d.components.UI.BreakthroughQuiz.k61', {
                arg0: Object.keys(draftRestorePrompt.answers).length,
                arg1: total,
              })}
            </div>
            <div className="bk-draft-prompt__actions">
              <button className="game3d-btn game3d-btn--primary" onClick={handleRestoreDraft}>
                {t('game3d.components.UI.BreakthroughQuiz.k62')}
              </button>
              <button className="game3d-btn" onClick={handleDiscardDraft}>
                {t('game3d.components.UI.BreakthroughQuiz.k63')}
              </button>
            </div>
          </div>
        </div>
      )}

      {/* D4.6 深化#9：批改等待动画（submitting 时覆盖全屏） */}
      {submitting && (
        <GradingOverlay progress={gradingProgress} phrase={GRADING_PHRASES[gradingPhraseIdx]} />
      )}
    </div>;
}

// ============================================================================
// 子组件 1：单题展示
// ============================================================================

interface QuestionViewProps {
  question: BreakthroughQuestion;
  index: number;
  total: number;
  answer: string;
  onAnswer: (value: string) => void;
}
function QuestionView({
  question,
  index,
  total,
  answer,
  onAnswer
}: QuestionViewProps) {
  // D4.5 多模态：image_choice 题型在题干下方展示媒体占位区
  const isImageChoice = question.question_type === 'image_choice';
  return <div className="bk-question">
      <div className="bk-question__meta">
        <span className="bk-question__type">{QUESTION_TYPE_LABELS[question.question_type]}</span>
        <span className="bk-question__domain">{DOMAIN_LABELS[question.domain_id]}</span>
        <span className="bk-question__point">▓ {question.knowledge_point}</span>
        <span className="bk-question__difficulty">{t("game3d.components.UI.BreakthroughQuiz.k30")}{question.difficulty.toFixed(1)}</span>
        <span className="bk-question__seq">{t("game3d.components.UI.BreakthroughQuiz.k22")} {index + 1}/{total}</span>
      </div>

      <div className="bk-question__content">{question.content}</div>

      {isImageChoice && <MediaBlock question={question} />}

      <div className="bk-question__answer">
        {(question.question_type === 'choice' || question.question_type === 'image_choice') && question.options ? <ChoiceOptions options={question.options} answer={answer} onAnswer={onAnswer} /> : <textarea className="bk-question__textarea" value={answer} onChange={e => onAnswer(e.target.value)} placeholder={question.question_type === 'short_answer' ? t("game3d.components.UI.BreakthroughQuiz.k31") : t("game3d.components.UI.BreakthroughQuiz.k32")} rows={question.question_type === 'application' ? 8 : 4} disabled={false} />}
      </div>
    </div>;
}

// ============================================================================
// 子组件 1.5：D4.5 多模态媒体展示块（image_choice 专用）
// ============================================================================

/**
 * 媒体展示块：当前无文生图能力，渲染占位图框 + media_description 文字描述。
 * 后续接入文生图 API 时，若 media_url 是 http(s) 链接，可改为 <img src={media_url}> 渲染真实图。
 */
function MediaBlock({ question }: { question: BreakthroughQuestion }) {
  const isPlaceholder = !question.media_url || question.media_url.startsWith('placeholder://');
  return <div className="bk-media">
      <div className="bk-media__frame">
        {isPlaceholder ? <div className="bk-media__placeholder">
            <span className="bk-media__placeholder-icon">🖼</span>
            <span className="bk-media__placeholder-text">{t("game3d.components.UI.BreakthroughQuiz.k49")}</span>
          </div> : <img className="bk-media__image" src={question.media_url ?? ''} alt={question.media_description ?? ''} />}
      </div>
      {question.media_description && <div className="bk-media__desc">
          <span className="bk-media__desc-label">{t("game3d.components.UI.BreakthroughQuiz.k48")}</span>
          <span className="bk-media__desc-text">{question.media_description}</span>
        </div>}
    </div>;
}

// ============================================================================
// 子组件 2：选择题选项
// ============================================================================

interface ChoiceOptionsProps {
  options: string[];
  answer: string;
  onAnswer: (value: string) => void;
}
function ChoiceOptions({
  options,
  answer,
  onAnswer
}: ChoiceOptionsProps) {
  // 选项 label：A/B/C/D
  const labels = useMemo(() => options.map((_, i) => String.fromCharCode(65 + i)), [options]);
  return <div className="bk-options">
      {options.map((opt, i) => <label key={i} className={`bk-option ${answer === labels[i] ? 'is-selected' : ''}`}>
          <input type="radio" name="bk-choice" value={labels[i]} checked={answer === labels[i]} onChange={() => onAnswer(labels[i])} />
          <span className="bk-option__label">{labels[i]}</span>
          <span className="bk-option__text">{opt}</span>
        </label>)}
    </div>;
}

// ============================================================================
// 子组件 3：结果摘要卡
// ============================================================================

interface BreakthroughResultCardProps {
  session: BreakthroughSession;
  outcome: BreakthroughOutcome;
  onClose: () => void;
}
function BreakthroughResultCard({
  session,
  outcome,
  onClose
}: BreakthroughResultCardProps) {
  const isSuccess = outcome.result === 'success';
  const isDropped = outcome.result === 'dropped';
  return <div className="bk-overlay">
      <div className={`bk-result ${isSuccess ? 'is-success' : isDropped ? 'is-dropped' : 'is-failed'}`}>
        <header className="bk-result__header">
          <div className="bk-result__title">
            {isSuccess ? t("game3d.components.UI.BreakthroughQuiz.k33") : isDropped ? t("game3d.components.UI.BreakthroughQuiz.k34") : t("game3d.components.UI.BreakthroughQuiz.k35")}
          </div>
          <div className="bk-result__subtitle">
            {REALM_MAJOR_LABELS[session.from_realm_major]}
            {REALM_MINOR_LABELS[session.from_realm_minor]} →{' '}
            {isSuccess ? REALM_MAJOR_LABELS[outcome.new_realm_major] : t("game3d.components.UI.BreakthroughQuiz.k36")}
          </div>
        </header>

        <div className="bk-result__grid">
          <div className="bk-result__row">
            <span className="bk-result__label">{t("game3d.components.UI.BreakthroughQuiz.k37")}</span>
            <span className="bk-result__value bk-result__value--score">
              {outcome.score.toFixed(0)} <span className="bk-result__unit">/ 100</span>
            </span>
          </div>
          <div className="bk-result__row">
            <span className="bk-result__label">{t("game3d.components.UI.BreakthroughQuiz.k38")}</span>
            <span className="bk-result__value">
              {DAO_FOUNDATION_LABELS[outcome.daoji_awarded]}
            </span>
          </div>
          <div className="bk-result__row">
            <span className="bk-result__label">{isSuccess ? t("game3d.components.UI.BreakthroughQuiz.k39") : t("game3d.components.UI.BreakthroughQuiz.k40")}</span>
            <span className="bk-result__value">
              {REALM_MAJOR_LABELS[outcome.new_realm_major]}
              {REALM_MINOR_LABELS[outcome.new_realm_minor]}
              <span className="bk-result__sub">
                {' '}· {DAO_FOUNDATION_LABELS[outcome.new_dao_foundation]}
              </span>
            </span>
          </div>
          {outcome.cooldown_until && <div className="bk-result__row">
              <span className="bk-result__label">{t("game3d.components.UI.BreakthroughQuiz.k41")}</span>
              <span className="bk-result__value bk-result__value--cooldown">
                {new Date(outcome.cooldown_until).toLocaleString('zh-CN', {
              hour12: false
            })}
              </span>
            </div>}
          {outcome.weakness_updated && <div className="bk-result__row">
              <span className="bk-result__label">{t("game3d.components.UI.BreakthroughQuiz.k42")}</span>
              <span className="bk-result__value">{t("game3d.components.UI.BreakthroughQuiz.k43")}</span>
            </div>}
        </div>

        <div className="bk-result__review">
          <div className="bk-result__review-title">{t("game3d.components.UI.BreakthroughQuiz.k44")}</div>
          <div className="bk-result__review-text">{outcome.ai_review || t("game3d.components.UI.BreakthroughQuiz.k45")}</div>
        </div>

        {/* D4.6 深化#11：个性化复习建议卡片（仅失败/跌落 + AI 返回非空时展示） */}
        {outcome.review_suggestions && outcome.review_suggestions.length > 0 && (
          <ReviewSuggestionsCard suggestions={outcome.review_suggestions} />
        )}

        <footer className="bk-result__footer">
          <button className="game3d-btn game3d-btn--primary" onClick={onClose}>
            {t("game3d.components.UI.BreakthroughQuiz.k46")}
          </button>
        </footer>
      </div>
    </div>;
}

// ============================================================================
// 子组件 4：D4.3 自适应难度 - 玩家技能评分条
// ============================================================================

interface SkillBarProps {
  skill: PlayerSkill | null;
}

/**
 * 玩家技能评分条（D4.3 自适应难度）。
 *
 * 显示内容：
 *   - 技能等级标签（新手/初学/熟练/精通/大师，5 级配色）
 *   - 技能评分（0-100）
 *   - 连胜/连败数（attempt_count > 0 时）
 *   - 当前难度乘数（0.6-1.5，影响本次考验难度）
 *
 * 数据来源：`game.playerSkill(worldId)` → 后端 `GameDifficultyService::get_or_create_skill`。
 * 拉取失败时 skill=null，显示"未评定"，不阻塞答题。
 */
function SkillBar({ skill }: SkillBarProps) {
  if (!skill) {
    return (
      <div className="bk-quiz__skill bk-skill--loading">
        <span className="bk-skill__label">{t("game3d.components.UI.BreakthroughQuiz.k50")}</span>
        <span className="bk-skill__grade bk-skill__grade--none">{t("game3d.components.UI.BreakthroughQuiz.k59")}</span>
      </div>
    );
  }
  const label = skillLabelFromScore(skill.skill_score);
  const labelZh = SKILL_LABEL_KEYS[label];
  return (
    <div className="bk-quiz__skill">
      <span className="bk-skill__label">{t("game3d.components.UI.BreakthroughQuiz.k50")}</span>
      <span className={`bk-skill__grade bk-skill__grade--${label}`}>{labelZh}</span>
      <span className="bk-skill__score">{skill.skill_score.toFixed(0)}</span>
      {skill.attempt_count > 0 && (
        <>
          {skill.streak > 0 && (
            <span className="bk-skill__streak bk-skill__streak--win">
              {t("game3d.components.UI.BreakthroughQuiz.k56", { arg0: skill.streak })}
            </span>
          )}
          {skill.streak < 0 && (
            <span className="bk-skill__streak bk-skill__streak--lose">
              {t("game3d.components.UI.BreakthroughQuiz.k57", { arg0: Math.abs(skill.streak) })}
            </span>
          )}
        </>
      )}
      <span className="bk-skill__multiplier">
        {t("game3d.components.UI.BreakthroughQuiz.k58")} ×{skill.difficulty_multiplier.toFixed(2)}
      </span>
    </div>
  );
}

// ============================================================================
// 子组件 5：D4.6 深化#9 - 批改等待动画（旋转太极图 + 模拟进度 + 修仙文案）
// ============================================================================

interface GradingOverlayProps {
  /** 模拟进度 0-100。 */
  progress: number;
  /** 当前轮播文案。 */
  phrase: string;
}

/**
 * 批改等待动画。
 *
 * UI 元素：
 *   - 中心旋转太极图（CSS animation 旋转，阴阳鱼用 SVG）
 *   - 进度条（0-100%，前 80% 快速推进，后 20% 等待真实响应）
 *   - 修仙文案轮播（每 1.5s 切换）
 *
 * 设计意图：批改通常需 2-5s（云端 API 响应），动画填补等待空白，营造"天道审视"沉浸感。
 */
function GradingOverlay({ progress, phrase }: GradingOverlayProps) {
  return (
    <div className="bk-grading-overlay">
      <div className="bk-grading-overlay__inner">
        {/* 旋转太极图（SVG 阴阳鱼 + CSS 旋转） */}
        <div className="bk-grading-overlay__taiji">
          <svg viewBox="0 0 100 100" width="80" height="80">
            <circle cx="50" cy="50" r="48" fill="#000" stroke="#00FF00" strokeWidth="2" />
            <path
              d="M 50 2 A 48 48 0 0 1 50 98 A 24 24 0 0 1 50 50 A 24 24 0 0 0 50 2 Z"
              fill="#00FF00"
            />
            <circle cx="50" cy="26" r="6" fill="#000" />
            <circle cx="50" cy="74" r="6" fill="#00FF00" />
          </svg>
        </div>

        {/* 文案 */}
        <div className="bk-grading-overlay__phrase">{phrase}</div>

        {/* 进度条 */}
        <div className="bk-grading-overlay__progress-bar">
          <div
            className="bk-grading-overlay__progress-fill"
            style={{ width: `${progress}%` }}
          />
        </div>
        <div className="bk-grading-overlay__progress-text">
          {Math.floor(progress)}%
        </div>
      </div>
    </div>
  );
}

// ============================================================================
// 子组件 6：D4.6 深化#11 - 个性化复习建议卡片
// ============================================================================

interface ReviewSuggestionsCardProps {
  suggestions: ReviewSuggestion[];
}

/**
 * 个性化复习建议卡片。
 *
 * 展示内容：
 *   - 标题"天道建议"
 *   - 3-5 条建议项，每条包含：
 *     - 知识点 + 领域徽章
 *     - 复习原因（reason）
 *     - 具体复习动作（suggested_action）
 *     - 关联 KB 条目 ID（可选，点击跳转）
 *
 * 数据来源：后端 `ai_generate_review_suggestions`（云端 API 生成）。
 * 仅在突破失败/跌落 + AI 调用成功时展示。
 */
function ReviewSuggestionsCard({ suggestions }: ReviewSuggestionsCardProps) {
  return (
    <div className="bk-review-suggestions">
      <div className="bk-review-suggestions__title">
        <span className="bk-review-suggestions__icon">📜</span>
        {t('game3d.components.UI.BreakthroughQuiz.k64')}
      </div>
      <div className="bk-review-suggestions__list">
        {suggestions.map((s, i) => (
          <div key={i} className="bk-review-suggestion">
            <div className="bk-review-suggestion__header">
              <span className="bk-review-suggestion__domain">
                {DOMAIN_LABELS[s.domain_id as GameKnowledgeDomainId] ?? s.domain_id}
              </span>
              <span className="bk-review-suggestion__point">▓ {s.knowledge_point}</span>
            </div>
            <div className="bk-review-suggestion__reason">
              <span className="bk-review-suggestion__reason-label">
                {t('game3d.components.UI.BreakthroughQuiz.k65')}：
              </span>
              {s.reason}
            </div>
            <div className="bk-review-suggestion__action">
              <span className="bk-review-suggestion__action-label">
                {t('game3d.components.UI.BreakthroughQuiz.k66')}：
              </span>
              {s.suggested_action}
            </div>
            {s.related_entry_id && (
              <div className="bk-review-suggestion__entry">
                {t('game3d.components.UI.BreakthroughQuiz.k67')}：#{s.related_entry_id}
              </div>
            )}
          </div>
        ))}
      </div>
    </div>
  );
}