use std::env;
use std::io;
use std::fs::{self, File};
use std::path::Path;
use std::io::Write;

mod common;

use crate::common::*;

pub mod generator {
    use std::io;
    use std::fs::{self, read, read_dir};
    use std::path::{Path, PathBuf};
    use std::time::*;
    use fuse::*;
    use time::Timespec;
    use std::os::unix::fs::PermissionsExt;
    use crate::common::*;
    use std::borrow::Borrow;

    fn read_file(path: &Path) -> Option<Vec<u8>> {
        return match read(path) {
            Ok(data) =>
                Some(data),
            _ => {
                None
            }
        };
    }

    fn blob_read_file(file_path: &Path, inode: u64) -> Option<FuseFile> {
        let name = file_path.file_name()?.to_str()?.to_owned();
        let data = read_file(&file_path)?;
        let file = FuseFile {
            name,
            node: inode,
            data,
        };

        Some(file)
    }


    fn blob_read_all_files(working_directory: &Vec<PathBuf>, mut inode: u64) -> Option<(Vec<FuseFile>, u64)> {
        let mut files: Vec<FuseFile> = vec!();
        for sub_path in working_directory {
            if sub_path.is_file() {
                let file = blob_read_file(sub_path.as_path(), inode)?;
                files.push(file);

                inode += 1;
            }
        }
        Some((files, inode))
    }

    fn blob_read_all_directories(working_directory: &Vec<PathBuf>, mut inode: u64) -> Option<(Vec<FuseDirectory>, u64)> {
        let mut directories: Vec<FuseDirectory> = vec!();
        for path in working_directory {
            if path.is_dir() {
                let name = path.file_name()?.to_str()?.to_owned();
                directories.push(FuseDirectory {
                    name: name.to_owned(),
                    node: inode,
                    parent_node: 0,
                    nodes: vec!(),
                    node_types: vec!(),
                    is_root: false,
                });
                inode += 1;
            }
        }
        Some((directories, inode))
    }

    fn systemtime_to_timespec(time: SystemTime) -> Timespec {
        let duration = time.duration_since(SystemTime::UNIX_EPOCH).unwrap();
        Timespec::new(duration.as_secs() as i64, duration.subsec_nanos() as i32)
    }

    fn result_to_option<T, E>(result: Result<T, E>) -> Option<T> {
        match result {
            Ok(T) => Some(T),
            _ => None
        }
    }

    pub fn blob_generate_attributes(current_path: &str, directory: &FuseDirectory, mut fuse: &mut FuseStructure) -> Option<()> {
        let mut i = 0;
        for node in &directory.nodes {
            let node_type = directory.node_types[i];

            match node_type {
                1 => {
                    //file
                    let name;
                    let ino;
                    let size;
                    let file = FuseFile::find_by_node(&fuse.files, *node)?;
                    name = current_path.to_owned() + file.name.clone().as_str();
                    size = file.data.len();
                    ino = file.node;

                    let metadata = match fs::metadata(name) {
                        Ok(data) => data,
                        Err(_) => panic!("Error in metadata for file.")
                    };

                    let perms = metadata.permissions().mode();

                    let accessed = result_to_option(metadata.accessed())?;
                    let modified = result_to_option(metadata.modified())?;
                    let created = result_to_option(metadata.created())?;

                    fuse.attributes.push(FileAttr {
                        ino,
                        size: size as u64,
                        blocks: 0,
                        atime: systemtime_to_timespec(accessed),
                        mtime: systemtime_to_timespec(modified),
                        ctime: systemtime_to_timespec(created),
                        crtime: systemtime_to_timespec(created),
                        kind: FileType::RegularFile,
                        perm: perms as u16,
                        nlink: 1,
                        uid: 501,
                        gid: 20,
                        rdev: 0,
                        flags: 0,
                    });
                }
                _ => {
                    //directory
                    let name;
                    let directory: &FuseDirectory = {
                        let directory = FuseDirectory::find_by_node(&fuse.directories, *node)?;
                        name = current_path.to_owned() + directory.name.as_str() + "/";
                        &directory.clone()
                    };


                    blob_generate_attributes(name.as_str(), directory, fuse)?;
                }
            };


            i += 1;
        }

        let metadata = match fs::metadata(current_path) {
            Ok(data) => data,
            Err(_) => panic!("Error in metadata for directory.")
        };

        let perms = metadata.permissions().mode();

        let accessed = result_to_option(metadata.accessed())?;
        let modified = result_to_option(metadata.modified())?;
        let created = result_to_option(metadata.created())?;

        fuse.attributes.push(FileAttr {
            ino: directory.node,
            size: 0,
            blocks: 0,
            atime: systemtime_to_timespec(accessed),
            mtime: systemtime_to_timespec(modified),
            ctime: systemtime_to_timespec(created),
            crtime: systemtime_to_timespec(created),
            kind: FileType::Directory,
            perm: perms as u16,
            nlink: 1,
            uid: 501,
            gid: 20,
            rdev: 0,
            flags: 0,
        });
        Some(())
    }

    pub fn build_blob(path: &Path, mut inode: u64, parent: u64, current_node: u64, mut fuse: &mut FuseStructure, is_root: bool) -> Option<u64> {
        let mut files: Vec<FuseFile> = vec!();
        let mut nodes: Vec<u64> = vec!();
        let mut node_types: Vec<u8> = vec!();

        if path.is_dir() { //should always be true
            let directory = match result_to_option(read_dir(path))?.map(|res| res.map(|e| e.path())).collect::<Result<Vec<_>, io::Error>>() {
                Ok(dir) => {
                    dir
                },
                Err(_) => {
                    panic!("Bad directory.")
                }
            };

            let (temp_files, temp_inode) = blob_read_all_files(&directory, inode)?;
            inode = temp_inode;
            let (temp_directories, temp_inode) = blob_read_all_directories(&directory, inode)?;
            inode = temp_inode;

            let temp_directories2 = &temp_directories;
            for temp_dir in temp_directories2 {
                //create data for sub directory
                let start_path_name = path.to_str()?.to_owned();
                let name = start_path_name + temp_dir.name.as_str() + "/";//.as_str();
                inode = build_blob(&Path::new(name.as_str()), inode, current_node, temp_dir.node, fuse, false)?;
            }

            //build nodes and node_types
            for temp_file in temp_files {
                let node = temp_file.node;
                files.push(temp_file);
                nodes.push(node);
                node_types.push(1);
            }
            for temp_dir in temp_directories {
                let node = temp_dir.node;
                nodes.push(node);
                node_types.push(0);
            }
            //add them to fuse structure
            fuse.files.extend(files);

            //update directories with remaining data
            fuse.directories.push(FuseDirectory {
                name: path.file_name()?.to_str()?.to_owned(),
                nodes,
                node_types,
                node: current_node,
                is_root,
                parent_node: parent,
            });

            //build attributes in another function
        } else {
            println!("bad dir {}", path.to_str()?);
        }
        Some(inode)
    }
}


fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        println!("Usage: generate <directory>");
        return Err(io::Error::from(std::io::ErrorKind::Other));
    }
    let directory = args[1].as_str();

    let first = fs::metadata(directory)?;
    if first.is_dir() == false {
        println!("You have to use this program on a directory.");
        return Err(io::Error::from(std::io::ErrorKind::Other));
    }
    let mut fuse: FuseStructure = FuseStructure::new();
    let result = generator::build_blob(Path::new(directory), 3, 1, 2, &mut fuse, true);
    if result.is_none() {
        println!("Error, aborting!");
        return Err(io::Error::from(std::io::ErrorKind::Other));
    }
    let result = generator::blob_generate_attributes(args[1].as_str(), FuseDirectory::find_root_directory(&fuse.clone().directories).unwrap(), &mut fuse);
    if result.is_none() {
        println!("attribute error, aborting!");
        return Err(io::Error::from(std::io::ErrorKind::Other));
    }

    let mut file = File::create("./out.blob")?;
    file.write_all(fuse.serialize().as_slice())?;

    Ok(())
}
