extern crate rustfs;
extern crate rand;
#[macro_use]
extern crate criterion;


use criterion::Criterion;
use rustfs::{Proc, FileFlags, FileDescriptor};
use std::string::String;
use rand::random;
use std::iter::repeat;

static NUM: usize = 100;

macro_rules! bench {
  ($wrap:ident, $name:ident, $time:expr, |$p:ident, $filenames:ident| $task:stmt) => ({
    let $filenames = generate_names(NUM);
    let $wrap = |b: &mut Benchmarker| {
      let mut $p = Proc::new();
      b.run(|| {
        $task
      });
    };
    benchmark(stringify!($name), $wrap, $time);
  });
}

fn open_close_one() {
    let mut p = Proc::new();
    let fd = p.open("test", FileFlags::O_CREAT).unwrap();
    p.close(fd);
}

fn open_close_unlink() {
    let mut p = Proc::new();
    let filenames = generate_names(NUM);
    let fds = open_many(&mut p, &filenames);
    close_all(&mut p, &fds);
    unlink_all(&mut p, &filenames);
}

fn open_write_close_unlink(content: &[u8]) {
    let mut p = Proc::new();
    let filenames = generate_names(NUM);
    let fds = open_many(&mut p, &filenames);
    for i in 0..NUM {
        let filename = &filenames[i];
        let fd = p.open(filename, FileFlags::O_CREAT | FileFlags::O_RDWR).unwrap();
        p.write(fd, content);
        p.close(fd);
    }
}

fn open_write_large_close(content: &[u8]) {
    let mut p = Proc::new();
    let filenames = generate_names(NUM);
    let fds = open_many(&mut p, &filenames);
    for i in 0..NUM {
        let filename = &filenames[i];
        let fd = p.open(filename, FileFlags::O_CREAT | FileFlags::O_RDWR).unwrap();
        p.write(fd, content);
        p.close(fd);
    }
}

fn open_write_large_close_unlink(content: &[u8]) {
    let mut p = Proc::new();
    let filenames = generate_names(NUM);
    let fds = open_many(&mut p, &filenames);
    for i in 0..NUM {
        let filename = &filenames[i];
        let fd = p.open(filename, FileFlags::O_CREAT | FileFlags::O_RDWR).unwrap();
        p.write(fd, content);
        p.close(fd);
        p.unlink(filename);
    }
}

fn open_write_modify_small_close(content: &[u8]) {
    let mut p = Proc::new();
    let filenames = generate_names(NUM);
    let fds = open_many(&mut p, &filenames);
    for i in 0..NUM {
        let filename = &filenames[i];
        let fd = p.open(filename, FileFlags::O_CREAT | FileFlags::O_RDWR).unwrap();
        for _ in 0..100 {
            p.write(fd, content);
        }
        p.close(fd);
    }
}

fn open_write_modify_small_close_unlink(content: &[u8]) {
    let mut p = Proc::new();
    let filenames = generate_names(NUM);
    let fds = open_many(&mut p, &filenames);
    for i in 0..NUM {
        let filename = &filenames[i];
        let fd = p.open(filename, FileFlags::O_CREAT | FileFlags::O_RDWR).unwrap();
        for _ in 0..100 {
            p.write(fd, content);
        }
        p.close(fd);
        p.unlink(filename);
    }
}

fn open_write_modify_big_close(content: &[u8]) {
    let mut p = Proc::new();
    let filenames = generate_names(NUM);
    let fds = open_many(&mut p, &filenames);
    for i in 0..NUM {
        let filename = &filenames[i];
        let fd = p.open(filename, FileFlags::O_CREAT | FileFlags::O_RDWR).unwrap();
        for _ in 0..32 {
            p.write(fd, content);
        }
        p.close(fd);
    }
}

fn open_write_modify_big_close_unlink(content: &[u8]) {
    let mut p = Proc::new();
    let filenames = generate_names(NUM);
    let fds = open_many(&mut p, &filenames);
    for i in 0..NUM {
        let filename = &filenames[i];
        let fd = p.open(filename, FileFlags::O_CREAT | FileFlags::O_RDWR).unwrap();
        for _ in 0..32 {
            p.write(fd, content);
        }
        p.close(fd);
        p.unlink(filename);
    }
}

macro_rules! bench_many {
  ($wrap:ident, $name:ident, $time:expr, |$p:ident, $fd:ident, $filename:ident| $op:stmt) => ({
    let filenames = generate_names(NUM);
    let $wrap = |b: &mut Benchmarker| {
      let mut $p = Proc::new();
      b.run(|| {
        for i_j in 0..NUM {
          let $filename = &filenames[i_j];
          let $fd = $p.open($filename, FileFlags::O_CREAT | FileFlags::O_RDWR).unwrap();
          $op
        }
      });
    };
    benchmark(stringify!($name), $wrap, $time);
  })
}

fn rand_array(size: usize) -> Vec<u8> {
  (0..size).map(|_| random::<u8>()).collect()
}

fn generate_names(n: usize) -> Vec<String> {
  let name_length = ceil_div(n, 26);
  let mut name: Vec<_> = repeat('@' as u8).take(name_length).collect();

  (0..n).map(|i| {
    let next = name[i / 26] + 1;
    name[i / 26] = next;

    let string_result = String::from_utf8(name.clone());
    match string_result {
      Ok(string) => string,
      Err(_) => panic!("Bad string!")
    }
  }).collect()
}

fn open_many<'a>(p: &mut Proc<'a>, names: &'a Vec<String>) -> Vec<FileDescriptor> {
  (0..names.len()).map(|i| {
    let fd = p.open(&names[i], FileFlags::O_CREAT | FileFlags::O_RDWR).unwrap();
    fd
  }).collect()
}

fn close_all(p: &mut Proc, fds: &Vec<FileDescriptor>) {
  for fd in fds.iter() {
    p.close(*fd);
  }
}

fn unlink_all<'a>(p: &mut Proc<'a>, names: &'a Vec<String>) {
  for filename in names.iter() {
    p.unlink(&filename);
  }
}

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("Open Close 1", |b| b.iter(|| open_close_one()));
}

fn bench_OtCtU(c: &mut Criterion) {
    c.bench_function("Open Close Unlink", |b| b.iter(|| open_close_unlink()));
}

fn bench_OWsCU(c: &mut Criterion) {
    let size = 1024;
    let content = rand_array(size);
    c.bench_function("Open Write Close Unlink", move |b| b.iter(|| open_write_close_unlink(&content)));
}

fn bench_OWbC(c: &mut Criterion) {
    let size = 40960;
    let content = rand_array(size);
    c.bench_function("Open Write Large Close", move |b| b.iter(|| open_write_large_close(&content)));
}

fn bench_OWbCU(c: &mut Criterion) {
    let size = 40960;
    let content = rand_array(size);
    c.bench_function("Open Write Large Close Unlink", move |b| b.iter(|| open_write_large_close_unlink(&content)));
}

fn bench_OWMsC(c: &mut Criterion) {
    let size = 1024;
    let content = rand_array(size);
    c.bench_function("Open Write Modify Small Close", move |b| b.iter(|| open_write_modify_small_close(&content)));
}

fn bench_OWMsCU(c: &mut Criterion) {
    let size = 1024;
    let content = rand_array(size);
    c.bench_function("Open Write Modify Small Close Unlink", move |b| b.iter(|| open_write_modify_small_close_unlink(&content)));
}

fn bench_OWMbC(c: &mut Criterion) {
    let size = 1048576;
    let content = rand_array(size);
    c.bench_function("Open Write Modify big Close", move |b| b.iter(|| open_write_modify_big_close(&content)));
}

fn bench_OWMbCU(c: &mut Criterion) {
    let size = 1048576;
    let content = rand_array(size);
    c.bench_function("Open Write Modify big Close Unlink", move |b| b.iter(|| open_write_modify_big_close_unlink(&content)));
}

criterion_group!(benches, criterion_benchmark, bench_OtCtU, bench_OWsCU, bench_OWbC, bench_OWbCU, bench_OWMsC, bench_OWMsCU, bench_OWMbC);
criterion_main!(benches);

//#[allow(non_snake_case)]
//fn main() {
//
//  bench!(bench_OtC, OtC, 100, |p, filenames| {
//    let fds = open_many(&mut p, &filenames);
//    close_all(&mut p, &fds);
//  });
//
//  bench_many!(bench_OC, OC, 100, |p, fd, _f| {
//    p.close(fd);
//  });
//
//  let size = 1024;
//  let content = rand_array(size);
//  bench_many!(bench_OWsC, OWsC, 100, |p, fd, filename| {
//    p.write(fd, &content);
//    p.close(fd);
//  });
