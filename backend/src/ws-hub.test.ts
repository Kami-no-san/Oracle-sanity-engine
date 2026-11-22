import test from "node:test";
import assert from "node:assert/strict";

import {
  broadcast,
  broadcastNewEvent,
  clientCount,
  registerClient,
  unregisterClient,
} from "./ws-hub";
import { handleTrip, type CircuitBreakerEvent } from "./listener";

class FakeSocket {
  readyState = 1;
  bufferedAmount = 0;
  sent: string[] = [];

  send(data: string): void {
    this.sent.push(data);
  }
}

test("broadcast reaches every open client and prunes closed sockets", () => {
  const open = new FakeSocket();
  const closed = new FakeSocket();
  closed.readyState = 3; // CLOSED
  registerClient(open);
  registerClient(closed);

  try {
    broadcast({ type: "heartbeat" });

    assert.deepEqual(open.sent, [JSON.stringify({ type: "heartbeat" })]);
    assert.equal(closed.sent.length, 0);
    assert.equal(clientCount(), 1);
  } finally {
    unregisterClient(open);
    unregisterClient(closed);
  }
});

test("unregisterClient stops delivery", () => {
  const socket = new FakeSocket();
  registerClient(socket);
  unregisterClient(socket);

  broadcast({ type: "heartbeat" });

  assert.equal(socket.sent.length, 0);
  assert.equal(clientCount(), 0);
});

test("slow clients are skipped without being removed", () => {
  const slow = new FakeSocket();
  slow.bufferedAmount = 2_000_000;
  registerClient(slow);

  try {
    broadcast({ type: "heartbeat" });

    assert.equal(slow.sent.length, 0);
    assert.equal(clientCount(), 1);
  } finally {
    unregisterClient(slow);
  }
});

test("a client that throws on send is removed", () => {
  const broken = new FakeSocket();
  broken.send = () => {
    throw new Error("socket gone");
  };
  registerClient(broken);

  broadcast({ type: "heartbeat" });

  assert.equal(clientCount(), 0);
});

test("broadcastNewEvent sends the payload expected by the dashboard", () => {
  const socket = new FakeSocket();
  registerClient(socket);

  try {
    const event = { id: "trip-1", reasonCode: 1 };
    broadcastNewEvent(event);

    assert.equal(socket.sent.length, 1);
    assert.deepEqual(JSON.parse(socket.sent[0]), {
      type: "new_event",
      event,
    });
  } finally {
    unregisterClient(socket);
  }
});

test("handleTrip stores the trip and broadcasts it to connected clients", async () => {
  const socket = new FakeSocket();
  registerClient(socket);

  const event: CircuitBreakerEvent = {
    id: "trip-42",
    reason: "Feeds diverged beyond threshold",
    reasonCode: 1,
    deviationThresholdBps: 500,
    maxStalenessSecs: 600,
    detectedAt: "2026-01-01T00:00:00.000Z",
  };

  try {
    await handleTrip(event);

    assert.equal(socket.sent.length, 1);
    assert.deepEqual(JSON.parse(socket.sent[0]), {
      type: "new_event",
      event,
    });
  } finally {
    unregisterClient(socket);
  }
});
