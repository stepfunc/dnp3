use crate::app::{FileStatus, FunctionCode, Group70Var4, Group70Var5, Sequence};
use crate::master::association::AssociationConfig;
use crate::master::tasks::file::REQUEST_ID;
use crate::master::tests::harness::{create_association, AssocInfoEvent, TestHarness};
use crate::master::{AssociationHandle, FileError, FileHandle, TaskError, TaskType};

const HANDLE: FileHandle = FileHandle::new(42);

#[tokio::test]
async fn close_request_bytes() {
    let mut harness = create_association(AssociationConfig::quiet()).await;
    let _close_task = spawn_close_task(harness.association.clone(), FileHandle::new(0xDEADCAFE));
    harness
        .expect_write(close_request_bytes_for(0xC0, 0xDEADCAFE))
        .await;
}

#[tokio::test]
async fn fails_on_wrong_handle() {
    let result = close_with_response(super::file_status_response(
        0,
        43,
        0,
        0,
        FileStatus::Success,
    ))
    .await;
    assert_eq!(result, Err(FileError::WrongHandle));
}

#[tokio::test]
async fn fails_on_bad_status() {
    let result = close_with_response(super::file_status_response(
        0,
        HANDLE.into(),
        0,
        0,
        FileStatus::InvalidHandle,
    ))
    .await;
    assert_eq!(result, Err(FileError::BadStatus(FileStatus::InvalidHandle)));
}

#[tokio::test]
async fn wrong_handle_is_reported_before_bad_status() {
    let result = close_with_response(super::file_status_response(
        0,
        43,
        0,
        0,
        FileStatus::InvalidHandle,
    ))
    .await;
    assert_eq!(result, Err(FileError::WrongHandle));
}

#[tokio::test]
async fn ignores_size_and_block_size_in_response() {
    let mut harness = create_association(AssociationConfig::quiet()).await;
    let close_task = spawn_close_task(harness.association.clone(), HANDLE);
    harness.expect_write(close_file(0, HANDLE)).await;
    harness
        .process_response(super::file_status_response(
            0,
            HANDLE.into(),
            1234,
            512,
            FileStatus::Success,
        ))
        .await;
    assert_eq!(close_task.await.unwrap(), Ok(()));
}

#[tokio::test]
async fn fails_on_unexpected_variation() {
    let result = close_with_response(super::response(
        0,
        &Group70Var5 {
            file_handle: HANDLE.into(),
            block_number: 0,
            file_data: &[],
        },
    ))
    .await;
    assert_eq!(result, Err(FileError::BadResponse));
}

/// Run a CLOSE of [`HANDLE`] against a response that fails it, and check that the task fails
async fn close_with_response(response: Vec<u8>) -> Result<(), FileError> {
    let mut harness = create_association(AssociationConfig::quiet()).await;
    let close_task = spawn_close_task(harness.association.clone(), HANDLE);
    harness.expect_write(close_file(0, HANDLE)).await;
    harness.process_response(response).await;
    let result = close_task.await.unwrap();
    expect_failure(&mut harness);
    result
}

fn expect_failure(harness: &mut TestHarness) {
    assert_eq!(
        harness.assoc_events.pop().as_slice(),
        &[
            AssocInfoEvent::TaskStart(
                TaskType::FileClose,
                FunctionCode::CloseFile,
                Sequence::new(0)
            ),
            AssocInfoEvent::TaskFailure(TaskType::FileClose, TaskError::UnexpectedResponseHeaders),
        ]
    );
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

/// Expected bytes of a CLOSE_FILE request, written out literally
pub(super) fn close_request_bytes_for(control: u8, handle: u32) -> Vec<u8> {
    let mut bytes = vec![
        control, 0x1A, 70, 4, 0x5B, 0x01, 0x0D, 0x00, // header, length = 13
    ];
    bytes.extend_from_slice(&handle.to_le_bytes());
    bytes.extend_from_slice(&[
        0x00, 0x00, 0x00, 0x00, // file size
        0x00, 0x00, // max block size
        b'S', b'F', // request id
        0x00, // status code (success)
    ]);
    bytes
}
