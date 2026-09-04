# Events and Message Passing

## Summary

Use messages when systems should communicate without sharing mutable internals.
An event should carry the facts needed by its consumers and should not expose a
backend object as a shortcut.

## Key points

- Prefer direct function calls inside one cohesive subsystem.
- Use message passing at ownership or scheduling boundaries, not everywhere.
- Input adapters publish normalized actions; game systems emit domain events.
- Channels are bounded when producers can outpace consumers.
- Shutdown and cancellation are explicit messages.

## Example

For the target architecture:

- input adapters publish normalized actions;
- game systems emit domain events;
- the engine consumes render commands or renderer-neutral draw data;
- channels are bounded when producers can outpace consumers;
- shutdown and cancellation are explicit messages.

Prefer direct function calls inside one cohesive subsystem. Message passing is
valuable at ownership or scheduling boundaries, not everywhere.
