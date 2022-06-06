#[cfg(feature = "dynamic")]
#[allow(unused_imports)]
use zmaxion_dylib;
pub use zmaxion_internal::*;

// pipe could have a scale function which is called before spawning a pipeline. We could use that
// to determine how many pipe instances to run and set correct arguments.

// To spawn a workflow we send id to a topic.
// Then it gets picked up and moved to a priority topics, low, medium, high
// When reading workflows to spawn we read from high priority first
//
// Constraint: We cannot spawn a pipeline accross multiple nodes.
//
// TODO: implement state
// TODO: implement reader ack
// TODO: implement consumer groups to MemTopic<T>
// TODO: autoscale inside one node
// spawn the whole pipeline even the shared ones
// we can track execution time before/after
// if pipe can be executed concurrently and writers support out of order execution then try scale
// add one pipe and let it execute, compare execution speed
// if speed1 >= speed2 * 2 then keep it
// if we use wasi containers then we could keep track of memory consumption
// TODO: distributed autoscale
// we spawn a whole pipeline on one node, each pipe can be spawned on multiple nodes if it can be
// executed in parallel, if it crosses to another node then we upgrade topic
// if topic is in memory we don't store it
// if resource usage is too high, we can evict pipelines/pipe
// TODO: execution where data is located
// having higher replication factor makes things faster because we don't need to move data as much
// TODO: wasi containers

// visualization
// s3 -> load -> topic(mem)
// topic -> decompress -> topic # this one is stateless and should be merged up for best performance
// topic -> filter -> topic # also stateless
// topic -> draw_when_batched

// range_generator -> t -> backtest -> t -> compress -> t -> save_to_s3
//                                       -> filter   -> t -> draw_when_batched -> t -> save_to_s3
// if reader is statefull, attach generation to writer message
// pass all metadata through topics
// once a message has been procesed at the end of all pipe groups ack with stateful reader
// this requires that a pipe can only send one message per pipe invocation

// TODO: consumer groups
// TODO: synchronization: last writer sends data to first reader that has written
// TODO: error routing, pipes that return error can be piped to specific topic
// TODO: locals, memory that is private to pipes
// TODO: states, persistent memory
// TODO: backoff strategy
// TODO: pipe definition, require only one field
// use swappable r/w where reader can be upgraded when another pipe is spawned that requires more
// restrictive features
// add metadata between topics

// keep in mind that these are not iterators
// reader_item.join(reader_item2).write(writer, |id0, id1| Data{id0, id1})
// reader.join(reader2).for_each(|a, b| reader3.join(a).write(writer2))
// allow for easy reading but make writers unsafe
