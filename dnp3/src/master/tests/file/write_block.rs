use crate::app::{FileStatus, FunctionCode, Group70Var4, Group70Var6, Sequence};
use crate::master::association::AssociationConfig;
use crate::master::tests::harness::{create_association, AssocInfoEvent, TestHarness};
use crate::master::{BlockNumber, FileError, FileHandle, TaskError, TaskType};

const HANDLE: u32 = 0xDEADCAFE;

#[tokio::test]
async fn write_block_request_bytes_and_success() {
    let mut harness = create_association(AssociationConfig::quiet()).await;
    let task = spawn_write_task(&harness);
    harness
        .expect_write(vec![
            0xC0, 0x02, 70, 5, 0x5B, 0x01, 0x0B, 0x00, // header, length = 11
            0xFE, 0xCA, 0xAD, 0xDE, // file handle
            0x02, 0x00, 0x00, 0x80, // block number 2, last
            0x01, 0x02, 0x03, // data
        ])
        .await;
    harness
        .process_response(block_status_response(2, FileStatus::Success))
        .await;

    assert_eq!(task.await.unwrap(), Ok(()));
    assert_eq!(
        harness.assoc_events.pop().as_slice(),
        &[
            AssocInfoEvent::TaskStart(
                TaskType::FileWriteBlock,
                FunctionCode::Write,
                Sequence::new(0)
            ),
            AssocInfoEvent::TaskSuccess(
                TaskType::FileWriteBlock,
                FunctionCode::Write,
                Sequence::new(0)
            ),
        ]
    );
}

#[tokio::test]
async fn fails_on_bad_status() {
    let result = write_with_response(block_status_response(2, FileStatus::BlockSeq)).await;
    assert_eq!(result, Err(FileError::BadStatus(FileStatus::BlockSeq)));
}

#[tokio::test]
async fn fails_on_unexpected_variation() {
    let result = write_with_response(super::response(
        0,
        &Group70Var4 {
            file_handle: HANDLE,
            file_size: 0,
            max_block_size: 0,
            request_id: 0,
            status_code: FileStatus::Success,
            text: "",
        },
    ))
    .await;
    assert_eq!(result, Err(FileError::BadResponse));
}

/// Run the block write against a response that fails it, and check that the task fails
async fn write_with_response(response: Vec<u8>) -> Result<(), FileError> {
    let mut harness = create_association(AssociationConfig::quiet()).await;
    let task = spawn_write_task(&harness);
    harness.pop_write().await;
    harness.process_response(response).await;
    let result = task.await.unwrap();
    assert_eq!(
        harness.assoc_events.pop().as_slice(),
        &[
            AssocInfoEvent::TaskStart(
                TaskType::FileWriteBlock,
                FunctionCode::Write,
                Sequence::new(0)
            ),
            AssocInfoEvent::TaskFailure(
                TaskType::FileWriteBlock,
                TaskError::UnexpectedResponseHeaders
            ),
        ]
    );
    result
}

fn spawn_write_task(harness: &TestHarness) -> tokio::task::JoinHandle<Result<(), FileError>> {
    let mut association = harness.association.clone();
    tokio::spawn(async move {
        let mut block = BlockNumber::default();
        block.increment().unwrap();
        block.increment().unwrap();
        block.set_last();
        association
            .write_file_block(FileHandle::new(HANDLE), block, vec![1, 2, 3])
            .await
    })
}

fn block_status_response(block_number: u32, status_code: FileStatus) -> Vec<u8> {
    super::response(
        0,
        &Group70Var6 {
            file_handle: HANDLE,
            block_number: super::last_block(block_number),
            status_code,
            text: "",
        },
    )
}
