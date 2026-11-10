import { afterAll, beforeAll, expect, test } from 'bun:test';
import type { Subprocess } from 'bun';

// Drives the real server (src/index.ts) in a child process with two WebSocket
// clients, the way the Rust client talks to it.
const port = 40_000 + Math.floor(Math.random() * 10_000);
let server: Subprocess;

beforeAll(async () => {
  server = Bun.spawn(['bun', 'src/index.ts'], {
    cwd: `${import.meta.dir}/..`,
    // APP_PORT_UDP4/6 = 0 disables the STUN server (see src/stun/index.ts).
    env: {
      ...process.env,
      PORT: String(port),
      DEEZCORD_DB: ':memory:',
      APP_PORT_UDP4: '0',
      APP_PORT_UDP6: '0',
    },
    stdout: 'ignore',
    stderr: 'inherit',
  });

  for (let attempt = 0; attempt < 50; attempt++) {
    try {
      await fetch(`http://localhost:${port}/`);

      return;
    } catch {
      await Bun.sleep(100);
    }
  }

  throw new Error('server did not start');
});

afterAll(() => {
  server.kill();
});

type Envelope = { user_id: string; command: { Server: any } };

const connect = async (userId: string) => {
  const ws = new WebSocket(`ws://localhost:${port}/ws`);
  const inbox: Envelope[] = [];
  const waiters: (() => void)[] = [];

  ws.onmessage = event => {
    inbox.push(JSON.parse(String(event.data)));
    for (const wake of waiters.splice(0)) wake();
  };
  await new Promise(resolve => (ws.onopen = resolve));

  const client = {
    ws,
    inbox,
    send: (command: unknown, asUserId = userId) =>
      ws.send(
        JSON.stringify({ user_id: asUserId, command: { Client: command } }),
      ),
    // Resolves with the next message whose Server command has key `key`.
    next: async (key: string): Promise<Envelope> => {
      for (;;) {
        const index = inbox.findIndex(m => key in m.command.Server);

        if (index !== -1) return inbox.splice(index, 1)[0];
        await new Promise<void>((resolve, reject) => {
          waiters.push(resolve);
          setTimeout(
            () => reject(new Error(`timed out waiting for ${key}`)),
            2000,
          );
        });
      }
    },
  };

  client.send({ Connect: `name-${userId}` });
  expect((await client.next('Ack')).user_id).toBe(userId);

  return client;
};

test('room membership, targeted signaling and disconnect cleanup', async () => {
  const alice = await connect('alice');
  const bob = await connect('bob');

  // Alice creates a room and receives a RoomList addressed to her.
  alice.send({ CreateRoom: 'general' });
  const created = await alice.next('RoomList');
  expect(created.user_id).toBe('alice');
  const [room] = created.command.Server.RoomList;
  expect(room).toEqual({
    id: expect.any(String),
    name: 'general',
    users: ['alice'],
  });

  // Bob lists rooms, then sends his offer BEFORE Join, like the client does.
  bob.send('ListRooms');
  expect((await bob.next('RoomList')).command.Server.RoomList).toEqual([room]);

  // The envelope user_id is ignored after Connect: bob cannot pose as carol.
  bob.send({ SendOffer: ['alice', room.id, 'sdp-offer'] }, 'carol');
  const offer = await alice.next('IncomingOffer');
  expect(offer).toEqual({
    user_id: 'alice',
    command: { Server: { IncomingOffer: ['bob', room.id, 'sdp-offer'] } },
  });

  // Alice answers bob, who is not a member yet.
  alice.send({ SendAnswer: ['bob', room.id, 'sdp-answer'] });
  expect(
    (await bob.next('IncomingAnswer')).command.Server.IncomingAnswer,
  ).toEqual(['alice', room.id, 'sdp-answer']);

  // Bob joins: both members get a RoomList addressed to themselves.
  bob.send({ Join: room.id });
  for (const [client, id] of [
    [alice, 'alice'],
    [bob, 'bob'],
  ] as const) {
    const list = await client.next('RoomList');
    expect(list.user_id).toBe(id);
    expect(list.command.Server.RoomList[0].users.toSorted()).toEqual([
      'alice',
      'bob',
    ]);
  }

  // Bob disconnects: alice gets the updated list.
  bob.ws.close();
  const afterClose = await alice.next('RoomList');
  expect(afterClose.command.Server.RoomList).toEqual([
    { ...room, users: ['alice'] },
  ]);

  // Alice leaves: the room is deleted and she gets the empty list.
  alice.send('Leave');
  expect((await alice.next('RoomList')).command.Server.RoomList).toEqual([]);
  alice.ws.close();
});

test('commands before Connect are ignored', async () => {
  const ws = new WebSocket(`ws://localhost:${port}/ws`);
  const inbox: string[] = [];

  ws.onmessage = event => inbox.push(String(event.data));
  await new Promise(resolve => (ws.onopen = resolve));
  ws.send(
    JSON.stringify({ user_id: 'mallory', command: { Client: 'ListRooms' } }),
  );
  await Bun.sleep(200);

  expect(inbox).toEqual([]);
  ws.close();
});
