use crate::app::{Group70Var7, MaybeAsync};
use crate::master::promise::Promise;
use crate::master::{FileAction, FileError, FileInfo, FileReader};
use scursor::ReadCursor;

pub(crate) struct DirectoryReader {
    data: Vec<u8>,
    promise: Option<Promise<Result<Vec<FileInfo>, FileError>>>,
}

impl DirectoryReader {
    pub(crate) fn new(promise: Promise<Result<Vec<FileInfo>, FileError>>) -> Self {
        Self {
            data: Vec::new(),
            promise: Some(promise),
        }
    }
}

impl FileReader for DirectoryReader {
    fn opened(&mut self, _size: u32) -> FileAction {
        FileAction::Continue
    }

    fn block_received(&mut self, _block_num: u32, data: &[u8]) -> MaybeAsync<FileAction> {
        self.data.extend(data);
        MaybeAsync::ready(FileAction::Continue)
    }

    fn aborted(&mut self, err: FileError) {
        if let Some(x) = self.promise.take() {
            x.complete(Err(err));
        }
    }

    fn completed(&mut self) {
        /// Read every entry in `data`, passing each one to `each`
        fn read_entries<'a>(
            data: &'a [u8],
            mut each: impl FnMut(Group70Var7<'a>),
        ) -> Result<(), FileError> {
            let mut cursor = ReadCursor::new(data);
            while !cursor.is_empty() {
                match Group70Var7::read(&mut cursor) {
                    Ok(x) => each(x),
                    Err(err) => {
                        tracing::warn!("Error reading directory information: {err}");
                        return Err(FileError::BadResponse);
                    }
                }
            }
            Ok(())
        }

        fn parse(data: &[u8]) -> Result<Vec<FileInfo>, FileError> {
            // every entry is read before any is converted, so a rejected directory allocates
            // nothing
            let mut count: usize = 0;
            read_entries(data, |_| count = count.saturating_add(1))?;
            let mut items = Vec::with_capacity(count);
            read_entries(data, |x| items.push(x.into()))?;
            Ok(items)
        }

        // parse the accumulated data

        let res = parse(self.data.as_slice());
        if let Some(promise) = self.promise.take() {
            promise.complete(res);
        }
    }
}

impl<'a> From<Group70Var7<'a>> for FileInfo {
    fn from(value: Group70Var7<'a>) -> Self {
        Self {
            name: value.file_name.to_string(),
            file_type: value.file_type,
            size: value.file_size,
            time_created: value.time_of_creation,
            permissions: value.permissions,
        }
    }
}
