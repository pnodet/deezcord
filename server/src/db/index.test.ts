// Characterization tests: they pin what the db module does TODAY, bugs
// included. A test marked "BUG" is expected to change in a later plan.
// `as unknown` marks places where the runtime value contradicts the declared
// TypeScript type (that mismatch is part of what is being pinned).
import { beforeEach, expect, test } from 'bun:test';
import {
  createUser,
  db,
  getSharedSecret,
  getUserById,
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

test('getSharedSecret returns the row object, not the secret string', () => {
  // BUG: typed `string | null`, but the query returns `{ secret }`.
  setSharedSecret('stun-user', 'test-secret');

  expect(getSharedSecret('stun-user') as unknown).toEqual({
    secret: 'test-secret',
  });
});
