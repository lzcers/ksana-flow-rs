import { type ReactNode, useEffect } from 'react';
import { Navigate, Route, Routes, useLocation, useNavigate } from 'react-router-dom';
import {
  appRoutes,
  isCloneShareSearch,
  readSessionIdFromSearch,
  routeWithSession,
} from './lib/appRoutes';
import { installNavigator } from './lib/navigation';
import LobbyPage from './pages/LobbyPage';
import ArchiveListPage from './pages/ArchiveListPage';
import CreationPage from './pages/CreationPage';
import GeneratingPage from './pages/GeneratingPage';
import GameplayPage from './pages/GameplayPage';
import EndingPage from './pages/EndingPage';
import { useGameInternalStore } from './store/gameStore';
import { useGameUIStore } from './store/gameUIStore';

function NavigationBridge() {
  const navigate = useNavigate();

  useEffect(() => {
    installNavigator(navigate);
    return () => installNavigator(null);
  }, [navigate]);

  return null;
}

function GeneratingRoute() {
  const sessionId = useGameInternalStore((state) => state.sessionId);
  const stateView = useGameUIStore((state) => state.stateView);
  const startupStage = useGameUIStore((state) => state.startupStage);
  const preparedProfiles = useGameUIStore((state) => state.preparedProfiles);

  if (sessionId && stateView && startupStage === 'idle') {
    return <Navigate to={routeWithSession(appRoutes.gameplay, sessionId)} replace />;
  }

  if (startupStage === 'idle' && !preparedProfiles) {
    return <Navigate to={appRoutes.creation} replace />;
  }

  return <GeneratingPage />;
}

function SessionRestoreGate({ sessionId }: { sessionId: string }) {
  const restoreSession = useGameUIStore((state) => state.restoreSession);
  const isLoading = useGameUIStore((state) => state.isLoading);
  const error = useGameUIStore((state) => state.error);

  useEffect(() => {
    void restoreSession(sessionId).catch(() => {
      // The store keeps the user-facing error and navigates back to the lobby.
    });
  }, [restoreSession, sessionId]);

  return (
    <div className="flex h-full w-full items-center justify-center px-6 text-center">
      <div className="max-w-md space-y-3 rounded-[1.1rem] border border-[#6d86b7]/25 bg-[#101827]/78 px-5 py-5 text-[#efe4cd] shadow-[0_10px_30px_rgba(3,8,18,0.25)]">
        <p className="text-sm font-semibold tracking-[0.18em] text-[#8fa4ca]">SESSION</p>
        <p className="text-lg font-medium">
          {isLoading ? '正在续上这段旅程...' : '正在打开旅程...'}
        </p>
        {error ? <p className="text-sm leading-6 text-[#ffd7d7]">{error}</p> : null}
      </div>
    </div>
  );
}

function SessionCloneGate({ sourceSessionId }: { sourceSessionId: string }) {
  const cloneSharedSession = useGameUIStore((state) => state.cloneSharedSession);
  const isLoading = useGameUIStore((state) => state.isLoading);
  const error = useGameUIStore((state) => state.error);
  const navigate = useNavigate();

  useEffect(() => {
    void cloneSharedSession(sourceSessionId)
      .then((cloned) => {
        navigate(
          routeWithSession(
            cloned.isEnding ? appRoutes.ending : appRoutes.gameplay,
            cloned.sessionId,
          ),
          { replace: true },
        );
      })
      .catch(() => {
        // The store keeps the user-facing error and navigates back to the lobby.
      });
  }, [cloneSharedSession, navigate, sourceSessionId]);

  return (
    <div className="flex h-full w-full items-center justify-center px-6 text-center">
      <div className="max-w-md space-y-3 rounded-[1.1rem] border border-[#6d86b7]/25 bg-[#101827]/78 px-5 py-5 text-[#efe4cd] shadow-[0_10px_30px_rgba(3,8,18,0.25)]">
        <p className="text-sm font-semibold tracking-[0.18em] text-[#8fa4ca]">SHARE</p>
        <p className="text-lg font-medium">
          {isLoading ? '正在复制这段旅程...' : '正在打开分享旅程...'}
        </p>
        <p className="text-sm leading-6 text-[#a8b4c7]">
          会为你生成一条独立分支，原玩家的故事不会被改变。
        </p>
        {error ? <p className="text-sm leading-6 text-[#ffd7d7]">{error}</p> : null}
      </div>
    </div>
  );
}

function SessionRouteGuard({
  route,
  children,
}: {
  route: typeof appRoutes.gameplay | typeof appRoutes.ending;
  children: ReactNode;
}) {
  const location = useLocation();
  const requestedSessionId = readSessionIdFromSearch(location.search);
  const shouldCloneSession = isCloneShareSearch(location.search);
  const sessionId = useGameInternalStore((state) => state.sessionId);
  const stateView = useGameUIStore((state) => state.stateView);

  if (requestedSessionId && shouldCloneSession) {
    return <SessionCloneGate sourceSessionId={requestedSessionId} />;
  }

  if (requestedSessionId && (sessionId !== requestedSessionId || !stateView)) {
    return <SessionRestoreGate sessionId={requestedSessionId} />;
  }

  if (!sessionId || !stateView) {
    return <Navigate to={appRoutes.lobby} replace />;
  }

  if (!requestedSessionId) {
    return <Navigate to={routeWithSession(route, sessionId)} replace />;
  }

  if (route === appRoutes.gameplay && stateView.isEnding) {
    return <Navigate to={routeWithSession(appRoutes.ending, sessionId)} replace />;
  }

  if (route === appRoutes.ending && !stateView.isEnding) {
    return <Navigate to={routeWithSession(appRoutes.gameplay, sessionId)} replace />;
  }

  return children;
}

function GameplayRoute() {
  return (
    <SessionRouteGuard route={appRoutes.gameplay}>
      <GameplayPage />
    </SessionRouteGuard>
  );
}

function EndingRoute() {
  return (
    <SessionRouteGuard route={appRoutes.ending}>
      <EndingPage />
    </SessionRouteGuard>
  );
}

function App() {
  return (
    <div className="relative h-dvh w-full overflow-hidden bg-background">
      <div className="pointer-events-none absolute inset-0 z-0">
        <div className="absolute -left-24 top-12 h-72 w-72 rounded-full bg-sky-500/10 blur-3xl" />
        <div className="absolute bottom-10 -right-16 h-80 w-80 rounded-full bg-indigo-500/10 blur-3xl" />
        <div className="absolute inset-y-0 left-[8%] w-px bg-white/5" />
        <div className="absolute inset-y-0 right-[8%] w-px bg-white/5" />
      </div>

      <main className="akashic-scroll relative z-10 h-full w-full touch-pan-y overflow-y-auto overflow-x-hidden">
        <NavigationBridge />
        <Routes>
          <Route path={appRoutes.lobby} element={<LobbyPage />} />
          <Route path={appRoutes.archives} element={<ArchiveListPage />} />
          <Route path={appRoutes.creation} element={<CreationPage />} />
          <Route path={appRoutes.generating} element={<GeneratingRoute />} />
          <Route path={appRoutes.gameplay} element={<GameplayRoute />} />
          <Route path={appRoutes.ending} element={<EndingRoute />} />
          <Route path="*" element={<Navigate to={appRoutes.lobby} replace />} />
        </Routes>
      </main>
    </div>
  );
}

export default App;
