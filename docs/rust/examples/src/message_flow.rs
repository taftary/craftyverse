//! Event and message flow.
//!
//! Demonstrates message passing at an ownership boundary: a game system emits
//! domain events that a consumer handles without sharing mutable internals.
//!
//! See `docs/rust/book/patterns/events.md`.

use std::sync::mpsc;

/// Domain event produced by game systems.
#[derive(Debug, Clone, PartialEq)]
pub enum GameEvent {
    /// A node was split.
    NodeSplit {
        /// Name of the split node.
        name: String,
    },
    /// The plan was regenerated from scratch.
    PlanRegenerated,
}

/// Producer side of the game-event channel.
pub struct EventSource {
    sender: mpsc::Sender<GameEvent>,
}

/// Consumer side of the game-event channel.
pub struct EventSink {
    receiver: mpsc::Receiver<GameEvent>,
}

impl EventSource {
    /// Emits a `NodeSplit` event.
    pub fn split_node(&self, name: &str) {
        self.sender
            .send(GameEvent::NodeSplit {
                name: name.to_string(),
            })
            .ok();
    }

    /// Emits a `PlanRegenerated` event.
    pub fn regenerate_plan(&self) {
        self.sender.send(GameEvent::PlanRegenerated).ok();
    }
}

impl EventSink {
    /// Drains all currently queued events.
    pub fn drain(&self) -> Vec<GameEvent> {
        let mut events = Vec::new();
        while let Ok(event) = self.receiver.try_recv() {
            events.push(event);
        }
        events
    }
}

/// Creates a connected event source and sink.
pub fn channel() -> (EventSource, EventSink) {
    let (sender, receiver) = mpsc::channel();
    (EventSource { sender }, EventSink { receiver })
}

/// Runs the message-flow demonstration.
///
/// Events cross the producer/consumer boundary by value; neither side holds a
/// reference to the other's state.
pub fn run() {
    let (source, sink) = channel();
    source.split_node("north_base_node_0");
    source.regenerate_plan();

    let events = sink.drain();
    assert_eq!(events.len(), 2);
    assert_eq!(
        events[0],
        GameEvent::NodeSplit {
            name: "north_base_node_0".to_string()
        }
    );
}

#[cfg(test)]
mod tests {
    use super::run;

    #[test]
    fn message_flow_demo_runs() {
        run();
    }
}
