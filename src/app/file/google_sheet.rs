//! Saving to and loading from a Google Sheet

pub struct FileSource;
pub fn export(data: super::FileData, receiver: impl FnOnce(String) -> ()) {
    let data = TimeTaggedFileData::now_with_data(data);
    match data.serialize() {
        Ok(data) => receiver(data),
        Err(err) => log::error!("Failed to serialize data: {err}"),
    }
}

/// When exporting in plain text (not to a file), we add an extra tag with the export time so that
/// we know which exported version is which.
#[derive(serde::Serialize, serde::Deserialize)]
struct TimeTaggedFileData(chrono::DateTime<chrono::Utc>, super::FileData);
impl TimeTaggedFileData {
    /// Make one of these out of a `FileData` using the current time as the time tag
    fn now_with_data(data: super::FileData) -> TimeTaggedFileData {
        TimeTaggedFileData(chrono::Utc::now(), data)
    }
    /// Serialize into a RON string
    fn serialize(&self) -> Result<String, ron::Error> {
        ron::to_string(self)
    }
}
