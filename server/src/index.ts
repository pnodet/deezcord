import { Hono } from 'hono';
import { createBunWebSocket } from 'hono/bun';
import { initStunServer } from './stun';
import { onClose, onMessage } from './commands';
import { setAllUsersDead } from './db';
import type { Session } from './commands';

initStunServer();

const { upgradeWebSocket, websocket } = createBunWebSocket();

const app = new Hono();

app.get(
  '/ws',
  // The factory runs once per connection, so `session` is per-socket state.
  upgradeWebSocket(_ctx => {
    const session: Session = { userId: null };

    return {
      onMessage: (evt, ws) => onMessage(evt, ws, session),
      onClose: () => onClose(session),
    };
  }),
);

setAllUsersDead();

const port = Number(process.env.PORT ?? 3030);

console.log(`Listening on port ${port}`);

Bun.serve({
  fetch: app.fetch,
  websocket,
  port,
});
