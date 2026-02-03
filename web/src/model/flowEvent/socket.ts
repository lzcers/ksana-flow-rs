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
        // We don't error the stream here to avoid breaking the connection on a single bad message
      }
    };

    ws.onerror = (error) => {
      // Error will trigger retry in the consumer
      subscriber.error(error);
    };

    ws.onclose = () => {
      // Treat close as an error to trigger retry
      subscriber.error(new Error('WebSocket closed'));
    };

    return () => {
      if (ws.readyState === WebSocket.OPEN || ws.readyState === WebSocket.CONNECTING) {
        ws.close();
      }
    };
  });
}
