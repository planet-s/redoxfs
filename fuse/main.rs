//#![deny(warnings)]

extern crate fuse;
extern crate redoxfs;
extern crate system;
extern crate time;

use image::Image;
use std::env;
use std::path::Path;
use time::Timespec;
use fuse::{FileType, FileAttr, Filesystem, Request, ReplyData, ReplyEntry, ReplyAttr, ReplyCreate, ReplyDirectory, ReplyEmpty, ReplyStatfs, ReplyWrite};

pub mod image;

const TTL: Timespec = Timespec { sec: 1, nsec: 0 };                 // 1 second

const CREATE_TIME: Timespec = Timespec { sec: 0, nsec: 0 };

struct RedoxFS {
    fs: redoxfs::FileSystem,
}

impl RedoxFS {
    fn inode_block(&self, ino: u64) -> u64 {
        self.fs.header.1.root + ino - 1
    }

    fn block_inode(&self, block: u64) -> u64 {
        block + 1 - self.fs.header.1.root
    }
}

impl Filesystem for RedoxFS {
    fn lookup(&mut self, _req: &Request, ino: u64, name: &Path, reply: ReplyEntry) {
        let parent_block = self.inode_block(ino);
        match self.fs.find_node(name.to_str().unwrap(), parent_block) {
            Ok(node) => {
                reply.entry(&TTL, &FileAttr {
                    ino: self.block_inode(node.0),
                    size: node.1.extents[0].length,
                    blocks: (node.1.extents[0].length + 511)/512,
                    atime: CREATE_TIME,
                    mtime: CREATE_TIME,
                    ctime: CREATE_TIME,
                    crtime: CREATE_TIME,
                    kind: if node.1.is_dir() {
                        FileType::Directory
                    } else {
                        FileType::RegularFile
                    },
                    perm: 0o777,
                    nlink: 1,
                    uid: 0,
                    gid: 0,
                    rdev: 0,
                    flags: 0,
                }, 0);
            },
            Err(err) => {
                reply.error(err.errno as i32);
            }
        }
    }

    fn getattr(&mut self, _req: &Request, ino: u64, reply: ReplyAttr) {
        let block = self.inode_block(ino);
        match self.fs.node(block) {
            Ok(node) => {
                reply.attr(&TTL, &FileAttr {
                    ino: self.block_inode(node.0),
                    size: node.1.extents[0].length,
                    blocks: (node.1.extents[0].length + 511)/512,
                    atime: CREATE_TIME,
                    mtime: CREATE_TIME,
                    ctime: CREATE_TIME,
                    crtime: CREATE_TIME,
                    kind: if node.1.is_dir() {
                        FileType::Directory
                    } else {
                        FileType::RegularFile
                    },
                    perm: 0o777,
                    nlink: 1,
                    uid: 0,
                    gid: 0,
                    rdev: 0,
                    flags: 0,
                });
            },
            Err(err) => {
                reply.error(err.errno as i32);
            }
        }
    }

    fn setattr(&mut self, _req: &Request, ino: u64, _mode: Option<u32>,
                _uid: Option<u32>, _gid: Option<u32>, _size: Option<u64>,
                _atime: Option<Timespec>, _mtime: Option<Timespec>, _fh: Option<u64>,
                _crtime: Option<Timespec>, _chgtime: Option<Timespec>, _bkuptime: Option<Timespec>,
                _flags: Option<u32>, reply: ReplyAttr) {
        //TODO: Implement truncate
        let block = self.inode_block(ino);
        match self.fs.node(block) {
            Ok(node) => {
                reply.attr(&TTL, &FileAttr {
                    ino: self.block_inode(node.0),
                    size: node.1.extents[0].length,
                    blocks: (node.1.extents[0].length + 511)/512,
                    atime: CREATE_TIME,
                    mtime: CREATE_TIME,
                    ctime: CREATE_TIME,
                    crtime: CREATE_TIME,
                    kind: if node.1.is_dir() {
                        FileType::Directory
                    } else {
                        FileType::RegularFile
                    },
                    perm: 0o777,
                    nlink: 1,
                    uid: 0,
                    gid: 0,
                    rdev: 0,
                    flags: 0,
                });
            },
            Err(err) => {
                reply.error(err.errno as i32);
            }
        }
    }

    fn read(&mut self, _req: &Request, ino: u64, _fh: u64, offset: u64, size: u32, reply: ReplyData) {
        let block = self.inode_block(ino);
        let mut data = vec![0; size as usize];
        match self.fs.read_node(block, offset, &mut data) {
            Ok(count) => {
                reply.data(&data[..count]);
            },
            Err(err) => {
                reply.error(err.errno as i32);
            }
        }
    }

    fn write(&mut self, _req: &Request, ino: u64, _fh: u64, offset: u64, data: &[u8], _flags: u32, reply: ReplyWrite) {
        let block = self.inode_block(ino);
        match self.fs.write_node(block, offset, &data) {
            Ok(count) => {
                reply.written(count as u32);
            },
            Err(err) => {
                reply.error(err.errno as i32);
            }
        }
    }

    fn flush(&mut self, _req: &Request, _ino: u64, _fh: u64, _lock_owner: u64, reply: ReplyEmpty) {
        reply.ok();
    }

    fn fsync(&mut self, _req: &Request, _ino: u64, _fh: u64, _datasync: bool, reply: ReplyEmpty) {
        reply.ok();
    }

    fn readdir(&mut self, _req: &Request, ino: u64, _fh: u64, offset: u64, mut reply: ReplyDirectory) {
        let parent_block = self.inode_block(ino);
        let mut children = Vec::new();
        match self.fs.child_nodes(&mut children, parent_block) {
            Ok(()) => {
                if offset == 0 {
                    let mut i = 0;
                    reply.add(parent_block - self.fs.header.0, i, FileType::Directory, ".");
                    i += 1;
                    reply.add(parent_block - self.fs.header.0, i, FileType::Directory, "..");
                    i += 1;
                    for child in children.iter() {
                        reply.add(child.0 - self.fs.header.0, i, if child.1.is_dir() {
                            FileType::Directory
                        } else {
                            FileType::RegularFile
                        }, child.1.name().unwrap());
                        i += 1;
                    }
                }
                reply.ok();
            },
            Err(err) => {
                reply.error(err.errno as i32);
            }
        }
    }

    fn create(&mut self, _req: &Request, ino: u64, name: &Path, _mode: u32, flags: u32, reply: ReplyCreate) {
        let parent_block = self.inode_block(ino);
        match self.fs.create_node(redoxfs::Node::MODE_FILE, name.to_str().unwrap(), parent_block) {
            Ok(node) => {
                reply.created(&TTL, &FileAttr {
                    ino: self.block_inode(node.0),
                    size: node.1.extents[0].length,
                    blocks: (node.1.extents[0].length + 511)/512,
                    atime: CREATE_TIME,
                    mtime: CREATE_TIME,
                    ctime: CREATE_TIME,
                    crtime: CREATE_TIME,
                    kind: if node.1.is_dir() {
                        FileType::Directory
                    } else {
                        FileType::RegularFile
                    },
                    perm: 0o777,
                    nlink: 1,
                    uid: 0,
                    gid: 0,
                    rdev: 0,
                    flags: 0,
                }, 0, 0, flags);
            },
            Err(error) => {
                reply.error(error.errno as i32);
            }
        }
    }

    fn mkdir(&mut self, _req: &Request, ino: u64, name: &Path, _mode: u32, reply: ReplyEntry) {
        let parent_block = self.inode_block(ino);
        match self.fs.create_node(redoxfs::Node::MODE_DIR, name.to_str().unwrap(), parent_block) {
            Ok(node) => {
                reply.entry(&TTL, &FileAttr {
                    ino: self.block_inode(node.0),
                    size: node.1.extents[0].length,
                    blocks: (node.1.extents[0].length + 511)/512,
                    atime: CREATE_TIME,
                    mtime: CREATE_TIME,
                    ctime: CREATE_TIME,
                    crtime: CREATE_TIME,
                    kind: if node.1.is_dir() {
                        FileType::Directory
                    } else {
                        FileType::RegularFile
                    },
                    perm: 0o777,
                    nlink: 1,
                    uid: 0,
                    gid: 0,
                    rdev: 0,
                    flags: 0,
                }, 0);
            },
            Err(error) => {
                reply.error(error.errno as i32);
            }
        }
    }

    fn rmdir(&mut self, _req: &Request, ino: u64, name: &Path, reply: ReplyEmpty) {
        let parent_block = self.inode_block(ino);
        match self.fs.remove_node(redoxfs::Node::MODE_DIR, name.to_str().unwrap(), parent_block) {
            Ok(()) => {
                reply.ok();
            },
            Err(err) => {
                reply.error(err.errno as i32);
            }
        }
    }

    fn unlink(&mut self, _req: &Request, ino: u64, name: &Path, reply: ReplyEmpty) {
        let parent_block = self.inode_block(ino);
        match self.fs.remove_node(redoxfs::Node::MODE_FILE, name.to_str().unwrap(), parent_block) {
            Ok(()) => {
                reply.ok();
            },
            Err(err) => {
                reply.error(err.errno as i32);
            }
        }
    }

    fn statfs(&mut self, _req: &Request, _ino: u64, reply: ReplyStatfs) {
        let free = self.fs.header.1.free;
        match self.fs.node_len(free) {
            Ok(free_size) => {
                let bsize = 512;
                let blocks = self.fs.header.1.size/bsize;
                let bfree = free_size/bsize;
                reply.statfs(blocks, bfree, bfree, 0, 0, bsize as u32, 256, 0);
            },
            Err(err) => {
                reply.error(err.errno as i32);
            }
        }
    }
}

fn main() {
    if let Some(path) = env::args().nth(1) {
        //Open an existing image
        match Image::open(&path) {
            Ok(disk) => match redoxfs::FileSystem::open(Box::new(disk)) {
                Ok(filesystem) => {
                    println!("redoxfs: opened filesystem {}", path);

                    if let Some(mountpoint) = env::args_os().nth(2) {
                        fuse::mount(RedoxFS {
                            fs: filesystem
                        }, &mountpoint, &[]);
                    } else {
                        println!("redoxfs: no mount point provided");
                    }
                },
                Err(err) => println!("redoxfs: failed to open filesystem {}: {}", path, err)
            },
            Err(err) => println!("redoxfs: failed to open image {}: {}", path, err)
        }
    } else {
        println!("redoxfs: no disk image provided");
    }
}
