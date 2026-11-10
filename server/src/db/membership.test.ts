import { afterAll, beforeEach, expect, test } from 'bun:test';
import {
  canSignal,
  createRoom,
  createUser,
  db,
  getRooms,
  getUserById,
  setAllUsersDead,
  setUserRoom,
} from '.';

// bun test shares one module instance (one database) across test files, so
// start from empty tables and leave them empty.
const clear = () => {
  db.run('DELETE FROM users');
  db.run('DELETE FROM rooms');
};

beforeEach(() => {
  clear();
  for (const id of ['a', 'b', 'c']) createUser(id, `name-${id}`);
});

afterAll(clear);

test('rooms table has no users_ids column', () => {
  const columns = db
    .query<{ name: string }, []>("SELECT name FROM pragma_table_info('rooms')")
    .all()
    .map(column => column.name);

  expect(columns).toEqual(['id', 'name']);
});

test('joining puts the user in the room', () => {
  const room = createRoom('r1');

  setUserRoom('a', room.id);
  setUserRoom('b', room.id);

  expect(getRooms()).toEqual([
    { id: room.id, name: 'r1', users: expect.arrayContaining(['a', 'b']) },
  ]);
  expect(getRooms()[0]?.users).toHaveLength(2);
});

test('a user is in at most one room and the room left empty is deleted', () => {
  const r1 = createRoom('r1');
  const r2 = createRoom('r2');

  setUserRoom('a', r1.id);
  setUserRoom('b', r2.id);
  setUserRoom('a', r2.id);

  expect(getRooms()).toEqual([
    { id: r2.id, name: 'r2', users: expect.arrayContaining(['a', 'b']) },
  ]);
});

test('leaving removes the user and deletes the room when it becomes empty', () => {
  const room = createRoom('r1');

  setUserRoom('a', room.id);
  setUserRoom('b', room.id);
  setUserRoom('a', null);

  expect(getRooms()).toEqual([{ id: room.id, name: 'r1', users: ['b'] }]);
  expect(getUserById('a')?.room_id).toBeNull();

  setUserRoom('b', null);

  expect(getRooms()).toEqual([]);
});

test('startup cleanup takes every user out of every room', () => {
  const room = createRoom('r1');

  setUserRoom('a', room.id);
  setAllUsersDead();

  expect(getUserById('a')).toMatchObject({ alive: 0, room_id: null });
  expect(getRooms()).toEqual([{ id: room.id, name: 'r1', users: [] }]);
});

test('canSignal: members of the room, or a lobby user and a member', () => {
  const r1 = createRoom('r1');
  const r2 = createRoom('r2');

  setUserRoom('a', r1.id);
  setUserRoom('c', r2.id);

  // b is in the lobby, about to join r1: offer and ICE go out before Join.
  expect(canSignal('b', 'a', r1.id)).toBe(true);
  // a answers b, who is not a member yet.
  expect(canSignal('a', 'b', r1.id)).toBe(true);

  setUserRoom('b', r1.id);
  expect(canSignal('a', 'b', r1.id)).toBe(true);

  // Cross-room and wrong-room attempts are refused.
  expect(canSignal('c', 'a', r1.id)).toBe(false);
  expect(canSignal('c', 'a', r2.id)).toBe(false);
  expect(canSignal('a', 'b', r2.id)).toBe(false);
  // Unknown users, and signaling yourself.
  expect(canSignal('a', 'nobody', r1.id)).toBe(false);
  expect(canSignal('a', 'a', r1.id)).toBe(false);
});
