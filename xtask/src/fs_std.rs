use fs::{cache::BlockDevice, config::BLOCK_SIZE, FileSystem};
use std::{
    fs::{read_dir, File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::PathBuf,
    sync::{Arc, Mutex},
};

use crate::{project_path, BuildArgs};

struct BlockFile(Mutex<File>);

impl BlockDevice for BlockFile {
    fn read_block(&self, block_id: usize, buf: &mut [u8]) {
        let mut file = self.0.lock().unwrap();
        file.seek(SeekFrom::Start((block_id * BLOCK_SIZE) as u64))
            .expect("Err when seeking");
        assert_eq!(
            file.read(buf).unwrap(),
            BLOCK_SIZE,
            "Not a complete block for"
        );
    }

    fn write_block(&self, block_id: usize, buf: &[u8]) {
        let mut file = self.0.lock().unwrap();
        file.seek(SeekFrom::Start((block_id * BLOCK_SIZE) as u64))
            .expect("Err when seeking");
        assert_eq!(file.write(buf).unwrap(), BLOCK_SIZE, "Not a complete block");
    }
}

impl BuildArgs {
    pub fn fs_pack(&self, dst: &PathBuf) -> std::io::Result<()> {
        let src = project_path().join("user").join("src").join("bin");
        let src_bin = self.base_path();
        const BLOCK_NUM: u64 = 17 * 2048;
        let target = Arc::new(BlockFile(Mutex::new({
            let file = OpenOptions::new()
                .create(true)
                .read(true)
                .write(true)
                .truncate(true)
                .open(dst)?;
            file.set_len(BLOCK_NUM * 512)?;
            file
        })));
        let fs = FileSystem::create(target, BLOCK_NUM as u32, 1);
        let root_inode = Arc::new(FileSystem::get_root_inode(&fs));
        if let Ok(files) = read_dir(src) {
            for file in files {
                let path = file?.path();
                let name = path.file_stem().unwrap();
                println!("writing {:?} to img", name);
                let mut bin = File::open(src_bin.join(name))?;
                let mut data = Vec::new();
                bin.read_to_end(&mut data)?;
                if let Some(inode) = root_inode.create(name.to_str().unwrap()) {
                    inode.write_at(0, &data.as_slice());
                }
            }
        }
        for app in root_inode.ls() {
            println!("find {} in img", app);
        }
        Ok(())
    }
}
