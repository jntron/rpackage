use fuse::*;
use std::ffi::OsStr;
use time::Timespec;
use libc::ENOENT;
use byteorder::*;

#[derive(Clone)]
pub struct FuseDirectory {
    pub name: String,
    pub nodes: Vec<u64>,
    pub node_types: Vec<u8>,
    pub node: u64,
    pub is_root: bool,
    pub parent_node: u64
}

#[derive(Clone)]
pub struct FuseFile {
    pub name: String,
    pub data: Vec<u8>,
    pub node: u64,
}

#[derive(Clone)]
pub struct FuseStructure {
    pub epoch: Timespec,
    pub directories: Vec<FuseDirectory>,
    pub files: Vec<FuseFile>,
    pub attributes: Vec<FileAttr>
}

pub trait FuseCommon<T> {
    fn find_by_node(container:&Vec<T>, node:u64) -> Option<&T>;
    fn serialize(&self) -> Vec<u8>;
    fn deserialize(start:usize, data:&Vec<u8>) -> (T, u64); // returns type and read bytes
}

impl FuseCommon<FuseFile> for FuseFile {
    fn find_by_node(container:&Vec<FuseFile>, node:u64) -> Option<&FuseFile> {
        for file in container {
            if file.node == node {
                return Some(file);
            }
        }

        return None;
    }

    fn serialize(&self) -> Vec<u8> {
        let mut returned:Vec<u8> = vec!();

        let name_size = self.name.len();
        let name = &self.name;

        returned.extend(name_size.to_be_bytes().to_vec());
        returned.extend(name.as_bytes().to_vec());

        returned.extend(self.node.to_be_bytes().to_vec());

        let file_size = self.data.len();
        returned.extend(file_size.to_be_bytes().to_vec());

        returned.extend(&self.data);

        returned
    }

    fn deserialize(start:usize, data: &Vec<u8>) -> (FuseFile, u64) {
        let mut bytes_read:usize = start;

        let name_size = BigEndian::read_u64(&FuseStructure::get_sclice_from_vector(data, bytes_read, 8));
        bytes_read += 8;
        let name = String::from_utf8(FuseStructure::get_sclice_from_vector(data, bytes_read, name_size as usize).to_vec()).unwrap();
        bytes_read += name_size as usize;

        let node = BigEndian::read_u64(&FuseStructure::get_sclice_from_vector(data, bytes_read, 8));
        bytes_read += 8;

        let file_size = BigEndian::read_u64(&FuseStructure::get_sclice_from_vector(data, bytes_read, 8));
        bytes_read += 8;
        let file_data = FuseStructure::get_sclice_from_vector(data, bytes_read, file_size as usize);
        bytes_read += file_size as usize;

        (FuseFile {
            name,
            node,
            data: file_data
        }, (bytes_read - start) as u64)
    }

}

impl FuseDirectory {
    /*
    pub fn find_by_parent(container:&Vec<FuseDirectory>, parent:u64) -> Option<&FuseDirectory> {
        for directory in container {
            if directory.parent_node == parent {
                return Some(directory);
            }
        }
        None
    }*/

    pub fn find_root_directory(container:&Vec<FuseDirectory>) -> Option<&FuseDirectory> {
        for directory in container {
            if directory.is_root {
                return Some(directory);
            }
        }
        None
    }
}

impl FuseCommon<FuseDirectory> for FuseDirectory {
    fn find_by_node(container:&Vec<FuseDirectory>, node:u64) -> Option<&FuseDirectory> {
        for directory in container {
            if directory.node == node {
                return Some(directory);
            }
        }
        None
    }

    fn serialize(&self) -> Vec<u8> {
        let mut returned:Vec<u8> = vec!();

        let name_size = self.name.len();
        let name = &self.name;

        returned.extend(name_size.to_be_bytes().to_vec());
        returned.extend(name.as_bytes().to_vec());

        returned.extend(self.node.to_be_bytes().to_vec());

        let nodes_size = self.nodes.len();
        returned.extend(nodes_size.to_be_bytes().to_vec());

        for node in &self.nodes {
            returned.extend(node.to_be_bytes().to_vec());
        }

        for node_type in &self.node_types {
            returned.push(*node_type);
        }

        returned.extend(self.parent_node.to_be_bytes().to_vec());

        if self.is_root {
            returned.push(1);
        } else {
            returned.push(0);
        }

        returned
    }

    fn deserialize(start:usize, data:&Vec<u8>) -> (FuseDirectory, u64) {
        let mut bytes_read:usize = start;

        let name_size = BigEndian::read_u64(&FuseStructure::get_sclice_from_vector(data, bytes_read, 8));
        bytes_read += 8;
        let name = String::from_utf8_lossy(FuseStructure::get_sclice_from_vector(data, bytes_read, name_size as usize).as_slice()).into_owned();
        bytes_read += name_size as usize;

        let node = BigEndian::read_u64(&FuseStructure::get_sclice_from_vector(data, bytes_read, 8));
        bytes_read += 8;

        let nodes_size = BigEndian::read_u64(&FuseStructure::get_sclice_from_vector(data, bytes_read, 8));
        bytes_read += 8;

        let mut nodes:Vec<u64> = vec!();

        for _ in 0..nodes_size {
            let node = BigEndian::read_u64(&FuseStructure::get_sclice_from_vector(data, bytes_read, 8));
            nodes.push(node);
            bytes_read += 8;
        }

        let mut node_types:Vec<u8> = vec!();

        for _ in 0..nodes_size {
            node_types.push(*&FuseStructure::get_sclice_from_vector(data, bytes_read, 1)[0]);
            bytes_read += 1;
        }

        let parent_node = BigEndian::read_u64(&FuseStructure::get_sclice_from_vector(data, bytes_read, 8));
        bytes_read += 8;

        let root = *&FuseStructure::get_sclice_from_vector(data, bytes_read, 1)[0];
        bytes_read += 1;

        let is_root;
        if root == 1 {
            is_root = true;
        } else {
            is_root = false;
        }

        (FuseDirectory {
            name,
            node,
            parent_node,
            nodes,
            node_types,
            is_root
        }, (bytes_read - start) as u64)
    }
}

impl FuseCommon<FileAttr> for FileAttr {
    fn find_by_node(container:&Vec<FileAttr>, node:u64) -> Option<&FileAttr> {
        for attribute in container {
            if attribute.ino == node {
                return Some(attribute);
            }
        }

        return None;
    }

    fn serialize(&self) -> Vec<u8> {
        let mut returned:Vec<u8> = vec!();

        returned.extend(self.ino.to_be_bytes().to_vec());

        returned.extend(self.size.to_be_bytes().to_vec());

        returned.extend(self.atime.sec.to_be_bytes().to_vec());
        returned.extend(self.atime.nsec.to_be_bytes().to_vec());

        returned.extend(self.mtime.sec.to_be_bytes().to_vec());
        returned.extend(self.mtime.nsec.to_be_bytes().to_vec());

        returned.extend(self.ctime.sec.to_be_bytes().to_vec());
        returned.extend(self.ctime.nsec.to_be_bytes().to_vec());

        returned.extend(self.perm.to_be_bytes().to_vec());

        if self.kind == FileType::RegularFile {
            returned.push(1 as u8);
        } else {
            returned.push(0 as u8);
        }

        returned
    }

    fn deserialize(start:usize, data: &Vec<u8>) -> (FileAttr, u64) {
        let mut bytes_read:usize = start;

        let ino = BigEndian::read_u64(&FuseStructure::get_sclice_from_vector(data, bytes_read, 8));
        bytes_read = bytes_read + 8;

        let size = BigEndian::read_u64(&FuseStructure::get_sclice_from_vector(data, bytes_read, 8));
        bytes_read = bytes_read + 8;

        let atime = Timespec::new(BigEndian::read_u64(&FuseStructure::get_sclice_from_vector(data, bytes_read, 8)) as i64, BigEndian::read_u32(&FuseStructure::get_sclice_from_vector(data, bytes_read+8, 4)) as i32);
        bytes_read += 12;
        let mtime = Timespec::new(BigEndian::read_u64(&FuseStructure::get_sclice_from_vector(data, bytes_read, 8)) as i64, BigEndian::read_u32(&FuseStructure::get_sclice_from_vector(data, bytes_read+8, 4)) as i32);
        bytes_read += 12;
        let ctime = Timespec::new(BigEndian::read_u64(&FuseStructure::get_sclice_from_vector(data, bytes_read, 8)) as i64, BigEndian::read_u32(&FuseStructure::get_sclice_from_vector(data, bytes_read+8, 4)) as i32);
        bytes_read += 12;

        let perms = BigEndian::read_u16(&FuseStructure::get_sclice_from_vector(data, bytes_read, 2));
        bytes_read = bytes_read + 2;

        let isfile = FuseStructure::get_sclice_from_vector(data, bytes_read, 1)[0];
        bytes_read = bytes_read + 1;

        let kind;
        if isfile == 1 {
            kind = FileType::RegularFile;
        } else {
            kind = FileType::Directory;
        }

        (FileAttr {
            ino,
            size,
            blocks: 0,
            atime,
            mtime,
            ctime,
            crtime: ctime,
            kind,
            perm: perms,
            nlink: 1,
            uid: 501,
            gid: 20,
            rdev: 0,
            flags: 0
        }, (bytes_read - start) as u64)
    }
}

impl FuseStructure {

    pub fn serialize(&self) -> Vec<u8> {
        let mut returned: Vec<u8> = vec!();

        let number_directories = self.directories.len();
        let number_files = self.files.len();
        let number_attributes = self.attributes.len();

        returned.extend("rpack0".as_bytes().to_vec()); //header for blobs
        returned.extend(number_directories.to_be_bytes().to_vec());
        returned.extend(number_files.to_be_bytes().to_vec());
        returned.extend(number_attributes.to_be_bytes().to_vec());

        for directory in &self.directories {
            returned.extend(directory.serialize());
        }

        for file in &self.files {
            returned.extend(file.serialize());
        }

        for attribute in &self.attributes {
            returned.extend(attribute.serialize());
        }

        returned
    }



    pub fn deserialize(data:&mut Vec<u8>) -> Option<FuseStructure> {
        let mut returned =  FuseStructure {
            epoch: Timespec::new(0,0),
            directories: vec!(),
            files: vec!(),
            attributes: vec!()
        };

        let mut counter:usize = 0;

        let header = FuseStructure::get_sclice_from_vector(data, 0, 6);
        counter += 6;

        if header == "rpack0".as_bytes().to_vec() {
            let number_directories = BigEndian::read_u64(&FuseStructure::get_sclice_from_vector(data, counter, 8));
            counter += 8;
            let number_files = BigEndian::read_u64(&FuseStructure::get_sclice_from_vector(data, counter, 8));
            counter += 8;
            let number_attributes = BigEndian::read_u64(&FuseStructure::get_sclice_from_vector(data, counter, 8));
            counter += 8;

            for _ in 0..number_directories {
                let (dir, count) = FuseDirectory::deserialize(counter, data);
                returned.directories.push(dir);
                counter += count as usize;
            }

            for _ in 0..number_files {
                let (file, count) = FuseFile::deserialize(counter, data);
                returned.files.push(file);
                counter += count as usize;
            }

            for _ in 0..number_attributes {
                let (attr, count) = FileAttr::deserialize(counter, data);
                returned.attributes.push(attr);
                counter += count as usize;
            }

        } else {
            return None;
        }

        return Some(returned);
    }

    pub fn new() -> FuseStructure {
        let timespec = Timespec::new(0, 0);
        return FuseStructure {
            epoch: timespec,
            directories: vec!(),
            files: vec!(),
            attributes: vec!(FileAttr {
                ino: 1,
                size: 0,
                blocks: 0,
                atime: timespec,                                  // 1970-01-01 00:00:00
                mtime: timespec,
                ctime: timespec,
                crtime: timespec,
                kind: FileType::Directory,
                perm: 0o755,
                nlink: 2,
                uid: 501,
                gid: 20,
                rdev: 0,
                flags: 0,
            }
            ),
        };
    }

    pub fn get_sclice_from_vector(data: &Vec<u8>, start:usize, amount:usize) -> Vec<u8> {
        data.as_slice()[start..start + amount].to_vec()
    }

}

impl Filesystem for FuseStructure {
    fn lookup(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry) {
        let dir_node;
        if parent == 1 {
            dir_node = *&FuseDirectory::find_root_directory(&self.directories).unwrap().node;
        } else {
            dir_node = *&FuseDirectory::find_by_node(&self.directories, parent).unwrap().node;
        }

        let directory= &FuseDirectory::find_by_node(&self.directories, dir_node).unwrap();

        let mut i=0;
        for node in &directory.nodes {
            let node_type = directory.node_types[i];

            if node_type == 0 { //directory
                let directory = FuseDirectory::find_by_node(&self.directories, *node).unwrap();
                if directory.name == name.to_str().unwrap() {
                    let attribute = FileAttr::find_by_node(&self.attributes, *node).unwrap();
                    reply.entry(&self.epoch, &attribute, 0);
                    return;
                }
            } else { //file
                let file = FuseFile::find_by_node(&self.files, *node).unwrap();
                if file.name == name.to_str().unwrap() {
                    let attribute = FileAttr::find_by_node(&self.attributes, *node).unwrap();
                    reply.entry(&self.epoch, &attribute, 0);
                    return;
                }
            }

            i += 1;
        }
        reply.error(ENOENT);
    }

    fn getattr(&mut self, _req: &Request, ino: u64, reply: ReplyAttr) {
        let attribute = FileAttr::find_by_node(&self.attributes, ino);
        if attribute.is_some() {
            reply.attr(&self.epoch, &attribute.unwrap());
        }
        else {
            reply.error(ENOENT);
        }
    }

    fn read(&mut self, _req: &Request, ino: u64, _fh: u64, offset: i64, _size: u32, reply: ReplyData) {
        let file = FuseFile::find_by_node(&self.files, ino);
        if file.is_some() {
            reply.data(&file.unwrap().data[offset as usize..]);
        } else {
            reply.error(ENOENT);
        }
    }

    fn readdir(&mut self, _req: &Request, ino: u64, _fh: u64, offset: i64, mut reply: ReplyDirectory) {
        let mut real_ino = ino;

        if ino == 1 {
            real_ino = *&FuseDirectory::find_root_directory(&self.directories).unwrap().node;//self.directories[0].node; //root directory
        }

        let directory = FuseDirectory::find_by_node(&self.directories, real_ino);
        if directory.is_none() {
            reply.error(ENOENT);
            return;
        }
        let directory = directory.unwrap();

        if directory.is_root {
            if offset == 0 {
                reply.add(real_ino, 1 as i64, FileType::Directory, ".");
            }
            if offset < 2 {
                reply.add(1, 2 as i64, FileType::Directory, "..");
            }
        } else {
            if offset == 0 {
                reply.add(real_ino, 1 as i64, FileType::Directory, ".");
            }
            if offset < 2 {
                reply.add(directory.parent_node, 2 as i64, FileType::Directory, "..");
            }
        }

        for i in 0..directory.nodes.len() as usize {
            if i < offset as usize { //shouldn't this be offset - 2 because of . and ..?
                continue;
            }

            let node = directory.nodes[i];
            let node_type = directory.node_types[i];
            if node_type == 1 {
                let file = FuseFile::find_by_node(&self.files, node).unwrap();
                reply.add(node, (i + offset as usize + 1) as i64, FileType::RegularFile, &file.name);
            } else {
                let directory = FuseDirectory::find_by_node(&self.directories, node).unwrap();
                reply.add(node, (i + offset as usize + 1) as i64, FileType::Directory, &directory.name);
            }
        }
        reply.ok();
    }
}
