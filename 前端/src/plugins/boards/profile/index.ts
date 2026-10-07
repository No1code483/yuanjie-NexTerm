import { lazy } from 'react';
import { definePlugin } from '@/kernel/registry/definePlugin';
import { withAuthGuard } from '@/routes/AuthGuard';
import { ROUTES } from '@/routes/routes';
import { PROFILE_IPC_METHODS } from './ipc/profile';
import { manifest } from './manifest';

const ProfilePage = lazy(() => import('./Profile'));
/** 守卫参数用绝对路径（决定权限资源），路由注册用相对路径由 buildRouter 拼父前缀 /home */
const GuardedProfilePage = lazy(async () => ({
  default: withAuthGuard(ProfilePage, ROUTES.PROFILE)
}));

export default definePlugin({
  manifest,
  contributions: {
    routes: [
      { path: 'profile', component: GuardedProfilePage },
      { path: 'profile/account', component: GuardedProfilePage },
      { path: 'profile/resume', component: GuardedProfilePage },
      { path: 'profile/setting', component: GuardedProfilePage },
      { path: 'profile/logout', component: GuardedProfilePage },
      { path: 'profile/quote', component: GuardedProfilePage }
    ],
    navItems: [
      {
        target: 'kernel:corner',
        labelKey: 'hooks.useModuleTheme.profile',
        order: 10,
        routePath: ROUTES.PROFILE
      }
    ],
    slotComponents: [],
    ipc: {
      namespace: 'pf',
      methods: PROFILE_IPC_METHODS
    }
  }
});
