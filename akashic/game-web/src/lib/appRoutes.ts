export const appRoutes = {
  lobby: '/',
  archives: '/archives',
  creation: '/creation',
  generating: '/generating',
  gameplay: '/play',
  ending: '/ending',
} as const;

export const SESSION_ID_QUERY_KEY = 'session_id';
export const SHARE_MODE_QUERY_KEY = 'share';
export const CLONE_SHARE_MODE = 'clone';

export function readSessionIdFromSearch(search: string): string | null {
  const sessionId = new URLSearchParams(search).get(SESSION_ID_QUERY_KEY)?.trim();
  return sessionId || null;
}

export function isCloneShareSearch(search: string): boolean {
  return new URLSearchParams(search).get(SHARE_MODE_QUERY_KEY) === CLONE_SHARE_MODE;
}

export function routeWithSession(route: string, sessionId: string): string {
  const search = new URLSearchParams({
    [SESSION_ID_QUERY_KEY]: sessionId,
  });

  return `${route}?${search.toString()}`;
}

export function routeWithClonedSession(route: string, sessionId: string): string {
  const search = new URLSearchParams({
    [SESSION_ID_QUERY_KEY]: sessionId,
    [SHARE_MODE_QUERY_KEY]: CLONE_SHARE_MODE,
  });

  return `${route}?${search.toString()}`;
}
