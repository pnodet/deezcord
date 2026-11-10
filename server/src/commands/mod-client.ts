import {
  object,
  picklist,
  strictObject,
  strictTuple,
  string,
  union,
} from 'valibot';

const VSignal = strictTuple([string(), string(), string()]);

// The client envelope as serde serializes it: unit variants are bare strings,
// the others are objects with exactly one key.
export const VClientMessage = object({
  user_id: string(),
  command: object({
    Client: union([
      picklist(['ListRooms', 'Leave']),
      strictObject({ Connect: string() }),
      strictObject({ CreateRoom: string() }),
      strictObject({ Join: string() }),
      strictObject({ SendOffer: VSignal }),
      strictObject({ SendAnswer: VSignal }),
      strictObject({ SendIceCandidate: VSignal }),
    ]),
  }),
});

export type ClientCommandKeys =
  | 'Join'
  | 'Leave'
  | 'ListRooms'
  | 'CreateRoom'
  | 'Connect'
  | 'SendOffer'
  | 'SendAnswer'
  | 'SendIceCandidate';

// Payloads of the Rust tuple variants in client/src/commands/mod.rs.
// `ListRooms` and `Leave` are Rust unit variants: they have no payload and
// serde sends them as a bare string, e.g. `{ "Client": "ListRooms" }`.
export type ClientCommandData = {
  Connect: string;
  CreateRoom: string;
  Join: string;
  SendOffer: [
    string, // user_id
    string, // room_id
    string, // sdp
  ];
  SendAnswer: [
    string, // user_id
    string, // room_id
    string, // sdp
  ];
  SendIceCandidate: [
    string, // user_id
    string, // room_id
    string, // candidate
  ];
};

export type ClientCommand<K extends ClientCommandKeys> = {
  user_id: string;
  command: {
    Client: K extends keyof ClientCommandData
      ? { [P in K]: ClientCommandData[P] }
      : K;
  };
};
