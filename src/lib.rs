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

// TODO: every pipe should return a compute graph (a structure representing what it is doing with
//  iterator api), the graph has acces to features of r/w and pipe features, it will then use them
//  to construct the most optimized way of running a pipe
// write can return an err to allow performing rpc and getting result inplace
// track message generation subscribe reader/writer to ack group

// if we are joining and repeating with condition then we need to track metadata for acks
// else we use ack groups

// join on id

// message features, contains: timestamp, generation, is pointer

// reader list (must be async)
// basic (just read)
// with consumer group
// with acks
// replay specific
// replay after

// writer list
// serial write - executes callback when every system and pipe has read successfully

// pipe
// error handling fns

// pipegraph - generate a template just like useing normal iterators
// 3 traits:
// construct a template
// convert template wrapper into execution wrapper
// execute graph

// topics are behind a pointer anyway
// we need to put r/w states behind a pointer also

// TODO: wrap a topic with a struct that can allow reading an owned value when there are is one
//  reader left for particular message

// TODO: use static memory pools for each message type by using crate injection, we would need to
//  remove duplicates by expanding a macro first, we could do that if we generate a file that has a
//  macro inide it and then rustc -Zunstable-options --pretty=expanded file.rs
// possible speedup (slower than baseline):
// batch size   no mempools             with hashmap    static mempools
// 512          1.46                    1.09            1
// 256          1.92                    1.03            1
// 128          1.85                    1.05            1
// 64           2.94                    1.22            1
// 32           4.84                    1.06            1
// 16           7.07                    1.48            1
// 8            5.53                    1.65            1
// 4            3.09                    2.06            1
// 2            6.7                     3.63            1
// 1            12.27                   6.27            1

// TODO: pipe can receive a topic id from a message and then use that as its destination
// TODO: instead of using structs as messages find a way on how to use tuples there is higher chance
//  that one tuple will have multiple messages which will decrease nuber of topics and their
//  update systems

// 0.1.0 todo list
// graph templating
// graph running
// maybe create generic pipe executor
// column based topics
// an async topic with reader groups
// cleanup - imports, no wranings...
// more examples
// readme goals

// 0.2.0 todo list
// have a feature where all processes use dynamic dispatch to improve compile times while developing
// that feature would replace arguments in a process with &mut dyn TopicReader<T>
// we could force-replace all arguments to improve compile times
// process macro that supports trait obejcts which will always use dynamic dispatch
// unloadable plugins

// NEW
// 3 versions of rw total
// 1 - realtime, uses locks (concureent queue)
// 2 - double buffered, no locks, at the end of epoch buffers are swapped. or quadruple buffered but
//     same rust type. Use quadruple buffering if there are async pipes that need access. First 2
//     buffers are copied, once async hawe processed message then it gets sent to main.
// 3 - double buffered unbounded, readers are same type as above, writers have a method which tells
//     how much space is left, still does bounds checking but should be optimized away, that number
//     gets used in a graph automatically
//
// acks: ack writer writes the topic id along a message, it has a method to check of any failed or
//       acked writes msg, consumer uses acked reader to send ack/fail back to topic id
//       if failed then writer pipe can use custom impl or under pipeline settings use default like
//       send to dead letter inbox, retry specific, replay it and after
// trace: should be compile option
// primitives: Data<T> shared across even async pipes
// v0.2.0 - have our own schedule which will allow to run async pipes, separate async and non async
// at begining poll async once then after each x seconds poll agen, use block_on at the end of a
// stage so that topic systems can run, if mutations are clashing then run them after block_on

mod primitives {
    use std::marker::PhantomData;

    struct Local<T> {
        data: T,
    }

    // if pipes are replicated for speed then it is shared
    struct PipeData<T> {
        data: T,
    }
    struct PipelineData<T> {
        data: T,
    }
    struct WorkflowData<T> {
        data: T,
    }
    struct NodeData<T> {
        data: T,
    }

    struct PipeQuery<T> {
        _t: PhantomData<T>,
    }
    struct PipelineQuery<T> {
        _t: PhantomData<T>,
    }
    struct WorkflowQuery<T> {
        _t: PhantomData<T>,
    }
    struct NodeQuery<T> {
        _t: PhantomData<T>,
    }

    struct Commands; // set reader bound

    struct Stateful<T> {
        _t: PhantomData<T>,
    }

    // specified by user
    // namespace
    struct Query<T> {
        _t: PhantomData<T>,
    }
    struct Data<T> {
        _t: PhantomData<T>,
    }
    // concrete
    struct Actor<T> {
        _t: PhantomData<T>,
    }
    struct BoundedReader<T> {
        _t: PhantomData<T>,
    }
    struct BoundedWriter<T> {
        _t: PhantomData<T>,
    }

    struct PipeSet {}
}
