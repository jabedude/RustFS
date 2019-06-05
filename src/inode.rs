use time;
use time::Timespec;
use std::mem;
use std::ptr;
use std::ptr::copy_nonoverlapping;
use std::io::{Result, Error, ErrorKind};

const PAGE_SIZE: usize = 4096;
const LIST_SIZE: usize = 256;
const FILE_SIZE: usize = 8388608;

type Page = Box<([u8; PAGE_SIZE])>;
type Entry = Page;
type EntryList = TList<Entry>; // TODO: Option<TList> for lazy loading
type DoubleEntryList = TList<EntryList>;
pub type TList<T> = Box<([Option<T>; LIST_SIZE])>;

#[inline(always)]
fn ceil_div(x: usize, y: usize) -> usize {
  return (x + y - 1) / y;
}

#[inline(always)]
pub fn create_tlist<T>() -> TList<T> {
  let mut list: TList<T> = Box::new(unsafe { mem::uninitialized() });
  for x in list.iter_mut() { unsafe { ptr::write(x, None); } };
  list
}

pub struct Inode {
    store: Vec<u8>,
    size: usize,

    mod_time: Timespec,
    access_time: Timespec,
    create_time: Timespec,
}

impl Inode {
  pub fn new() -> Inode {
    let time_now = time::get_time();
    let mut store = Vec::with_capacity(FILE_SIZE);

    Inode {
      store: store,
      size: 0,

      mod_time: time_now,
      access_time: time_now,
      create_time: time_now
    }
  }

  //fn get_or_alloc_page<'a>(&'a mut self, num: usize) -> &'a mut Page {
  //  if num >= LIST_SIZE + LIST_SIZE * LIST_SIZE {
  //    panic!("Maximum file size exceeded!")
  //  };

  //  // Getting a pointer to the page
  //  let page = if num < LIST_SIZE {
  //    // if the page num is in the singly-indirect list
  //    &mut self.single[num]
  //  } else {
  //    // if the page num is in the doubly-indirect list. We allocate a new
  //    // entry list where necessary (*entry_list = ...)
  //    let double_entry = num - LIST_SIZE;
  //    let slot = double_entry / LIST_SIZE;
  //    let entry_list = &mut self.double[slot];

  //    match *entry_list {
  //      None => *entry_list = Some(create_tlist()),
  //      _ => { /* Do nothing */ }
  //    }

  //    let entry_offset = double_entry % LIST_SIZE;
  //    &mut entry_list.as_mut().unwrap()[entry_offset]
  //  };

  //  match *page {
  //    None => *page = Some(Box::new([0u8; 4096])),
  //    _ => { /* Do Nothing */ }
  //  }

  //  page.as_mut().unwrap()
  //}

  //fn get_page<'a>(&'a self, num: usize) -> &'a Option<Page> {
  //  if num >= LIST_SIZE + LIST_SIZE * LIST_SIZE {
  //    panic!("Page does not exist.")
  //  };

  //  if num < LIST_SIZE {
  //    &self.single[num]
  //  } else {
  //    let double_entry = num - LIST_SIZE;
  //    let slot = double_entry / LIST_SIZE;
  //    let entry_offset = double_entry % LIST_SIZE;
  //    let entry_list = &self.double[slot];

  //    match *entry_list {
  //      None => panic!("Page does not exist."),
  //      _ => &entry_list.as_ref().unwrap()[entry_offset]
  //    }
  //  }
  //}

  pub fn write(&mut self, offset: usize, data: &[u8]) -> Result<usize> {
    //println!("************");
    //println!("offset: {}", offset);
    //println!("store.capacity: {}", self.store.capacity());
    //println!("store.len: {}", self.store.len());
    //println!("data.len: {}, offset+data.len: {}", data.len(), offset+data.len());

    let end = offset + data.len();
    //if self.store.capacity() < end {
    //    return Err(Error::new(ErrorKind::Other, "OOM"))
    //}

    if offset + data.len() >= self.store.len() {
        //self.store.reserve(offset + data.len());
        self.store.resize(offset + data.len(), b'\0');
    }

    //TODO: bench this
    //self.store.extend_from_slice(data);
    let slice = &mut self.store[offset..offset+data.len()];
    unsafe {
        let src = data.as_ptr();
        copy_nonoverlapping(src, slice.as_mut_ptr(), data.len());
    }

    let time_now = time::get_time();
    self.mod_time = time_now;
    self.access_time = time_now;

    Ok(data.len())
  }

  pub fn read(&self, offset: usize, data: &mut [u8]) -> usize {
    let slice = &self.store[offset..data.len()];
    unsafe {
        let dst = data.as_mut_ptr();
        copy_nonoverlapping(slice.as_ptr(), dst, data.len());
    }

    //self.access_time = time::get_time();
    data.len()
  }

  pub fn size(&self) -> usize {
    self.store.len()
  }

  pub fn stat(&self) -> (Timespec, Timespec, Timespec) {
    (self.create_time, self.access_time, self.mod_time)
  }
}

#[cfg(test)]
mod tests {
  extern crate rand;

  use super::{Inode};
  use self::rand::random;
  use time;

  fn rand_array(size: usize) -> Vec<u8> {
    (0..size).map(|_| random::<u8>()).collect()
  }

  #[test]
  fn test_simple_write() {
    const SIZE: usize = 4096 * 8 + 3434;

    let original_data = rand_array(SIZE);
    let time_now = time::get_time();
    let mut inode = Inode::new();
    let mut buf = [0u8; SIZE];

    // Write the random data, read it back into buffer
    inode.write(0, original_data.as_slice());
    inode.read(0, &mut buf);

    // Make sure inode is right size
    assert_eq!(SIZE, inode.size());

    // Make sure contents are correct
    for i in 0..SIZE {
      assert_eq!(buf[i], original_data[i]);
    }

    let (create, _, _) = inode.stat();
    assert_eq!(create.sec, time_now.sec);
  }
}
