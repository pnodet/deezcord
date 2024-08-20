export type ClientCommandKeys =
  | 'Join'
  | 'Leave'
  | 'ListRooms'
  | 'CreateRoom'
  | 'Connect'
  | 'SendOffer'
  | 'SendAnswer'
  | 'SendIceCandidate';

export type ClientCommandData = {
  ListRooms: null;
  Connect: string;
  CreateRoom: string;
  Join: string;
  Leave: {
    room: string;
  };
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
  command: { Client: { [P in K]: ClientCommandData[K] } };
};
