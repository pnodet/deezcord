// Checks the shared wire fixtures in /protocol-fixtures against the server.
// The client checks the same files in client/src/commands/mod.rs.
import { readFileSync, readdirSync } from 'node:fs';
import path from 'node:path';
import { beforeEach, expect, test } from 'bun:test';
import { db, setUserRoom } from '../db';
import { onMessage } from '.';
import { sockets } from './send';
import type { Session } from '.';
import type { WSContext, WSMessageReceive } from 'hono/ws';
import type { ClientCommand } from './mod-client';
import type { ServerCommand } from './mod-server';

const FIXTURES_DIR = path.join(import.meta.dir, '../../../protocol-fixtures');

const readFixture = (name: string): string =>
  readFileSync(path.join(FIXTURES_DIR, name), 'utf8');

const SDP = 'v=0\r\n';
const CANDIDATE =
  '{"candidate":"candidate:1 1 udp 2130706431 192.0.2.1 50000 typ host","sdpMid":"","sdpMLineIndex":0,"usernameFragment":null}';

// One typed value per fixture file. `bun run typecheck` proves each value
// fits the TypeScript protocol types; the tests below prove each value equals
// its JSON file.
const clientMessages = {
  'client-connect.json': {
    user_id: 'user-a',
    command: { Client: { Connect: 'alice' } },
  } satisfies ClientCommand<'Connect'>,
  'client-list-rooms.json': {
    user_id: 'user-a',
    command: { Client: 'ListRooms' },
  } satisfies ClientCommand<'ListRooms'>,
  'client-create-room.json': {
    user_id: 'user-a',
    command: { Client: { CreateRoom: 'lobby' } },
  } satisfies ClientCommand<'CreateRoom'>,
  'client-join.json': {
    user_id: 'user-b',
    command: { Client: { Join: 'room-1' } },
  } satisfies ClientCommand<'Join'>,
  'client-send-offer.json': {
    user_id: 'user-a',
    command: { Client: { SendOffer: ['user-b', 'room-1', SDP] } },
  } satisfies ClientCommand<'SendOffer'>,
  'client-send-answer.json': {
    user_id: 'user-b',
    command: { Client: { SendAnswer: ['user-a', 'room-1', SDP] } },
  } satisfies ClientCommand<'SendAnswer'>,
  'client-send-ice-candidate.json': {
    user_id: 'user-a',
    command: { Client: { SendIceCandidate: ['user-b', 'room-1', CANDIDATE] } },
  } satisfies ClientCommand<'SendIceCandidate'>,
};

const serverMessages = {
  'server-ack.json': {
    user_id: 'user-a',
    command: { Server: { Ack: null } },
  } satisfies ServerCommand<'Ack'>,
  'server-room-list.json': {
    user_id: 'user-a',
    command: {
      Server: {
        RoomList: [
          {
            id: 'room-1',
            name: 'lobby',
            users: ['user-a', 'user-b'],
          },
        ],
      },
    },
  } satisfies ServerCommand<'RoomList'>,
  'server-incoming-offer.json': {
    user_id: 'user-b',
    command: { Server: { IncomingOffer: ['user-a', 'room-1', SDP] } },
  } satisfies ServerCommand<'IncomingOffer'>,
  'server-incoming-answer.json': {
    user_id: 'user-a',
    command: { Server: { IncomingAnswer: ['user-b', 'room-1', SDP] } },
  } satisfies ServerCommand<'IncomingAnswer'>,
  'server-incoming-ice-candidate.json': {
    user_id: 'user-b',
    command: {
      Server: { IncomingIceCandidate: ['user-a', 'room-1', CANDIDATE] },
    },
  } satisfies ServerCommand<'IncomingIceCandidate'>,
  'server-error.json': {
    user_id: 'user-a',
    command: { Server: { Error: 'Room not found' } },
  } satisfies ServerCommand<'Error'>,
};

// One fake connection: what the server sends to it, and its per-socket state.
const fakeSocket = () => {
  const sent: unknown[] = [];
  const ws = {
    send: (data: string) => {
      sent.push(JSON.parse(data));
    },
    close: () => {},
  } as unknown as WSContext;
  const session: Session = { userId: null };

  return { ws, sent, session };
};

type FakeSocket = ReturnType<typeof fakeSocket>;

const receive = (socket: FakeSocket, data: string) => {
  onMessage(
    { data } as MessageEvent<WSMessageReceive>,
    socket.ws,
    socket.session,
  );
};

// A fake socket that has already sent Connect (its Ack is discarded).
const connected = (userId: string) => {
  const socket = fakeSocket();

  receive(
    socket,
    JSON.stringify({
      user_id: userId,
      command: { Client: { Connect: userId } },
    }),
  );
  socket.sent.length = 0;

  return socket;
};

beforeEach(() => {
  sockets.clear();
  db.run('DELETE FROM users');
  db.run('DELETE FROM rooms');
});

test('every fixture file has a typed message and vice versa', () => {
  expect(readdirSync(FIXTURES_DIR).sort()).toEqual(
    [...Object.keys(clientMessages), ...Object.keys(serverMessages)].sort(),
  );
});

test.each(Object.entries({ ...clientMessages, ...serverMessages }))(
  '%s matches its typed message',
  (name, message) => {
    expect(JSON.parse(readFixture(name))).toEqual(message);
  },
);

test('Connect is answered with the Ack fixture', () => {
  const socket = fakeSocket();

  receive(socket, readFixture('client-connect.json'));

  expect(socket.sent).toEqual([JSON.parse(readFixture('server-ack.json'))]);
});

test('ListRooms (a bare-string unit variant) is dispatched', () => {
  const socket = connected('user-a');

  receive(socket, readFixture('client-list-rooms.json'));

  expect(socket.sent).toEqual([
    { user_id: 'user-a', command: { Server: { RoomList: [] } } },
  ]);
});

test.each([
  ['client-send-offer.json', 'server-incoming-offer.json'],
  ['client-send-answer.json', 'server-incoming-answer.json'],
  ['client-send-ice-candidate.json', 'server-incoming-ice-candidate.json'],
])('%s is relayed as %s', (input, output) => {
  const clients: Record<string, FakeSocket> = {
    'user-a': connected('user-a'),
    'user-b': connected('user-b'),
  };

  db.run("INSERT INTO rooms (id, name) VALUES ('room-1', 'lobby')");
  setUserRoom('user-a', 'room-1');
  setUserRoom('user-b', 'room-1');

  const sender = JSON.parse(readFixture(input)).user_id as string;
  const expected = JSON.parse(readFixture(output));

  receive(clients[sender], readFixture(input));

  // Only the target receives it, addressed to itself.
  expect(clients[expected.user_id].sent).toEqual([expected]);
  expect(clients[sender].sent).toEqual([]);
});
