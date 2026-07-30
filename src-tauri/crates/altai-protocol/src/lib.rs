//! Public protocol v1 for the ALTAI agent host.
//!
//! Transport framing intentionally has no stdin/stdout dependency. Hosts and
//! clients feed byte buffers into [`FrameDecoder`] and write [`encode_frame`]
//! output to their chosen transport.

pub mod frame;
pub mod message;

pub use frame::{encode_frame, FrameDecoder, FrameError, FrameLimits};
pub use message::{
    validate_message, AgentProtocolError, JsonRpcErrorCode, ProtocolMessage, RunSequenceTracker,
    PROTOCOL_VERSION,
};
