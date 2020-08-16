use std::env;
use std::ffi::OsStr;
use std::fs::{create_dir, remove_dir};
use std::path::Path;
use std::process::Command;
use std::{thread, time};

mod common;
use crate::common::*;

fn main() -> std::io::Result<()>{
    let mut data = include_bytes!("out.blob").to_vec();

    //TODO: handle errors deserializing
    let fuse_structure = FuseStructure::deserialize(&mut data).unwrap();

    let options = ["-o", "ro", "-o", "fsname=rpackage"]
        .iter()
        .map(|o| o.as_ref())
        .collect::<Vec<&OsStr>>();

    create_dir(Path::new("./fusemount"))?;
    let mountpoint = &"./fusemount";

    let h = thread::spawn(move || {
        let sec = time::Duration::from_millis(1000);
        thread::sleep(sec); // wait for the structure to mount
        env::set_current_dir(Path::new("./fusemount"));
        let output = Command::new("bash")
            .arg("./startup.sh")
            .output()
            .expect("command failed to start");

        println!("Hello, world! {}\nStderr: {}", String::from_utf8(output.stdout).unwrap(), String::from_utf8(output.stderr).unwrap());
        env::set_current_dir(Path::new(".."));
        Command::new("fusermount")
            .arg("-u")
            .arg("./fusemount")
            .output()
            .expect("command failed to start");
    });
    fuse::mount(fuse_structure, mountpoint, options.as_slice()).unwrap();
    h.join();
    remove_dir("./fusemount");

    Ok(())
}
