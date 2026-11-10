import { canSignal } from '../../db';
import { sendTo } from '../send';
import type { ClientCommand } from '../mod-client';

export const handleSendIceCandidate = (
  userId: string,
  cmd: ClientCommand<'SendIceCandidate'>,
) => {
  const [targetId, roomId, candidate] = cmd.command.Client.SendIceCandidate;

  if (!canSignal(userId, targetId, roomId)) {
    console.warn(
      `Dropping SendIceCandidate from ${userId} to ${targetId} in ${roomId}`,
    );
    sendTo(userId, { Error: 'Cannot signal this user' });

    return;
  }

  sendTo(targetId, { IncomingIceCandidate: [userId, roomId, candidate] });
};
