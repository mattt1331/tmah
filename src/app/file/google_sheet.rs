//! Saving to and loading from a Google Sheet

pub struct FileSource;
pub struct SaveFileState;
impl SaveFileState {
    pub fn poll_saved(&mut self) -> bool {
        todo!()
    }
}
pub fn save(_file: &FileSource, _data: super::FileData) -> SaveFileState {
    todo!()
}
