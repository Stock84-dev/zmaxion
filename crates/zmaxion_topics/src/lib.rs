mod built {
    include!(concat!(env!("OUT_DIR"), "/topics.rs"));

    //    pub struct Reader<'s, T>(&'s mut ReaderState<T>);
    //    pub struct Writer<'s, T>(&'s mut WriterState<T>);
}

pub use built::*;
