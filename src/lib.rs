extern crate time;
#[macro_use]
extern crate bitflags;

mod directory;
mod file;
mod inode;

use file::{RcInode, File, FileHandle};
use file::File::{EmptyFile, DataFile, Directory};
use time::Timespec;
use std::rc::Rc;
use std::cell::{RefCell};
use std::collections::HashMap;
use std::io::{Result, Error, ErrorKind};
use directory::DirectoryHandle;
pub use file::Whence;
pub use inode::Inode;

pub type FileDescriptor = isize;

bitflags!{
    pub struct FileFlags: u32 {
        const O_RDONLY =   0b00000001;
        const O_WRONLY =   0b00000010;
        const O_RDWR =     0b00000100;
        const O_NONBLOCK = 0b00001000;
        const O_APPEND =   0b00010000;
        const O_CREAT =    0b00100000;
    }
}

//TODO: paths ("/dir/datafile")
pub struct Vfs<'r> {
  cwd: File<'r>,
  fd_table: HashMap<FileDescriptor, FileHandle<'r>>,
  fds: Vec<FileDescriptor>
}

impl<'r> Vfs<'r> {
  pub fn new() -> Vfs<'r> {
    Vfs {
      cwd: File::new_dir(None),
      fd_table: HashMap::new(),
      fds: (0..(256 - 2)).map(|i| 256 - i).collect(),
    }
  }

  #[inline(always)]
  fn extract_fd(fd_opt: &Option<FileDescriptor>) -> FileDescriptor {
    match fd_opt {
      &Some(fd) => fd,
      &None => panic!("Error in FD allocation.")
    }
  }

  pub fn open(&mut self, path: &'r str, flags: FileFlags) -> Result<FileDescriptor> {
    let lookup = self.cwd.get(path);
    let file = match lookup {
      Some(f) => f,
      None => {
        if (flags & FileFlags::O_CREAT) == FileFlags::O_CREAT {
          // FIXME: Fetch from allocator
          let rcinode = Rc::new(RefCell::new(Box::new(Inode::new())));
          let file = File::new_data_file(rcinode);
          self.cwd.insert(path, file.clone());
          file
        } else {
          EmptyFile
        }
      }
    };

    match file {
      DataFile(_) => {
        let fd = Vfs::extract_fd(&self.fds.pop());
        let handle = FileHandle::new(file);
        self.fd_table.insert(fd, handle);
        Ok(fd)
      }
      Directory(_) => Err(Error::new(ErrorKind::Other, "Directory")),
      EmptyFile => Err(Error::new(ErrorKind::Other, "EmptyFile")),
    }
  }

  pub fn get_stats(&mut self, fd: FileDescriptor) -> (Timespec, Timespec, Timespec) {
    let handle = self.fd_table.get_mut(&fd).expect("fd does not exist");
    let inode = handle.file.get_inode_rc();
    inode.borrow().stat()
  }

  pub fn rename(&mut self, old_path: &'r str, new_path: &'r str) -> Result<()> {
    let lookup = self.cwd.get(old_path);
    match lookup {
      Some(f) => {
        self.unlink(old_path);
        self.cwd.insert(new_path, f);
        Ok(())
    }
      None => Err(Error::new(ErrorKind::NotFound, "fd not found")),
    }
  }

  pub fn chdir(&mut self, new_path: &'r str) -> Result<()> {
    unimplemented!();
  }

  pub fn read(&self, fd: FileDescriptor, dst: &mut [u8]) -> Result<usize> {
    let handle = match self.fd_table.get(&fd) {
        Some(h) => h,
        None => return Err(Error::new(ErrorKind::NotFound, "fd not found")),
    };
    Ok(handle.read(dst))
  }

  pub fn write(&mut self, fd: FileDescriptor, src: &[u8]) -> usize {
    let handle = self.fd_table.get_mut(&fd).expect("fd does not exist");
    handle.write(src)
  }

  pub fn seek(&mut self, fd: FileDescriptor, o: isize, whence: Whence) -> usize {
    let handle = self.fd_table.get_mut(&fd).expect("fd does not exist");
    handle.seek(o, whence)
  }

  pub fn close(&mut self, fd: FileDescriptor) {
    self.fd_table.remove(&fd);
    self.fds.push(fd);
  }

  pub fn unlink(&mut self, path: &'r str) {
    self.cwd.remove(path);
  }
}

#[cfg(test)]
mod proc_tests {
  // extern crate test;
  extern crate rand;

  use super::{Vfs, FileFlags};
  use file::Whence::SeekSet;
  use inode::Inode;
  use self::rand::random;

  static mut test_inode_drop: bool = false;

  impl Drop for Inode {
    fn drop(&mut self) {
      unsafe {
        if test_inode_drop {
          test_inode_drop = false;
          panic!("Dropping.");
        } else {
          println!("Dropping, but no flag.");
        }
      }
    }
  }

  fn rand_array(size: usize) -> Vec<u8> {
    (0..size).map(|_| random::<u8>()).collect()
  }

  fn assert_eq_buf(first: &[u8], second: &[u8]) {
    assert_eq!(first.len(), second.len());

    for i in 0..first.len() {
      assert_eq!(first[i], second[i]);
    }
  }

  #[test]
  fn test_rename_simple() {
    const SIZE: usize = 4096 * 8 + 3434;
    let mut p = Proc::new();
    let data = rand_array(SIZE);
    let mut buf = [0u8; SIZE];
    let filename = "first_file";
    let newname = "new_file";

    let fd = p.open(filename, FileFlags::O_RDWR | FileFlags::O_CREAT).expect("open failed!");

    p.write(fd, &data);
    p.seek(fd, 0, SeekSet);
    assert!(p.rename(filename, newname).is_ok());

    p.close(fd);
    let ret = p.open(filename, FileFlags::O_RDWR);
    assert!(ret.is_err());
  }

  #[test]
  fn test_rename_old_nonexistent() {
    let mut p = Proc::new();
    let filename = "first_file";
    let newname = "new_file";

    let ret = p.rename(filename, newname);
    assert!(ret.is_err());
  }

  #[test]
  fn test_inode_stat_time() {
    const SIZE: usize = 4096 * 8 + 3434;
    let mut p = Vfs::new();
    let data = rand_array(SIZE);
    let mut buf = [0u8; SIZE];
    let filename = "first_file";

    let fd = p.open(filename, FileFlags::O_RDWR | FileFlags::O_CREAT).expect("open failed!");

    let (ctime, atime, mtime) = p.get_stats(fd);
    // All three timestamps should be equal after creation.
    assert_eq!((ctime, atime), (atime, mtime));

    p.write(fd, &data);
    p.seek(fd, 0, SeekSet);
    p.read(fd, &mut buf);

    let (ctime, atime, mtime) = p.get_stats(fd);
    assert_ne!((ctime, atime), (atime, mtime));
  }

  #[test]
  fn simple_test() {
    const SIZE: usize = 4096 * 8 + 3434;
    let mut p = Vfs::new();
    let data = rand_array(SIZE);
    let mut buf = [0u8; SIZE];
    let filename = "first_file";

    let fd = p.open(filename, FileFlags::O_RDWR | FileFlags::O_CREAT).expect("open failed!");
    p.write(fd, &data);
    p.seek(fd, 0, SeekSet);
    p.read(fd, &mut buf);

    assert_eq_buf(&data, &buf);

    let fd2 = p.open(filename, FileFlags::O_RDWR).expect("open failed!");
    let mut buf2 = [0u8; SIZE];
    p.read(fd2, &mut buf2);

    assert_eq_buf(&data, &buf2);

    p.close(fd);
    p.close(fd2);

    let fd3 = p.open(filename, FileFlags::O_RDWR).expect("open failed!");
    let mut buf3 = [0u8; SIZE];
    p.read(fd3, &mut buf3);

    assert_eq_buf(&data, &buf3);
    p.close(fd3);

    p.unlink(filename);

    let fd4 = p.open(filename, FileFlags::O_RDWR);
    assert!(fd4.is_err());
  }

  #[test]
  #[should_panic]
  fn test_proc_drop_inode_dealloc() {
    // Variable is used to make sure that the Drop implemented is only valid for
    // tests that set that test_inode_drop global variable to true.
    unsafe { test_inode_drop = true; }

    const SIZE: usize = 4096 * 3 + 3498;
    let mut p = Vfs::new();
    let mut data = rand_array(SIZE);

    let fd = p.open("file", FileFlags::O_RDWR | FileFlags::O_CREAT).expect("open failed!");
    p.write(fd, &mut data);
  }

  /**
   * This function makes sure that on unlink, the inode's data structure is
   * indeed dropped. This means that a few things have gone right:
   *
   * 1) The FileHandle was dropped. If it wasn't, it would hold a reference to
   *    the file and so the file wouldn't be dropped. This should happen on
   *    close.
   * 2) The File, containing the Inode, was dropped. This should happen on
   *    unlink.
   */
  #[test]
  #[should_panic]
  fn test_inode_dealloc() {
    // Make sure flag is set to detect drop.
    unsafe { test_inode_drop = true; }

    const SIZE: usize = 4096 * 3 + 3498;
    let mut p = Vfs::new();
    let mut data = rand_array(SIZE);
    let mut buf = [0u8; SIZE];
    let filename = "first_file";

    let fd = p.open(filename, FileFlags::O_RDWR | FileFlags::O_CREAT).expect("open failed!");
    p.write(fd, &mut data);
    p.seek(fd, 0, SeekSet);
    p.read(fd, &mut buf);

    assert_eq_buf(&data, &buf);

    // close + unlink should remove both references to inode, dropping it,
    // causing a failure
    p.close(fd);
    p.unlink(filename);

    // If inode is not being dropped properly, ie, on the unlink call this will
    // cause a double failure: once for panic! call, and once when then the Inode
    // is dropped since the Vfs structure will be dropped.
    //
    // To test that RC is working properly, make sure that a double failure
    // occurs when either the close or unlink calls above are commented out.
    panic!("Inode not dropped!");
  }

  //#[test]
  //fn test_max_singly_file_size() {
  //  const SIZE: usize = 4096 * 256;
  //  let mut p = Vfs::new();
  //  let mut data = rand_array(SIZE);
  //  let mut buf = [0u8; SIZE];
  //  let filename = "first_file";

  //  let fd = p.open(filename, FileFlags::O_RDWR | FileFlags::O_CREAT).expect("open failed!");
  //  p.write(fd, &mut data);
  //  p.seek(fd, 0, SeekSet);
  //  p.read(fd, &mut buf);

  //  assert_eq_buf(&data, &buf);

  //  p.close(fd);
  //  p.unlink(filename);

  //  let fd4 = p.open(filename, FileFlags::O_RDWR);
  //  assert!(fd4.is_err());
  //}

  //#[test]
  //fn test_max_file_size() {
  //  const SIZE: usize = 2 * 4096 * 256;
  //  let mut p = Vfs::new();
  //  let mut data1 = rand_array(SIZE);
  //  let mut data2 = rand_array(SIZE);
  //  let mut buf = vec![0; SIZE];
  //  let filename = "first_file";

  //  let fd = p.open(filename, FileFlags::O_RDWR | FileFlags::O_CREAT).expect("open failed!");
  //  p.write(fd, &mut data1);
  //  p.seek(fd, 4096 * 257 * 256 - SIZE as isize, SeekSet);
  //  p.write(fd, &mut data2);

  //  p.seek(fd, 0, SeekSet);
  //  p.read(fd, &mut buf);
  //  assert_eq_buf(&data1, &buf);

  //  p.seek(fd, 4096 * 257 * 256 - SIZE as isize, SeekSet);
  //  p.read(fd, &mut buf);
  //  assert_eq_buf(&data2, &buf);
  //}

  //#[test]
  //#[should_panic]
  //fn test_morethan_max_file_size() {
  //  const SIZE: usize = 2 * 4096 * 256;
  //  let mut p = Vfs::new();
  //  let mut data = rand_array(SIZE);
  //  let filename = "first_file";

  //  let fd = p.open(filename, FileFlags::O_RDWR | FileFlags::O_CREAT).expect("open failed!");
  //  p.write(fd, &mut data);
  //  p.seek(fd, 4096 * 257 * 256 + 1 - SIZE as isize, SeekSet);
  //  p.write(fd, &mut data);
  //}
}
