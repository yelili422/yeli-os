pub struct File {
    pub type_:    FileType,
    pub readable: bool,
    pub writable: bool,
}

impl File {
    pub fn new() -> File {
        Self {
            type_:    FileType::None,
            readable: false,
            writable: false,
        }
    }

    pub fn read(&self, addr: usize, n: usize) {
        match self.type_ {
            FileType::Inode => {}
            _ => unimplemented!(),
        }
    }
}

pub enum FileType {
    None,
    Pipe,
    Inode,
    Device,
}
