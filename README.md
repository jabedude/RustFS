RustFS
======

RustFS is a virtual file system written completely in Rust.

Usage
-----

Add RustFS to your dependencies:

```toml
[dependencies]
rustfs = { git = "https://github.com/SergioBenitez/RustFS" }
```

Then, import the crate into your project and bring types into the namespace:

```rust
extern crate rustfs;

use rustfs::{Vfs, O_CREAT, O_RDWR};
```

Finally, use `Vfs::new()` to create a new `Vfs`. Call `open` / `close` /
`seek` / `read` / `write` on it:

```rust
let mut p = Vfs::new();

// Let's write `data` to a new file named "file".
let data = b"... some data ...";
  let fd = p.open("file", O_CREAT | O_RDWR);
  p.write(fd, &data);
  p.close(fd);

  // Let's read back that data to a buffer named `buf` of the correct size.
  let mut buf = vec![0; size];
  let fd = p.open("file", O_RDWR);
  p.read(fd, &mut buf);
  p.close(fd);

  // All done. Unlink.
p.unlink("file");
```

For more examples on how to use RustFS, see the benchmarks in bench/bench.rs and
tests in src/proc.rs.

Testing
-------

Run the tests using `RUST_TEST_THREADS=1 cargo test`. The tests need to be run
sequentially.

Benchmarking
------------

You'll need Rust nightly to run the benchmarks. We use a custom built
benchmarking tool to get accurate results, and that benchmarking tool uses
assembly. Assembly can only be used in Rust nightly.

To run the benchmarks, switch into the `bench` directory:

```sh
cd bench
```

Run them with Cargo:

```sh
cargo run --release
```

Directory Structure
-------------------
* bench/
  * bench.rs _The benchmarks._

* libbench/lib.rs _The benchmarking library._

* libslab/lib.rs _The slab allocator library._

* src/
  * directory.rs _Insert/Remove/Get directory method implementations._
  * file.rs _FileHandle implementation and structure definitions._
  * inode.rs _Inode structure and implementation._
  * lib.rs _Vfs structure (which wraps everything) and implementation._
