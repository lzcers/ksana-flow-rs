export const appRoutes = {
  lobby: '/',
  archives: '/archives',
  creation: '/creation',
  generating: '/generating',
  gameplay: '/play',
  ending: '/ending',
} as const;

export const SESSION_ID_QUERY_KEY = 'session_id';

export function readSessionIdFromSearch(search: string): string | null {
  const sessionId = new URLSearchParams(search).get(SESSION_ID_QUERY_KEY)?.trim();
  return sessionId || null;
}

export function routeWithSession(route: string, sessionId: string): string {
  const search = new URLSearchParams({
    [SESSION_ID_QUERY_KEY]: sessionId,
  });

  return `${route}?${search.toString()}`;
}
