use crate::app::{
    FileStatus, FileType, FunctionCode, Group70Var2, Group70Var7, PermissionSet, Permissions,
    Sequence, Timestamp,
};
use crate::master::association::AssociationConfig;
use crate::master::tests::harness::{create_association, AssocInfoEvent, TestHarness};
use crate::master::{FileError, FileInfo, TaskError, TaskType};

#[tokio::test]
async fn get_info_request_bytes() {
    let mut harness = create_association(AssociationConfig::quiet()).await;
    let _task = spawn_get_info_task(&harness);
    harness
        .expect_write(vec![
            0xC0, 0x1C, 70, 7, 0x5B, 0x01, 0x19, 0x00, // header, length = 25
            0x14, 0x00, 0x05, 0x00, // file name offset and size
            0x00, 0x00, // file type
            0x00, 0x00, 0x00, 0x00, // file size
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // time of creation
            0x00, 0x00, // permissions
            0xFE, 0xCA, // request id
            b'f', b'.', b't', b'x', b't', // file name
        ])
        .await;
}

#[tokio::test]
async fn returns_file_info() {
    let mut harness = create_association(AssociationConfig::quiet()).await;
    let task = spawn_get_info_task(&harness);
    harness.expect_write(get_info_request(0)).await;
    harness
        .process_response(super::response(
            0,
            &Group70Var7 {
                file_type: FileType::File,
                file_size: 1234,
                time_of_creation: Timestamp::new(5678),
                permissions: Permissions::same(PermissionSet::all()),
                request_id: 0xCAFE,
                file_name: "f.txt",
            },
        ))
        .await;

    let info: FileInfo = task.await.unwrap().unwrap();
    assert_eq!(info.name, "f.txt");
    assert_eq!(info.file_type, FileType::File);
    assert_eq!(info.size, 1234);
    assert_eq!(info.time_created, Timestamp::new(5678));
    assert_eq!(info.permissions, Permissions::same(PermissionSet::all()));

    assert_eq!(
        harness.assoc_events.pop().as_slice(),
        &[
            AssocInfoEvent::TaskStart(
                TaskType::GetFileInfo,
                FunctionCode::GetFileInfo,
                Sequence::new(0)
            ),
            AssocInfoEvent::TaskSuccess(
                TaskType::GetFileInfo,
                FunctionCode::GetFileInfo,
                Sequence::new(0)
            ),
        ]
    );
}

#[tokio::test]
async fn status_object_with_failure_fails_with_bad_status() {
    let result = get_info_with_response(super::file_status_response(
        0,
        0,
        0,
        0,
        FileStatus::FileNotFound,
    ))
    .await;
    assert_eq!(result, Err(FileError::BadStatus(FileStatus::FileNotFound)));
}

#[tokio::test]
async fn status_object_with_success_fails_with_bad_status() {
    let result =
        get_info_with_response(super::file_status_response(0, 0, 0, 0, FileStatus::Success)).await;
    assert_eq!(result, Err(FileError::BadStatus(FileStatus::Success)));
}

#[tokio::test]
async fn fails_on_unexpected_variation() {
    let result = get_info_with_response(super::response(
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

/// Run a GET_FILE_INFO against a response that fails it, and check that the task fails
async fn get_info_with_response(response: Vec<u8>) -> Result<(), FileError> {
    let mut harness = create_association(AssociationConfig::quiet()).await;
    let task = spawn_get_info_task(&harness);
    harness.expect_write(get_info_request(0)).await;
    harness.process_response(response).await;
    let result = task.await.unwrap().map(|_| ());
    assert_eq!(
        harness.assoc_events.pop().as_slice(),
        &[
            AssocInfoEvent::TaskStart(
                TaskType::GetFileInfo,
                FunctionCode::GetFileInfo,
                Sequence::new(0)
            ),
            AssocInfoEvent::TaskFailure(
                TaskType::GetFileInfo,
                TaskError::UnexpectedResponseHeaders
            ),
        ]
    );
    result
}

fn spawn_get_info_task(
    harness: &TestHarness,
) -> tokio::task::JoinHandle<Result<FileInfo, FileError>> {
    let mut association = harness.association.clone();
    tokio::spawn(async move { association.get_file_info("f.txt").await })
}

fn get_info_request(seq: u8) -> Vec<u8> {
    super::request(
        FunctionCode::GetFileInfo,
        seq,
        &Group70Var7 {
            file_type: FileType::Other(0),
            file_size: 0,
            time_of_creation: Timestamp::zero(),
            permissions: Permissions::default(),
            request_id: 0xCAFE,
            file_name: "f.txt",
        },
    )
}
