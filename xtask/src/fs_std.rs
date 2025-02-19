use fs::{cache::BlockDevice, config::BLOCK_SIZE};
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom, Write},
    sync::Mutex,
};

struct BlockFile(Mutex<File>);

impl BlockDevice for BlockFile {
    fn read_block(&self, block_id: usize, buf: &mut [u8]) {
        let mut file = self.0.lock().unwrap();
        file.seek(SeekFrom::Start((block_id * BLOCK_SIZE) as u64))
            .expect("Err when seeking");
        assert_eq!(file.read(buf).unwrap(), BLOCK_SIZE, "Not a complete block");
    }

    fn write_block(&self, block_id: usize, buf: &[u8]) {
        let mut file = self.0.lock().unwrap();
        file.seek(SeekFrom::Start((block_id * BLOCK_SIZE) as u64))
            .expect("Err when seeking");
        assert_eq!(file.write(buf).unwrap(), BLOCK_SIZE, "Not a complete block");
    }
}

pub fn fs_pack() -> std::io::Result<()> {
    Ok(())
}
