import { canSignal } from '../../db';
import { sendTo } from '../send';
import type { ClientCommand } from '../mod-client';

export const handleSendAnswer = (
  userId: string,
  cmd: ClientCommand<'SendAnswer'>,
) => {
  const [targetId, roomId, sdp] = cmd.command.Client.SendAnswer;

  if (!canSignal(userId, targetId, roomId)) {
    console.warn(
      `Dropping SendAnswer from ${userId} to ${targetId} in ${roomId}`,
    );
    sendTo(userId, { Error: 'Cannot signal this user' });

    return;
  }

  sendTo(targetId, { IncomingAnswer: [userId, roomId, sdp] });
};
