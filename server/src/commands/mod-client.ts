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
  Join: {
    room: string;
  };
  Leave: {
    room: string;
  };
  SendOffer: {
    user_id: string;
    sdp: RTCSessionDescription;
  };
  SendAnswer: {
    user_id: string;
    sdp: RTCSessionDescription;
  };
  SendIceCandidate: {
    user_id: string;
    candidate: string;
  };
};

export type ClientCommand<K extends ClientCommandKeys> = {
  user_id: string;
  command: { Client: { [P in K]: ClientCommandData[K] } };
};
