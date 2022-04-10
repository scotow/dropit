use bitflags::bitflags;

bitflags! {
    pub struct Features: u8 {
        const UPLOAD = 1 << 0;
        const DOWNLOAD = 1 << 1;
    }
}
