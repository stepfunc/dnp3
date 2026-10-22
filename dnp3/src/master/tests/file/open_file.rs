use crate::app::{
    FileStatus, FunctionCode, Group70Var2, Group70Var3, PermissionSet, Permissions, Sequence,
    Timestamp,
};
use crate::master::association::AssociationConfig;
use crate::master::tasks::file::REQUEST_ID;
use crate::master::tests::harness::{create_association, AssocInfoEvent, TestHarness};
use crate::master::{
    AssociationHandle, AuthKey, FileError, FileHandle, FileMode, OpenFile, TaskError, TaskType,
};

#[tokio::test]
async fn open_request_bytes() {
    let config = AssociationConfig::quiet();
    let mut harness = create_association(config).await;

    let mut association = harness.association.clone();
    let open_task = tokio::spawn(async move {
        association
            .open_file(
                "f.txt",
                AuthKey::new(0x01020304),
                Permissions::same(PermissionSet::all()),
                0x0A0B0C0D,
                FileMode::Append,
                0x0E0F,
            )
            .await
    });

    harness
        .expect_write(vec![
            0xC0, 0x19, 70, 3, 0x5B, 0x01, 0x1F, 0x00, // header, length = 31
            0x1A, 0x00, 0x05, 0x00, // file name offset and size
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // time of creation
            0xFF, 0x01, // permissions
            0x04, 0x03, 0x02, 0x01, // auth key
            0x0D, 0x0C, 0x0B, 0x0A, // file size
            0x03, 0x00, // mode (append)
            0x0F, 0x0E, // max block size
            b'S', b'F', // request id
            b'f', b'.', b't', b'x', b't', // file name
        ])
        .await;

    harness
        .process_response(super::file_status_response(
            0,
            0xDEADCAFE,
            7,
            128,
            FileStatus::Success,
        ))
        .await;

    assert_eq!(
        open_task.await.unwrap(),
        Ok(OpenFile {
            file_handle: FileHandle::new(0xDEADCAFE),
            max_block_size: 128,
            file_size: 7,
        })
    );

    assert_eq!(
        harness.assoc_events.pop().as_slice(),
        &[
            AssocInfoEvent::TaskStart(TaskType::FileOpen, FunctionCode::OpenFile, Sequence::new(0)),
            AssocInfoEvent::TaskSuccess(
                TaskType::FileOpen,
                FunctionCode::OpenFile,
                Sequence::new(0)
            ),
        ]
    );
}

#[tokio::test]
async fn bad_status_fails_the_task() {
    for status in [FileStatus::PermissionDenied, FileStatus::FileLocked] {
        let result = open_with_response(super::file_status_response(0, 21, 0, 512, status)).await;
        assert_eq!(result, Err(FileError::BadStatus(status)));
    }
}

#[tokio::test]
async fn fails_on_unexpected_variation() {
    let result = open_with_response(super::response(
        0,
        &Group70Var2 {
            auth_key: 1,
            user_name: "",
            password: "",
        },
    ))
    .await;
    assert_eq!(result, Err(FileError::BadResponse));
}

/// Run the default OPEN against a response that fails it, and check that the task fails
async fn open_with_response(response: Vec<u8>) -> Result<OpenFile, FileError> {
    let mut harness = create_association(AssociationConfig::quiet()).await;
    let open_task = spawn_open_task(harness.association.clone());
    harness
        .expect_write(open_file(
            0,
            "test.txt",
            Permissions::default(),
            AuthKey::none(),
            42,
            FileMode::Write,
            512,
        ))
        .await;
    harness.process_response(response).await;
    let result = open_task.await.unwrap();
    expect_failure(&mut harness);
    result
}

fn expect_failure(harness: &mut TestHarness) {
    assert_eq!(
        harness.assoc_events.pop().as_slice(),
        &[
            AssocInfoEvent::TaskStart(TaskType::FileOpen, FunctionCode::OpenFile, Sequence::new(0)),
            AssocInfoEvent::TaskFailure(TaskType::FileOpen, TaskError::UnexpectedResponseHeaders),
        ]
    );
}

fn spawn_open_task(
    mut association: AssociationHandle,
) -> tokio::task::JoinHandle<Result<OpenFile, FileError>> {
    tokio::spawn({
        async move {
            association
                .open_file(
                    "test.txt",
                    AuthKey::none(),
                    Permissions::default(),
                    42,
                    FileMode::Write,
                    512,
                )
                .await
        }
    })
}

fn open_file(
    seq: u8,
    file_name: &str,
    permissions: Permissions,
    auth_key: AuthKey,
    file_size: u32,
    mode: FileMode,
    max_block_size: u16,
) -> Vec<u8> {
    super::request(
        FunctionCode::OpenFile,
        seq,
        &Group70Var3 {
            time_of_creation: Timestamp::zero(),
            permissions,
            auth_key: auth_key.into(),
            file_size,
            mode,
            max_block_size,
            request_id: REQUEST_ID,
            file_name,
        },
    )
}
