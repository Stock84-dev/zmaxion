#[macro_export]
macro_rules! read_all_loop {
    ($reader:expr) => {
        match $reader.try_read() {
            Some(x) => x,
            None => continue,
        }
        .read_all()
    };
}

#[macro_export]
macro_rules! read_all {
    ($reader:expr) => {
        match $reader.try_read() {
            Some(x) => x,
            None => return,
        }
        .read_all()
    };
}

#[macro_export]
macro_rules! read_loop {
    ($reader:expr) => {
        match $reader.try_read() {
            Some(x) => x,
            None => continue,
        }
    };
}

#[macro_export]
macro_rules! read {
    ($reader:expr) => {
        match $reader.try_read() {
            Some(x) => x,
            None => return,
        }
        .read()
    };
}
