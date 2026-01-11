// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

//! Live state streaming support for operator UIs.
//!
//! This module provides read-only, non-authoritative state change notifications
//! via WebSocket connections. Events represent facts about what changed in the
//! canonical state, not directives or domain logic.
//!
//! # Architecture
//!
//! - Events are broadcast to all connected clients
//! - Events are informational only and never authoritative
//! - No commands are executed over WebSocket connections
//! - No audit events are emitted for streaming activity
//! - Clients must still query canonical state via HTTP APIs for authoritative data

use axum::{
    extract::{
        State as AxumState, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    response::Response,
};
use futures::{SinkExt, stream::StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{debug, error, info, warn};

/// Maximum number of events to buffer in the broadcast channel.
/// If clients cannot keep up, older events will be dropped.
const EVENT_BUFFER_SIZE: usize = 100;

/// Live state event types.
///
/// These events represent changes to canonical state and are purely informational.
/// They are derived from successful state transitions, not the source of truth.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LiveEvent {
    /// A bid year was created.
    BidYearCreated {
        /// The year identifier.
        year: u16,
    },
    /// An area was created within a bid year.
    AreaCreated {
        /// The bid year.
        bid_year: u16,
        /// The area identifier.
        area: String,
    },
    /// A user was registered in an area.
    UserRegistered {
        /// The bid year.
        bid_year: u16,
        /// The area identifier.
        area: String,
        /// The user's initials.
        initials: String,
    },
    /// A user's data was updated.
    UserUpdated {
        /// The bid year.
        bid_year: u16,
        /// The area identifier.
        area: String,
        /// The user's initials.
        initials: String,
    },
    /// A checkpoint was created.
    CheckpointCreated {
        /// The bid year.
        bid_year: u16,
        /// The area identifier.
        area: String,
    },
    /// A rollback occurred.
    RolledBack {
        /// The bid year.
        bid_year: u16,
        /// The area identifier.
        area: String,
    },
    /// A round was finalized.
    RoundFinalized {
        /// The bid year.
        bid_year: u16,
        /// The area identifier.
        area: String,
    },
    /// Connection confirmation (sent on initial connect).
    Connected {
        /// Server timestamp (ISO 8601).
        timestamp: String,
    },
}

/// Broadcaster for live state events.
///
/// This is a lightweight wrapper around `tokio::sync::broadcast` that allows
/// multiple WebSocket clients to receive state change notifications.
#[derive(Clone)]
pub struct LiveEventBroadcaster {
    /// The broadcast channel sender.
    tx: broadcast::Sender<LiveEvent>,
}

impl LiveEventBroadcaster {
    /// Creates a new event broadcaster.
    #[must_use]
    pub fn new() -> Self {
        let (tx, _rx) = broadcast::channel(EVENT_BUFFER_SIZE);
        Self { tx }
    }

    /// Broadcasts an event to all connected clients.
    ///
    /// If no clients are connected, the event is silently dropped.
    /// This is non-blocking and will not wait for clients to receive the event.
    pub fn broadcast(&self, event: &LiveEvent) {
        match self.tx.send(event.clone()) {
            Ok(count) => {
                debug!(?event, receivers = count, "Broadcast live event");
            }
            Err(_) => {
                // No receivers, which is fine
                debug!(?event, "No receivers for live event");
            }
        }
    }

    /// Subscribes to the event stream.
    ///
    /// Returns a receiver that will receive all future events.
    /// Events sent before subscription are not received.
    fn subscribe(&self) -> broadcast::Receiver<LiveEvent> {
        self.tx.subscribe()
    }
}

impl Default for LiveEventBroadcaster {
    fn default() -> Self {
        Self::new()
    }
}

/// WebSocket handler that upgrades HTTP connections and streams live events.
///
/// This handler:
/// - Accepts WebSocket upgrade requests
/// - Sends a connection confirmation event
/// - Streams all future live events to the client
/// - Handles client disconnections gracefully
///
/// Handles WebSocket upgrade requests for live event streaming.
///
/// # Arguments
///
/// * `ws` - WebSocket upgrade request
/// * `broadcaster` - The live event broadcaster from application state
///
/// # Returns
///
/// An HTTP response that upgrades the connection to WebSocket
pub async fn live_events_handler(
    ws: WebSocketUpgrade,
    AxumState(broadcaster): AxumState<Arc<LiveEventBroadcaster>>,
) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, broadcaster))
}

/// Handles an individual WebSocket connection.
///
/// Sends a connection confirmation, then streams all live events until
/// the client disconnects or an error occurs.
async fn handle_socket(socket: WebSocket, broadcaster: Arc<LiveEventBroadcaster>) {
    info!("Client connected to live event stream");

    let (mut sender, mut receiver) = socket.split();
    let mut rx: broadcast::Receiver<LiveEvent> = broadcaster.subscribe();

    // Send connection confirmation
    let connected_event = LiveEvent::Connected {
        timestamp: time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Iso8601::DEFAULT)
            .unwrap_or_else(|_| String::from("unknown")),
    };

    if let Ok(json) = serde_json::to_string(&connected_event)
        && sender.send(Message::Text(json.into())).await.is_err()
    {
        warn!("Failed to send connection confirmation");
        return;
    }

    // Task for sending events to the client
    let mut send_task = tokio::spawn(async move {
        while let Ok(event) = rx.recv().await {
            match serde_json::to_string(&event) {
                Ok(json) => {
                    if sender.send(Message::Text(json.into())).await.is_err() {
                        // Client disconnected
                        break;
                    }
                }
                Err(e) => {
                    error!(?e, "Failed to serialize live event");
                }
            }
        }
    });

    // Task for receiving messages from the client (though we don't expect any)
    let mut recv_task = tokio::spawn(async move {
        while let Some(msg) = receiver.next().await {
            match msg {
                Ok(Message::Text(_) | Message::Binary(_)) => {
                    // We don't process commands over WebSocket
                    warn!("Received unexpected message from client, ignoring");
                }
                Ok(Message::Close(_)) => {
                    debug!("Client sent close frame");
                    break;
                }
                Ok(Message::Ping(_) | Message::Pong(_)) => {
                    // Ping/pong handled automatically by Axum
                }
                Err(e) => {
                    error!(?e, "WebSocket receive error");
                    break;
                }
            }
        }
    });

    // Wait for either task to complete
    tokio::select! {
        _ = &mut send_task => {
            debug!("Send task completed");
            recv_task.abort();
        }
        _ = &mut recv_task => {
            debug!("Receive task completed");
            send_task.abort();
        }
    }

    info!("Client disconnected from live event stream");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_broadcaster_creation() {
        let broadcaster = LiveEventBroadcaster::new();
        assert_eq!(broadcaster.tx.receiver_count(), 0);
    }

    #[test]
    fn test_broadcast_no_receivers() {
        let broadcaster = LiveEventBroadcaster::new();
        // Should not panic when no receivers
        broadcaster.broadcast(&LiveEvent::BidYearCreated { year: 2026 });
    }

    #[test]
    fn test_broadcast_with_receiver() {
        let broadcaster = LiveEventBroadcaster::new();
        let mut rx = broadcaster.subscribe();

        broadcaster.broadcast(&LiveEvent::BidYearCreated { year: 2026 });

        match rx.try_recv() {
            Ok(LiveEvent::BidYearCreated { year: 2026 }) => {}
            other => panic!("Expected BidYearCreated, got {other:?}"),
        }
    }

    #[test]
    fn test_multiple_receivers() {
        let broadcaster = LiveEventBroadcaster::new();
        let mut rx1 = broadcaster.subscribe();
        let mut rx2 = broadcaster.subscribe();

        broadcaster.broadcast(&LiveEvent::AreaCreated {
            bid_year: 2026,
            area: String::from("ZAB"),
        });

        // Both receivers should get the event
        assert!(matches!(rx1.try_recv(), Ok(LiveEvent::AreaCreated { .. })));
        assert!(matches!(rx2.try_recv(), Ok(LiveEvent::AreaCreated { .. })));
    }

    #[test]
    fn test_event_serialization() {
        let event = LiveEvent::UserRegistered {
            bid_year: 2026,
            area: String::from("ZAB"),
            initials: String::from("ABC"),
        };

        let json = serde_json::to_string(&event).expect("Failed to serialize");
        let deserialized: LiveEvent = serde_json::from_str(&json).expect("Failed to deserialize");

        match deserialized {
            LiveEvent::UserRegistered {
                bid_year,
                area,
                initials,
            } => {
                assert_eq!(bid_year, 2026);
                assert_eq!(area, "ZAB");
                assert_eq!(initials, "ABC");
            }
            _ => panic!("Wrong event type"),
        }
    }
}
