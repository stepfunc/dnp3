use crate::app::{FileStatus, FunctionCode, Group70Var4};
use crate::master::association::AssociationConfig;
use crate::master::tasks::file::REQUEST_ID;
use crate::master::tests::harness::create_association;
use crate::master::{AssociationHandle, FileError, FileHandle};

#[tokio::test]
async fn can_close_file() {
    let config = AssociationConfig::quiet();
    let mut harness = create_association(config).await;

    const HANDLE: FileHandle = FileHandle::new(42);

    let open_task = spawn_close_task(harness.association.clone(), HANDLE);

    harness.expect_write(close_file(0, HANDLE)).await;

    harness
        .process_response(super::file_status_response(
            0,
            HANDLE.into(),
            0,
            0,
            FileStatus::Success,
        ))
        .await;

    assert_eq!(open_task.await.unwrap(), Ok(()))
}

fn spawn_close_task(
    mut association: AssociationHandle,
    handle: FileHandle,
) -> tokio::task::JoinHandle<Result<(), FileError>> {
    tokio::spawn(async move { association.close_file(handle).await })
}

fn close_file(seq: u8, handle: FileHandle) -> Vec<u8> {
    super::request(
        FunctionCode::CloseFile,
        seq,
        &Group70Var4 {
            file_handle: handle.into(),
            file_size: 0,
            max_block_size: 0,
            request_id: REQUEST_ID,
            status_code: FileStatus::Success,
            text: "",
        },
    )
}
