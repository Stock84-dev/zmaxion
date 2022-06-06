use std::borrow::Cow;

use bevy::{ecs::system::EntityCommands, prelude::*};
pub use mem::{
    MemTopic, ResTopicReaderState, ResTopicWriterState, TopicReaderState, TopicWriterState,
};
mod mem;
