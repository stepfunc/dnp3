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

#[cfg(test)]
mod tests {
    use scursor::WriteCursor;

    use super::*;
    use crate::app::format::free_format::FreeFormat;
    use crate::app::{FileType, PermissionSet, Permissions, Timestamp};

    const ENTRIES: [Group70Var7<'static>; 2] = [
        Group70Var7 {
            file_type: FileType::Directory,
            file_size: 3,
            time_of_creation: Timestamp::new(1),
            permissions: Permissions {
                world: PermissionSet::all(),
                group: PermissionSet::all(),
                owner: PermissionSet::all(),
            },
            request_id: 7,
            file_name: "sub",
        },
        Group70Var7 {
            file_type: FileType::File,
            file_size: 1234,
            time_of_creation: Timestamp::new(0x0102_0304_0506),
            permissions: Permissions {
                world: PermissionSet::all(),
                group: PermissionSet::all(),
                owner: PermissionSet::all(),
            },
            request_id: 0,
            file_name: "f.txt",
        },
    ];

    fn fields(info: &FileInfo) -> (&str, FileType, u32, Timestamp, Permissions) {
        (
            info.name.as_str(),
            info.file_type,
            info.size,
            info.time_created,
            info.permissions,
        )
    }

    #[tokio::test]
    async fn completes_with_every_entry_of_every_block() {
        let mut buffer = [0; 128];
        let mut cursor = WriteCursor::new(&mut buffer);
        for entry in &ENTRIES {
            entry.write(&mut cursor).unwrap();
        }
        // the second block starts inside the second entry
        let (first, second) = cursor.written().split_at(30);

        let (promise, mut rx) = Promise::one_shot();
        let mut reader = DirectoryReader::new(promise);
        assert_eq!(reader.opened(0), FileAction::Continue);
        for (block, data) in [first, second].into_iter().enumerate() {
            let block = u32::try_from(block).unwrap();
            let action = reader.block_received(block, data).get().await;
            assert_eq!(action, FileAction::Continue);
        }
        reader.completed();

        let entries = rx.try_recv().unwrap().unwrap();
        assert_eq!(
            entries.iter().map(fields).collect::<Vec<_>>(),
            ENTRIES
                .iter()
                .map(|x| (
                    x.file_name,
                    x.file_type,
                    x.file_size,
                    x.time_of_creation,
                    x.permissions
                ))
                .collect::<Vec<_>>()
        );
    }
}
