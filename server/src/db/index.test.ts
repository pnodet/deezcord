// Characterization tests: they pin what the db module does TODAY, bugs
// included. A test marked "BUG" is expected to change in a later plan.
// `as unknown` marks places where the runtime value contradicts the declared
// TypeScript type (that mismatch is part of what is being pinned).
import { beforeEach, expect, test } from 'bun:test';
import {
  addUserToRoom,
  clearRooms,
  createRoom,
  createUser,
  db,
  getRooms,
  getSharedSecret,
  getUserById,
  setAllUsersDead,
  setSharedSecret,
} from '.';

beforeEach(() => {
  db.run('DELETE FROM users');
  db.run('DELETE FROM rooms');
  db.run('DELETE FROM shared_secrets');
});

test('uses the in-memory database under bun test', () => {
  expect(db.filename).toBe(':memory:');
});

test('createUser returns null but inserts the user', () => {
  // BUG: the INSERT has no RETURNING clause, so the caller never gets the user.
  expect(createUser('user-a', 'alice')).toBeNull();
  expect({ ...getUserById('user-a') } as unknown).toEqual({
    id: 'user-a',
    username: 'alice',
    alive: 1, // SQLite BOOLEAN comes back as a number, not `true`
    room_id: null,
  });
});

test('getUserById returns null for an unknown id', () => {
  expect(getUserById('nobody')).toBeNull();
});

test('createRoom stores the creator as the only member', () => {
  const room = createRoom('user-a', 'lobby');

  expect(room.id).toMatch(/^[\da-f]{8}-[\da-f]{4}-7/); // a UUID v7
  expect(room).toEqual({
    id: room.id,
    name: 'lobby',
    users_ids: 'user-a',
    users: ['user-a'],
  });
  expect(getRooms()).toEqual([room]);
});

test('addUserToRoom appends a user once and ignores duplicates', () => {
  const { id } = createRoom('user-a', 'lobby');

  expect(addUserToRoom(id, 'user-b')?.users).toEqual(['user-a', 'user-b']);
  expect(addUserToRoom(id, 'user-b')?.users).toEqual(['user-a', 'user-b']);
  expect(getRooms()[0]?.users_ids).toBe('user-a,user-b');
});

test('addUserToRoom throws for an unknown room', () => {
  expect(() => addUserToRoom('missing', 'user-a')).toThrow('Room not found');
});

test('getRooms maps a NULL users_ids to an empty users list', () => {
  db.run(
    "INSERT INTO rooms (id, name, users_ids) VALUES ('room-x', 'empty', NULL)",
  );

  expect(getRooms() as unknown).toEqual([
    { id: 'room-x', name: 'empty', users_ids: null, users: [] },
  ]);
});

test('clearRooms deletes only rooms whose users_ids is NULL', () => {
  db.run(
    "INSERT INTO rooms (id, name, users_ids) VALUES ('room-x', 'empty', NULL)",
  );
  const kept = createRoom('user-a', 'lobby');

  clearRooms();

  expect(getRooms().map(room => room.id)).toEqual([kept.id]);
});

test('rooms keep their users after every user is marked dead', () => {
  // BUG: rooms never lose users. Nothing removes a user from users_ids, so
  // the startup sequence (setAllUsersDead + clearRooms) keeps every room.
  createUser('user-a', 'alice');
  const room = createRoom('user-a', 'lobby');

  setAllUsersDead();
  clearRooms();

  expect(getRooms()).toEqual([room]);
});

test('getSharedSecret returns the row object, not the secret string', () => {
  // BUG: typed `string | null`, but the query returns `{ secret }`.
  setSharedSecret('stun-user', 'test-secret');

  expect(getSharedSecret('stun-user') as unknown).toEqual({
    secret: 'test-secret',
  });
});
