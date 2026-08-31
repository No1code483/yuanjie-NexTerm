import type { UserInfo } from '@/types'
import UserIdentityCard from './UserIdentityCard'
import StatsDashboard from './StatsDashboard'
import AchievementWall from './AchievementWall'
import ActivityTimeline from './ActivityTimeline'

interface AccountPanelProps {
  isTempAccount: boolean
  user: UserInfo
  formatTime: (ts: number) => string
  resumeCount?: number
  quoteCount?: number
  kbCount?: number
}

export default function AccountPanel({
  isTempAccount,
  user,
  formatTime,
  resumeCount = 0,
  quoteCount = 0,
  kbCount = 0,
}: AccountPanelProps) {
  return (
    <div>
      <UserIdentityCard
        user={user}
        isTempAccount={isTempAccount}
        formatTime={formatTime}
      />

      <StatsDashboard
        userId={user.id}
        enabled={!isTempAccount}
        resumeCount={resumeCount}
        quoteCount={quoteCount}
        kbCount={kbCount}
      />

      <AchievementWall
        userId={user.id}
        enabled={!isTempAccount}
        resumeCount={resumeCount}
        quoteCount={quoteCount}
        kbCount={kbCount}
      />

      <ActivityTimeline
        userId={user.id}
        enabled={!isTempAccount}
        limit={8}
      />
    </div>
  )
}