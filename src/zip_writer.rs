use std::{fs::File, io::Write};

use crate::{EpubError, EpubWriter};

impl From<zip::result::ZipError> for EpubError {
    fn from(value: zip::result::ZipError) -> Self {
        match value {
            zip::result::ZipError::Io(io) => EpubError::Io(io),
            zip::result::ZipError::InvalidArchive(v) => EpubError::InvalidArchive(v),
            zip::result::ZipError::UnsupportedArchive(v) => EpubError::UnsupportedArchive(v),
            zip::result::ZipError::InvalidPassword => EpubError::InvalidPassword,
            zip::result::ZipError::FileNotFound => EpubError::FileNotFound,
            _ => EpubError::Unknown,
        }
    }
}

impl From<std::io::Error> for EpubError {
    fn from(value: std::io::Error) -> Self {
        EpubError::Io(value)
    }
}

pub(crate) struct ZipCratesWriter {
    inner: zip::ZipWriter<File>,
}

impl EpubWriter for ZipCratesWriter {
    fn write(&mut self, file: &str, data: &[u8]) -> Result<(), EpubError> {
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored)
            .unix_permissions(0o755);
        self.inner.start_file(file, options)?;
        self.inner.write_all(data)?;
        Ok(())
    }

    fn new(filename: &str) -> crate::EpubResult<Self>
    where
        Self: Sized,
    {
        let path = std::path::Path::new(filename);
        match std::fs::File::create(path) {
            Ok(file) => Ok(ZipCratesWriter {
                inner: zip::ZipWriter::new(file),
            }),
            Err(e) => Err(EpubError::Io(e)),
        }
    }
}
