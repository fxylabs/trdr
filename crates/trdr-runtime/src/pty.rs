//! The agent's terminal.
//!
//! What lands here (sections 3 and 9.1): spawning one Claude Code or Codex child
//! under a pseudo-terminal, owning it, and moving bytes both ways.
//!
//! The bytes are bytes. Section 1 is explicit that agent output is shown as a
//! raw stream and is never read as product state, and section 11 forbids
//! converting between terminal content and a query model in either direction.
//!
//! Nothing is implemented yet.
