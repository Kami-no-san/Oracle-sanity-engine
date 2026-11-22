// ---------------------------------------------------------------------------
// Oracle Sanity Engine — WebSocket Client Hub
//
// Tracks the sockets connected to `/ws` so that the event listener can push
// newly-detected circuit-breaker trips to every open client.
//
// Broadcasting is best-effort: sockets that are no longer open are dropped,
// and sockets whose outbound buffer is already saturated are skipped for that
// message instead of letting a slow consumer grow memory without bound.
// ---------------------------------------------------------------------------

/** WebSocket.OPEN from the WHATWG/`ws` readyState constants. */
const OPEN = 1;

/** Outbound buffer size above which a message is dropped for that client. */
const MAX_BUFFERED_BYTES = 1_000_000;

/** Minimal structural type shared by the `ws` and browser WebSocket APIs. */
export interface WsClient {
  readyState: number;
  bufferedAmount?: number;
  send(data: string): void;
}

/** Message pushed to clients when a new circuit-breaker event is stored. */
export interface NewEventMessage<TEvent = unknown> {
  type: "new_event";
  event: TEvent;
}

const clients = new Set<WsClient>();

/** Adds a socket to the broadcast registry. */
export function registerClient(socket: WsClient): void {
  clients.add(socket);
}

/** Removes a socket from the broadcast registry. */
export function unregisterClient(socket: WsClient): void {
  clients.delete(socket);
}

/** Number of sockets currently tracked (open or not yet pruned). */
export function clientCount(): number {
  return clients.size;
}

/**
 * Sends a pre-serialised payload to every open client.
 *
 * Closed sockets and sockets that fail to accept the message are removed.
 * Sockets with a saturated outbound buffer are skipped for this message and
 * kept registered so they can catch up later.
 */
export function broadcastRaw(message: string): void {
  for (const socket of clients) {
    if (socket.readyState !== OPEN) {
      clients.delete(socket);
      continue;
    }

    if (
      typeof socket.bufferedAmount === "number" &&
      socket.bufferedAmount > MAX_BUFFERED_BYTES
    ) {
      console.warn("[WS] Dropping message for slow client");
      continue;
    }

    try {
      socket.send(message);
    } catch (err) {
      console.error("[WS] Failed to send to client, removing it:", err);
      clients.delete(socket);
    }
  }
}

/** Broadcasts an arbitrary JSON-serialisable payload to every open client. */
export function broadcast(payload: unknown): void {
  broadcastRaw(JSON.stringify(payload));
}

/** Broadcasts a `new_event` message for a freshly stored event. */
export function broadcastNewEvent(event: unknown): void {
  broadcast({ type: "new_event", event } satisfies NewEventMessage);
}
