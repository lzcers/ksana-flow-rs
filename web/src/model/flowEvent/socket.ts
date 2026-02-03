import { Observable } from 'rxjs';
import type { WebSocketFlowMessage } from './types';

export function createFlowSocketObservable(spaceId: string): Observable<WebSocketFlowMessage> {
  return new Observable<WebSocketFlowMessage>((subscriber) => {
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const host = import.meta.env.PROD ? window.location.host : 'localhost:3000';
    const url = `${protocol}//${host}/ws?workspace_id=${spaceId}`;

    const ws = new WebSocket(url);

    ws.onmessage = (event) => {
      try {
        const message = JSON.parse(event.data) as WebSocketFlowMessage;
        subscriber.next(message);
      } catch (error) {
        console.error('WS parse error', error);
      }
    };

    ws.onerror = (error) => {
      subscriber.error(error);
    };

    ws.onclose = () => {
      subscriber.error(new Error('WebSocket closed'));
    };

    return () => {
      if (ws.readyState === WebSocket.OPEN || ws.readyState === WebSocket.CONNECTING) {
        ws.close();
      }
    };
  });
}
