use crate::app::{FileStatus, FunctionCode, Group70Var3, Permissions, Timestamp};
use crate::master::association::AssociationConfig;
use crate::master::tasks::file::REQUEST_ID;
use crate::master::tests::harness::create_association;
use crate::master::{AssociationHandle, AuthKey, FileError, FileHandle, FileMode, OpenFile};

#[tokio::test]
async fn can_open_file() {
    let config = AssociationConfig::quiet();
    let mut harness = create_association(config).await;

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

    harness
        .process_response(super::file_status_response(
            0,
            21,
            55,
            512,
            FileStatus::Success,
        ))
        .await;

    assert_eq!(
        open_task.await.unwrap(),
        Ok(OpenFile {
            file_handle: FileHandle::new(21),
            max_block_size: 512,
            file_size: 55,
        })
    )
}

#[tokio::test]
async fn fails_if_status_not_success() {
    let config = AssociationConfig::quiet();
    let mut harness = create_association(config).await;

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

    harness
        .process_response(super::file_status_response(
            0,
            21,
            0,
            512,
            FileStatus::FileLocked,
        ))
        .await;

    assert_eq!(
        open_task.await.unwrap(),
        Err(FileError::BadStatus(FileStatus::FileLocked))
    )
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
